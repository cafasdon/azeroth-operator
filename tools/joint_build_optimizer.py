"""Bounded offline native joint experiments over an explicit complete reference matrix."""
import argparse
import asyncio
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import re
import sqlite3
import subprocess

from google.protobuf import descriptor_pb2, descriptor_pool, json_format, message_factory
from tools import build_search as planner, positional_equipment as layout, run_forever_sim as adapter
from tools.simulation_cache import SimulationCache
from tools.sync_exports import absolute, ordinary

ROOT = Path(__file__).resolve().parents[1]
DESCRIPTOR_PATH = ROOT / 'clients/forever/sim-request-descriptor.pb'
DESCRIPTOR_SHA256 = '3d06a6abeec9567db5c891b4f39c7d5827ec68503e9a1df95f329c3128600225'
CATALOGUE_PATH = ROOT / 'clients/forever/catalogue.lock.json'
CATALOGUE_SHA256 = '2383a1aebf9745a88fb257dad343d323e9c9a16c91ab808dd3817c2b1a254300'
FIELDS = dict(gear='equipment', talents='talentsString', rotation='rotation', timing='cooldowns', consumables='consumables')
ERROR = 'Joint build experiment failed; check the bounded manifest and pinned engine.'
FAILURES = (OSError, ValueError, TypeError, KeyError, AttributeError, OverflowError, RecursionError,
            sqlite3.Error, subprocess.SubprocessError, json_format.ParseError)


def request_type():
    data = DESCRIPTOR_PATH.read_bytes()
    if hashlib.sha256(data).hexdigest() != DESCRIPTOR_SHA256:
        raise ValueError('Request schema changed')
    bundle = descriptor_pb2.FileDescriptorSet.FromString(data)
    pool = descriptor_pool.DescriptorPool()
    for file in bundle.file:
        pool.Add(file)
    return message_factory.GetMessageClass(pool.FindMessageTypeByName('proto.RaidSimRequest'))


def check_scalar(field, value):
    """Reject protobuf's coercions; only canonical field names and strict JSON scalars."""
    if field.message_type is not None:
        check_message(field.message_type, value)
    elif field.enum_type is not None:
        if not ((type(value) is str and value in field.enum_type.values_by_name)
                or (type(value) is int and value in field.enum_type.values_by_number)):
            raise ValueError('Unknown enum')
    elif field.type == field.TYPE_BOOL:
        if type(value) is not bool:
            raise ValueError('Expected boolean')
    elif field.type in (field.TYPE_STRING, field.TYPE_BYTES):
        if type(value) is not str:
            raise ValueError('Expected string')
    elif field.type in (field.TYPE_DOUBLE, field.TYPE_FLOAT):
        if type(value) not in (int, float) or not math.isfinite(value):
            raise ValueError('Expected finite number')
    elif field.type in (field.TYPE_INT64, field.TYPE_UINT64, field.TYPE_SINT64, field.TYPE_FIXED64, field.TYPE_SFIXED64):
        if type(value) is not str or re.fullmatch(r'(?:0|-?[1-9][0-9]*)', value) is None:
            raise ValueError('Expected canonical 64-bit integer string')
    elif type(value) is not int:
        raise ValueError('Expected integer')


def check_message(descriptor, value):
    if type(value) is not dict:
        raise ValueError('Expected native object')
    fields = {field.json_name: field for field in descriptor.fields}
    if set(value) - fields.keys():
        raise ValueError('Unknown or noncanonical native field')
    for name, item in value.items():
        field = fields[name]
        if field.is_repeated:
            if field.message_type and field.message_type.GetOptions().map_entry:
                if type(item) is not dict:
                    raise ValueError('Expected native map')
                for entry in item.values():
                    check_scalar(field.message_type.fields_by_name['value'], entry)
            else:
                if type(item) is not list:
                    raise ValueError('Expected native list')
                for entry in item:
                    check_scalar(field, entry)
        else:
            check_scalar(field, item)


