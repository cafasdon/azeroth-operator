"""Offline joint build experiment windows; no simulation, ranking or game input."""
import argparse
import hashlib
import json
import re

from tools.sync_exports import absolute, ordinary

MAX_BYTES = 256 * 1024
MAX_INPUT_BYTES = 512 * 1024
# Change the format version when identity or coordinate-order semantics change.
FORMAT_VERSION = 1
HASH_PATTERN = r'[0-9a-f]{64}'
TEXT_PATTERN = r'[A-Za-z0-9][A-Za-z0-9_. -]*'
NAME_PATTERN = r'[A-Za-z_][A-Za-z0-9_]*'
OFFSET_PATTERN = r'(?:0|[1-9][0-9]{0,79})'
DIRECTIONS = dict(dps='maximize', hps='maximize', tps='maximize', dtps='minimize', death='minimize', oom='minimize')
CATEGORIES = ('gear', 'talents', 'rotation', 'timing', 'consumables')
HASH_KEYS = ('snapshot_id', 'engine_sha256', 'data_manifest_sha256', 'encounter_sha256', 'constraints_sha256')
CASE_KEYS = {*HASH_KEYS, 'client', 'character_level', 'class_id', 'specialization', 'objective'}


def encoded(value):
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(',', ':'), allow_nan=False)


def digest(value):
    return hashlib.sha256(encoded(value).encode('ascii')).hexdigest()


def exact(value, keys):
    if type(value) is not dict or set(value) != set(keys):
        raise ValueError('Unexpected object fields')


def text(value, pattern, maximum):
    if type(value) is not str or not 1 <= len(value) <= maximum or re.fullmatch(pattern, value) is None:
        raise ValueError('Invalid bounded text')
    return value


def integer(value, maximum):
    if type(value) is not int or not 1 <= value <= maximum:
        raise ValueError('Invalid bounded integer')
    return value


def prepare_build_search_window(case, axes, offset='0', limit=16, expected_search_id=None):
    """Enumerate a half-open window; the last declared axis varies fastest."""
    exact(case, CASE_KEYS)
    client = case['client']
    exact(client, ('family', 'variant', 'build', 'interface'))
    if type(client['family']) is not str or client['family'] != 'forever':
        raise ValueError('Only explicit forever case bindings are supported')
    clean = dict(client=dict(family='forever', variant=text(client['variant'], TEXT_PATTERN, 32),
                            build=text(client['build'], TEXT_PATTERN, 64), interface=integer(client['interface'], 4294967295)),
                 character_level=integer(case['character_level'], 255), class_id=integer(case['class_id'], 255),
                 specialization=text(case['specialization'], TEXT_PATTERN, 64))
    clean.update({key: text(case[key], HASH_PATTERN, 64) for key in HASH_KEYS})
    if type(case['objective']) is not str or case['objective'] not in DIRECTIONS:
        raise ValueError('Unknown objective')
    clean['objective'] = case['objective']
    if type(axes) is not list or not 1 <= len(axes) <= 32:
        raise ValueError('Expected 1 to 32 ordered axes')
    matrix, names, total = [], set(), 1
    for axis in axes:
        exact(axis, ('name', 'category', 'candidates'))
        name = text(axis['name'], NAME_PATTERN, 64)
        if name in names or type(axis['category']) is not str or axis['category'] not in CATEGORIES:
            raise ValueError('Duplicate axis name or unknown category')
        values = axis['candidates']
        if type(values) is not list or not 1 <= len(values) <= 128:
            raise ValueError('Expected 1 to 128 ordered references per axis')
        values = [text(value, HASH_PATTERN, 64) for value in values]
        if len(set(values)) != len(values):
            raise ValueError('Duplicate references within an axis')
        matrix.append(dict(name=name, category=axis['category'], candidates=values))
        names.add(name)
        total *= len(values)
    search_id = digest(dict(format_version=FORMAT_VERSION, case=clean, axes=matrix))
    if expected_search_id is not None and text(expected_search_id, HASH_PATTERN, 64) != search_id:
        raise ValueError('Search identity changed; resume rejected')
    start = int(text(offset, OFFSET_PATTERN, 80))
    integer(limit, 64)
    if start > total:
        raise ValueError('Offset exceeds the declared matrix')
    end, candidates = min(start + limit, total), []
    for ordinal in range(start, end):
        coordinate, selection = ordinal, {}
        for axis in reversed(matrix):
            coordinate, index = divmod(coordinate, len(axis['candidates']))
            selection[axis['name']] = axis['candidates'][index]
        selection = {axis['name']: selection[axis['name']] for axis in matrix}
        candidate = dict(ordinal_decimal=str(ordinal), selection=selection)
        candidates.append(candidate | {'candidate_sha256': digest(dict(search_id=search_id, **candidate))})
    result = dict(format_version=FORMAT_VERSION, search_id=search_id, case=clean,
                  axes=[dict(name=a['name'], category=a['category'], count=len(a['candidates']),
                             digest=digest(a)) for a in matrix],
                  total_decimal=str(total), start_decimal=str(start), end_decimal=str(end),
                  next_offset=str(end) if end < total else None, candidates=candidates, window_count=len(candidates),
                  objective_direction=DIRECTIONS[clean['objective']], plan_only=True,
                  simulation_performed=False, objective_evaluated=False, candidate_legality_verified=False,
                  model_compatibility_verified=False, source_data_complete_verified=False, constraints_verified=False,
                  snapshot_reference_verified=False, active_character_verified=False,
                  all_owned_items_covered=False, global_optimum_verified=False,
                  coverage='Only the declared ordered reference matrix is planned. References are not resolved. '
                  'No complete build tree or owned inventory coverage is claimed. '
                  'A window or terminal offset is not proof that any earlier coordinate was visited or evaluated. '
                  'Start is inclusive; end is exclusive. The last declared axis varies fastest.')
    envelope = {'content': [{'type': 'text', 'text': encoded(result)}], 'isError': False}
    if len(encoded(envelope).encode('ascii')) > MAX_BYTES - 1024:
        raise ValueError('Window exceeds 256 KiB transport budget; reduce limit')
    return result


