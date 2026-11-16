"""Local stdio tools over a fixed private saved-character source and pinned runtimes."""
import argparse
import asyncio
from copy import deepcopy
from datetime import datetime, timezone
import os
import stat
from jsonschema import validate as validate_schema

from tools import build_search as wire, gear_builds, talent_builds
from tools.owned_build_optimizer import OwnedBuildOptimizer, bounded, STORAGE
from tools.owned_build_source import OwnedBuildSource, read_json
from tools.simulation_cache import SimulationCache, private_windows_path
from tools.sync_exports import absolute, ordinary

ERROR = 'Owned build tool unavailable, busy, changed or invalid'
LIST, PREPARE, EVALUATE = ('list_owned_build_characters', 'prepare_owned_build_search', 'evaluate_owned_build_window')
DESCRIPTION = ('Local saved captures only; no live character, current ownership, native legality or global optimum claim. '
               'Default selection is the configured saved export character; explicit name and realm override it. '
               'Saved age is returned. Native scoring supports level 60 DPS only and needs an explicit model; other cases stay plan-only. ')


def schemas():
    def obj(properties, required=()):
        return dict(type='object', properties=properties, required=list(required), additionalProperties=False)
    character = obj({k: dict(type='string', minLength=1, maxLength=256) for k in ('name', 'realm')}, ('name', 'realm'))
    shared = dict(character=dict(anyOf=[character, dict(type='null')]),
                  offset=dict(type='string', pattern='^(?:0|[1-9][0-9]{0,79})(?![\\s\\S])', default='0'),
                  limit=dict(type='integer', minimum=1, maximum=64, default=16))
    return {LIST: obj(dict(offset=dict(type='integer', minimum=0, maximum=1024, default=0),
                          limit=dict(type='integer', minimum=1, maximum=64, default=20))),
            PREPARE: obj(shared | dict(model=dict(anyOf=[dict(type='object'), dict(type='null')]),
                gear_choices=dict(anyOf=[gear_builds.schemas()['generate_gear_builds']['properties']['choices'], dict(type='null')]),
                talent_choices=dict(anyOf=[talent_builds.schemas()['generate_talent_builds']['properties']['choices'], dict(type='null')]))),
            EVALUATE: obj(shared | dict(expected_search_id=dict(type='string', pattern='^[0-9a-f]{64}(?![\\s\\S])')),
                          ('expected_search_id',))}


def age(captured_at):
    stamp = datetime.fromisoformat(captured_at.replace('Z', '+00:00'))
    if stamp.tzinfo is None:
        raise ValueError(ERROR)
    return max(0, round((datetime.now(timezone.utc)-stamp).total_seconds()))


def output(report):
    report = deepcopy(report)
    if 'selection' in report:
        report['selection']['age_seconds'] = age(report['selection']['captured_at'])
    for row in report.get('characters', []):
        row['age_seconds'] = age(row['captured_at'])
    envelope = dict(content=[dict(type='text', text=wire.encoded(report))], isError=False)
    if len(wire.encoded(envelope).encode('ascii')) > wire.MAX_BYTES:
        raise ValueError(ERROR)
    return report


def validate(name, arguments):
    schema = schemas().get(name)
    if (schema is None or type(arguments) is not dict or not set(schema['required']) <= arguments.keys()
            or set(arguments)-schema['properties'].keys()):
        raise ValueError(ERROR)
    arguments = bounded(arguments)
    validate_schema(arguments, schema)
    return arguments


async def dispatch(name, arguments, optimizer):
    arguments = validate(name, arguments)
    if name == LIST:
        offset, limit = arguments.get('offset', 0), arguments.get('limit', 20)
        if type(offset) is not int or not 0 <= offset <= 1024:
            raise ValueError(ERROR)
        wire.integer(limit, 64)
        rows = optimizer.source.list_characters()['characters']
        if offset > len(rows):
            raise ValueError(ERROR)
        selected = []
        for row in rows[offset:offset+limit]:
            clean = {k: row[k] for k in ('character', 'client', 'snapshot_id', 'captured_at')}
            clean['storage'] = {scope: {k: row['storage'][scope][k] for k in ('observed', 'quantity', 'item_locations')
                                       if k in row['storage'][scope]} for scope in STORAGE}
            selected.append(clean)
        report = dict(characters=selected, character_count=len(rows), offset=offset,
                      next_offset=offset+limit if offset+limit < len(rows) else None,
                      live=False, active_character_verified=False, all_characters_covered=False)
    else:
        report = await (optimizer.prepare if name == PREPARE else optimizer.evaluate)(**arguments)
    return output(report)


def serve(optimizer):
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import CallToolResult, TextContent, Tool, ToolAnnotations
    server = Server('Azeroth Saved Owned Builds')
    busy = asyncio.Lock()
    def release(completed):
        busy.release()
        if not completed.cancelled():
            completed.exception()
    @server.list_tools()
    async def list_tools():
        return [Tool(name=name, inputSchema=schema, description=DESCRIPTION + {
            LIST: 'List saved characters and captured storage counts, without historical or shared banks.',
            PREPARE: 'Prepare a bounded owned gear and talent matrix; no model or raw inventory is returned.',
            EVALUATE: 'Evaluate a remembered search window; repeat the same explicit selector if used at preparation.'}[name],
            annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False))
            for name, schema in schemas().items()]
    @server.call_tool(validate_input=False)
    async def call_tool(name, arguments):
        try:
            arguments = validate(name, arguments)
            if busy.locked():
                raise ValueError(ERROR)
            await busy.acquire()
            task = asyncio.create_task(asyncio.to_thread(lambda: asyncio.run(dispatch(name, arguments, optimizer))))
            task.add_done_callback(release)
            async with asyncio.timeout(300):
                result = await asyncio.shield(task)
            return [TextContent(type='text', text=wire.encoded(output(result)))]
        except Exception:
            return CallToolResult(isError=True, content=[TextContent(type='text', text=ERROR)])
    async def run():
        async with stdio_server() as (read, write):
            await server.run(read, write, server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('serve',))
    for key in ('source', 'catalogue', 'probe', 'engine'):
        parser.add_argument('--'+key, required=True)
    parser.add_argument('--cache-root')
    args = parser.parse_args()
    try:
        path = ordinary(absolute(args.source))
        info = path.stat()
        if (not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or
                not (private_windows_path(path) if os.name == 'nt' else
                     info.st_uid == os.getuid() and not info.st_mode & 0o077)):
            raise ValueError(ERROR)
        source = OwnedBuildSource(read_json(path, 64*1024))
        cache = SimulationCache(ordinary(absolute(args.cache_root))) if args.cache_root else None
        serve(OwnedBuildOptimizer(source, args.catalogue, args.probe, args.engine, cache))
    except Exception:
        parser.exit(1, ERROR+'\n')


if __name__ == '__main__':
    main()