def canonical_request(value, case, message_type):
    check_message(message_type.DESCRIPTOR, value)
    # ParseDict rejects conflicting oneof members, integer overflow and malformed maps.
    message = json_format.ParseDict(value, message_type(), ignore_unknown_fields=False)
    result = json_format.MessageToDict(message)
    options = value.get('simOptions', {})
    planner.exact(options, ('iterations', 'randomSeed'))
    if type(options['iterations']) is not int or not 2 <= options['iterations'] <= 5000:
        raise ValueError('Invalid iteration count')
    seed = options['randomSeed']
    if type(seed) is not str or re.fullmatch(r'[1-9][0-9]{0,9}', seed) is None or int(seed) > 2**32 - 1:
        raise ValueError('Invalid seed')
    if len(result['raid'].get('parties', [])) != 1 or result['raid'].get('numActiveParties', 0) not in (0, 1):
        raise ValueError('Exactly one active party required')
    player = adapter.single_player(result['raid'])
    original = adapter.single_player(value['raid'])
    spec = adapter.player_spec(player)
    class_name = adapter.SPEC_CLASSES[spec]
    if (spec != case['specialization'] or adapter.CLASS_IDS[class_name] != case['class_id']
            or player.get('class') != 'Class' + class_name.capitalize()):
        raise ValueError('Case class or specialisation disagrees')
    if (player.get('enableItemSwap') or player.get('database') or result['raid'].get('targetDummies')
            or result.get('type') != 'SimTypeIndividual' or not player.get('race')
            or type(original.get('talentsString')) is not str):
        raise ValueError('Unsupported player context')
    rotation = player.get('rotation', {})
    if rotation.get('type') != 'TypeAPL' or not any(
            row.get('action') and set(row['action']) - {'condition'} and not row.get('hide')
            for row in rotation.get('priorityList', [])):
        raise ValueError('Explicit actionable APL required')
    equipment = player.get('equipment', {})
    if equipment.get('preserveSlots') is not True or len(equipment.get('items', [])) != 17:
        raise ValueError('Explicit positional equipment required')
    encounter = result.get('encounter', {})
    duration, variation = encounter.get('duration', 0), encounter.get('durationVariation', 0)
    if not (1 <= duration <= 3600 and 0 <= variation < duration
            and 1 <= len(encounter.get('targets', [])) <= 20
            and 0 <= encounter.get('executeProportion20', 0) <= encounter.get('executeProportion35', 0) <= 1):
        raise ValueError('Unbounded encounter')
    return result