def prepare_request(value):
    if (type(value) is not dict or not {'case', 'axes'} <= set(value)
            or set(value) - {'case', 'axes', 'offset', 'limit', 'expected_search_id'}):
        raise ValueError('Expected case, axes and optional offset, limit, expected_search_id')
    return prepare_build_search_window(**value)


def input_schema():
    def string(pattern, maximum):
        return dict(type='string', minLength=1, maxLength=maximum, pattern='^(?:' + pattern + ')(?![\\s\\S])')
    def number(maximum):
        return dict(type='integer', minimum=1, maximum=maximum)
    def obj(properties, required=None):
        return dict(type='object', additionalProperties=False, required=list(properties) if required is None else required,
                    properties=properties)
    sha = string(HASH_PATTERN, 64)
    case = obj(dict(client=obj(dict(family={'const': 'forever', 'type': 'string'},
                                   variant=string(TEXT_PATTERN, 32), build=string(TEXT_PATTERN, 64), interface=number(4294967295))),
                    **{key: sha for key in HASH_KEYS}, character_level=number(255), class_id=number(255),
                    specialization=string(TEXT_PATTERN, 64), objective={'type': 'string', 'enum': list(DIRECTIONS)}))
    axis = obj(dict(name=string(NAME_PATTERN, 64), category={'type': 'string', 'enum': list(CATEGORIES)},
                    candidates=dict(type='array', minItems=1, maxItems=128, uniqueItems=True, items=sha)))
    return obj(dict(case=case, axes=dict(type='array', minItems=1, maxItems=32, items=axis),
                    offset=string(OFFSET_PATTERN, 80) | {'default': '0'}, limit=number(64) | {'default': 16},
                    expected_search_id={'anyOf': [sha, {'type': 'null'}]}), ['case', 'axes'])


def serve():
    import asyncio
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import TextContent, Tool, ToolAnnotations
    server = Server('Azeroth Offline Joint Build Experiment Planner')

    @server.list_tools()
    async def list_tools():
        return [Tool(name='prepare_build_search_window', inputSchema=input_schema(),
                     description='Plan an OFFLINE joint build experiment window over explicit reference hashes. '
                     'No simulation, objective score, ranking, native legality, inventory completeness or global optimum.',
                     annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False, idempotentHint=True, openWorldHint=False))]

    @server.call_tool(validate_input=False)
    async def call_tool(name, arguments):
        if name != 'prepare_build_search_window':
            raise ValueError('Unknown build search tool')
        try:
            return [TextContent(type='text', text=encoded(prepare_request(arguments)))]
        except (ValueError, TypeError, OverflowError, RecursionError):
            raise ValueError('Invalid or oversized build search window') from None

    async def run():
        async with stdio_server() as (read, write):
            await server.run(read, write, server.create_initialization_options())
    asyncio.run(run())


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate JSON keys are not supported')
        result[key] = value
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest='action', required=True)
    actions.add_parser('serve')
    actions.add_parser('prepare').add_argument('--input', required=True)
    args = parser.parse_args()
    if args.action == 'serve':
        serve()
        return
    try:
        path = ordinary(absolute(args.input))
        if not path.is_file() or path.stat().st_nlink != 1:
            raise ValueError('Input must be an ordinary single-link file')
        with path.open('rb') as stream:
            payload = stream.read(MAX_INPUT_BYTES + 1)
        if len(payload) > MAX_INPUT_BYTES:
            raise ValueError('Input exceeds 512 KiB')
        print(encoded(prepare_request(json.loads(payload, object_pairs_hook=unique_object))))
    except (OSError, ValueError, TypeError, RecursionError):
        parser.error('Invalid build search input; use an absolute ordinary JSON file within 512 KiB')


if __name__ == '__main__':
    main()
