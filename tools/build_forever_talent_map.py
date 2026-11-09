"""Reproduce the bundled talent map from the pinned local simulator ZIP, offline."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

REVISION = '9dc32ac8ca0d8d1b76337861d38b93c0f884af11'
SOURCE_SHA = 'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712'
CLASSES = ('druid', 'hunter', 'mage', 'paladin', 'priest', 'rogue', 'shaman', 'warlock', 'warrior')


def build(source):
    with source.open('rb') as handle:
        if hashlib.file_digest(handle, 'sha256').hexdigest() != SOURCE_SHA:
            raise ValueError('Pinned source ZIP checksum mismatch')
    result = {'revision': REVISION, 'source_sha256': SOURCE_SHA, 'classes': {}}
    with zipfile.ZipFile(source) as archive:
        def read(path):
            info = archive.getinfo(f'forever-{REVISION}/{path}')
            if info.file_size > 1024 * 1024:
                raise ValueError('Oversized source member')
            return archive.read(info).decode('utf-8')
        for name in CLASSES:
            trees = json.loads(read(f'ui/sim/talents/trees/{name}.json'))
            sizes = re.search(r'var TalentTreeSizes = \[3\]int\{(\d+), (\d+), (\d+)\}',
                              read(f'sim/{name}/{name}.go'))
            if sizes is None or len(trees) != 3:
                raise ValueError('Unsupported source talent trees')
            sizes = list(map(int, sizes.groups()))
            message = read(f'proto/{name}.proto').split(f'message {name.capitalize()}Talents {{', 1)[1].split('}', 1)[0]
            fields = re.findall(r'(int32|bool) (\w+) = (\d+) \[\(talent_spell_id\) = (\d+)(?:, \(talent_max_rank\) = (\d+))?\];', message)
            talents = [talent for tree in trees for talent in tree['talents']]
            actual = [len(tree['talents']) for tree in trees]
            # FillTalentsProto uses only the first two sizes as offsets for the
            # supported three strings. Upstream Priest overstates its final size.
            if sizes[:2] != actual[:2] or sizes[2] < actual[2] or len(fields) != len(talents):
                raise ValueError('Source talent layout mismatch')
            seen = set(); output = []
            for index, (talent, field) in enumerate(zip(talents, fields), 1):
                kind, snake, number, spell, maximum = field
                limit = 1 if kind == 'bool' else int(maximum)
                definition = talent['definitionId']
                if (int(number) != index or snake.replace('_', '') != talent['fieldName'].lower() or int(spell) != talent['spellId']
                        or limit != talent['maxPoints'] or not 1 <= limit <= 9
                        or type(definition) is not int or not 0 < definition <= 2**32-1 or definition in seen):
                    raise ValueError('Source talent identity mismatch')
                seen.add(definition)
                output.append({'definition_id': definition, 'max_rank': limit})
            result['classes'][name] = {'tree_sizes': actual, 'engine_tree_sizes': sizes, 'talents': output}
    return (json.dumps(result, sort_keys=True, separators=(',', ':')) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-zip', type=Path, required=True)
    parser.add_argument('--check', type=Path, required=True, help='Compare a bundled map without writing')
    args = parser.parse_args()
    data = build(args.source_zip)
    if args.check.read_bytes() != data:
        parser.exit(1, 'Bundled talent map differs from pinned source.\n')
    print(json.dumps({'status': 'matches', 'sha256': hashlib.sha256(data).hexdigest(), 'classes': len(CLASSES)}))


if __name__ == '__main__':
    main()