def preflight(manifest, expected_search_id=None):
    planner.exact(manifest, ('case', 'axes', 'base_request', 'references'))
    case, axes, references = manifest['case'], manifest['axes'], manifest['references']
    plan = planner.prepare_build_search_window(case, axes, limit=64)
    if int(plan['total_decimal']) > 64 or len({a['category'] for a in axes}) != len(axes):
        raise ValueError('Expected a complete matrix of at most 64 coordinates and distinct categories')
    if case['character_level'] != 60 or case['objective'] != 'dps' or case['engine_sha256'] != layout.ENGINE_SHA256:
        raise ValueError('Unsupported native experiment case')
    if (case['data_manifest_sha256'] != CATALOGUE_SHA256
            or hashlib.sha256(CATALOGUE_PATH.read_bytes()).hexdigest() != CATALOGUE_SHA256
            or planner.digest(manifest['base_request'].get('encounter')) != case['encounter_sha256']):
        raise ValueError('Case data or encounter binding changed')
    expected = {ref for axis in axes for ref in axis['candidates']}
    planner.exact(references, expected)
    for axis in axes:
        for ref in axis['candidates']:
            entry = references[ref]
            planner.exact(entry, ('category', 'value'))
            if entry['category'] != axis['category'] or planner.digest(entry) != ref:
                raise ValueError('Reference content or category mismatch')
    schema = request_type()
    baseline = canonical_request(manifest['base_request'], case, schema)
    plan['planner_search_id'] = plan['search_id']
    plan['search_id'] = planner.digest(dict(format_version=1, planner_search_id=plan['planner_search_id'],
                                           base_request_sha256=planner.digest(baseline)))
    if expected_search_id is not None and planner.text(expected_search_id, planner.HASH_PATTERN, 64) != plan['search_id']:
        raise ValueError('Native search identity changed')
    requests, coordinates = {}, []
    for candidate in plan['candidates']:
        request = deepcopy(manifest['base_request'])
        player = adapter.single_player(request['raid'])
        for axis in axes:
            player[FIELDS[axis['category']]] = deepcopy(references[candidate['selection'][axis['name']]]['value'])
        request = canonical_request(request, case, schema)
        key = planner.digest(request)
        requests.setdefault(key, request)
        binding = dict(search_id=plan['search_id'], ordinal_decimal=candidate['ordinal_decimal'], selection=candidate['selection'])
        coordinates.append(candidate | dict(candidate_sha256=planner.digest(binding), request_sha256=key))
    iterations = sum(r['simOptions']['iterations'] for r in requests.values())
    if iterations > 100_000:
        raise ValueError('Native iteration budget exceeded')
    # Reserve 1 KiB per coordinate for numeric telemetry before any native execution.
    if len(planner.encoded(coordinates).encode('ascii')) * 2 + 1024 * len(coordinates) + 8192 > planner.MAX_BYTES:
        raise ValueError('Result exceeds transport budget')
    return plan, requests, coordinates, iterations


def evaluate_joint_build_case(manifest, engine, expected_search_id=None, cache=None):
    plan, requests, coordinates, iterations = preflight(manifest, expected_search_id)
    engine = ordinary(absolute(engine))
    adapter.verify_engine(engine, layout.ENGINE_SHA256)
    comparison = adapter.compare(engine, layout.ENGINE_SHA256, list(requests.values()), cache=cache)
    rows = comparison['results']
    if len(rows) != len(requests):
        raise ValueError('Incomplete simulation results')
    scores = {}
    for key, row in zip(requests, rows, strict=True):
        mean, deviation = row['mean_dps'], row['dps_standard_deviation']
        if (any(type(n) not in (int, float) or not math.isfinite(n) or n < 0 for n in (mean, deviation))
                or type(row['iterations']) is not int or row['iterations'] != requests[key]['simOptions']['iterations']
                or row['seed'] != requests[key]['simOptions']['randomSeed']
                or row['engine_sha256'] != layout.ENGINE_SHA256):
            raise ValueError('Invalid native numeric result')
        scores[key] = dict(mean_dps=mean, dps_standard_deviation=deviation, iterations=row['iterations'],
                           seed=row['seed'], cache_status=row.get('cache_status', 'disabled')
                           if row.get('cache_status', 'disabled') in ('hit', 'miss', 'disabled') else 'disabled')
    ranked = [c | scores[c['request_sha256']] for c in coordinates]
    ranked.sort(key=lambda c: -c['mean_dps'])  # Stable declared order for exact observed ties.
    for index, row in enumerate(ranked):
        row['rank'] = 1 + next(i for i, other in enumerate(ranked) if other['mean_dps'] == row['mean_dps'])
        row['tied_observed_mean'] = sum(other['mean_dps'] == row['mean_dps'] for other in ranked) > 1
    result = dict(search_id=plan['search_id'], planner_search_id=plan['planner_search_id'], case=plan['case'], results=ranked,
                  declared_coordinates=len(coordinates), resolved_coordinates=len(coordinates),
                  evaluated_coordinates=len(coordinates), unique_requests=len(requests),
                  equivalent_coordinates=len(coordinates)-len(requests), unique_native_iterations=iterations,
                  simulation_performed=True, objective_evaluated=True, declared_matrix_complete=True,
                  candidate_legality_verified=False, model_compatibility_verified=False,
                  source_data_complete_verified=False, constraints_verified=False, snapshot_reference_verified=False,
                  active_character_verified=False, all_owned_items_covered=False, global_optimum_verified=False,
                  statistical_superiority_verified=False, game_execution=False,
                  coverage='Only the complete declared matrix was evaluated. Hypothetical model experiments only. '
                  'Snapshot and constraints are prospective references. Observed means do not certify statistical superiority, '
                  'native legality, ownership, the active character or a global optimum.')
    envelope = dict(content=[dict(type='text', text=planner.encoded(result))], isError=False)
    if len(planner.encoded(envelope).encode('ascii')) > planner.MAX_BYTES:
        raise ValueError('Result exceeds transport budget')
    return result


