"""Build full talent constraints from the authenticated public source archive."""
import argparse
import hashlib
import json
import zipfile
from tools import build_forever_talent_map as identity


def build(source):
    mapping = json.loads(identity.build(source))
    result = dict(revision=identity.REVISION, source_sha256=identity.SOURCE_SHA, source_files={}, classes={})
    with zipfile.ZipFile(source) as archive:
        def read(path):
            entry = archive.getinfo(f'forever-{identity.REVISION}/{path}')
            if entry.file_size > 1024 * 1024:
                raise ValueError('Oversized rule source')
            raw = archive.read(entry)
            result['source_files'][path] = hashlib.sha256(raw).hexdigest()
            return raw
        read('ui/sim/talents/broken_talents.ts')
        read('ui/features/talents/model/can_set_points.ts')
        for name in identity.CLASSES:
            source_trees = json.loads(read(f'ui/sim/talents/trees/{name}.json'))
            trees, definitions = [], []
            for tree in source_trees:
                nodes, locations = [], {}
                for talent in tree['talents']:
                    location = talent['location']
                    row, col = location['rowIdx'], location['colIdx']
                    if (type(row) is not int or type(col) is not int or not 0 <= row <= 20 or not 0 <= col <= 20
                            or (row, col) in locations or nodes and row < nodes[-1]['row']):
                        raise ValueError('Invalid talent location')
                    locations[row, col] = len(nodes)
                    nodes.append(dict(definition_id=talent['definitionId'], max_rank=talent['maxPoints'], row=row, col=col,
                                      prerequisite=None))
                for index, talent in enumerate(tree['talents']):
                    if 'prereqLocation' in talent:
                        p = talent['prereqLocation']
                        target = locations.get((p['rowIdx'], p['colIdx']))
                        if target is None or nodes[target]['row'] > nodes[index]['row']:
                            raise ValueError('Invalid talent prerequisite')
                        nodes[index]['prerequisite'] = target
                def visit(index, path):
                    if index in path:
                        raise ValueError('Talent prerequisite cycle')
                    target = nodes[index]['prerequisite']
                    if target is not None:
                        visit(target, path | {index})
                for index in range(len(nodes)):
                    visit(index, set())
                definitions.extend(dict(definition_id=n['definition_id'], max_rank=n['max_rank']) for n in nodes)
                trees.append(nodes)
            if definitions != mapping['classes'][name]['talents']:
                raise ValueError('Talent identity mismatch')
            result['classes'][name] = trees
    return (json.dumps(result, sort_keys=True, separators=(',', ':'))+'\n').encode('ascii')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-zip', required=True)
    parser.add_argument('--check', required=True)
    args = parser.parse_args()
    from pathlib import Path
    data = build(Path(args.source_zip))
    if data != Path(args.check).read_bytes():
        parser.error('Bundled rules differ from pinned source')
    print(json.dumps(dict(status='matches', sha256=hashlib.sha256(data).hexdigest(), classes=9)))


if __name__ == '__main__':
    main()