def read_manifest(value):
    path = ordinary(absolute(value))
    if not path.is_file() or path.stat().st_nlink != 1:
        raise ValueError('Expected ordinary single-link manifest')
    with path.open('rb') as stream:
        data = stream.read(planner.MAX_INPUT_BYTES + 1)
    if len(data) > planner.MAX_INPUT_BYTES:
        raise ValueError('Manifest exceeds input budget')
    return json.loads(data, object_pairs_hook=planner.unique_object)


def serve(manifest, engine, cache):
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import TextContent, Tool, ToolAnnotations
    prepared, _, _, _ = preflight(manifest)
    server = Server('Azeroth Offline Native Joint Build Experiments')
    busy = asyncio.Lock()

    def release(completed):
        busy.release()
        if not completed.cancelled():
            completed.exception()  # Consume failures even if the client cancelled its wait.

    @server.list_tools()
    async def list_tools():
        return [Tool(name='evaluate_joint_build_case', description='Fixed search_id: ' + prepared['search_id'] + '. '
                     'Evaluate the fixed offline hypothetical native matrix. '
                     'No game actions, character legality, ownership or global optimum claims.',
                     inputSchema=dict(type='object', additionalProperties=False, required=['expected_search_id'],
                                      properties=dict(expected_search_id=dict(type='string', pattern='^[0-9a-f]{64}$'))),
                     annotations=ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False))]

    @server.call_tool(validate_input=False)
    async def call_tool(name, arguments):
        try:
            if name != 'evaluate_joint_build_case':
                raise ValueError('Unknown tool')
            planner.exact(arguments, ('expected_search_id',))
            planner.text(arguments['expected_search_id'], planner.HASH_PATTERN, 64)
            if busy.locked():
                raise ValueError('An experiment is already active')
            await busy.acquire()
            task = asyncio.create_task(asyncio.to_thread(evaluate_joint_build_case, manifest, engine, arguments['expected_search_id'], cache))
            task.add_done_callback(release)
            result = await asyncio.shield(task)  # Cancellation must not admit a second native batch.
            return [TextContent(type='text', text=planner.encoded(result))]
        except FAILURES:
            raise ValueError(ERROR) from None

    async def run():
        async with stdio_server() as (read, write):
            await server.run(read, write, server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('prepare', 'run', 'serve'))
    parser.add_argument('--input', required=True)
    parser.add_argument('--engine')
    parser.add_argument('--cache-root')
    args = parser.parse_args()
    try:
        manifest = read_manifest(args.input)
        if args.action == 'prepare':
            plan, requests, coordinates, iterations = preflight(manifest)
            print(planner.encoded(dict(search_id=plan['search_id'], planner_search_id=plan['planner_search_id'],
                                       declared_coordinates=len(coordinates), unique_requests=len(requests),
                                       unique_native_iterations=iterations, simulation_performed=False)))
            return
        engine = ordinary(absolute(args.engine))
        cache = SimulationCache(ordinary(absolute(args.cache_root))) if args.cache_root else None
        if args.action == 'serve':
            serve(manifest, engine, cache)
        else:
            print(planner.encoded(evaluate_joint_build_case(manifest, engine, cache=cache)))
    except FAILURES:
        parser.exit(1, ERROR + '\n')


if __name__ == '__main__':
    main()
