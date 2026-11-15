"""Opt-in offline benchmark: public model fragments, independent exhaustive oracle.

python -m tests.benchmark_joint_build_optimizer --engine ABS_EXE --cache-root ABS_PRIVATE_DIR --output ABS_JSON
No download, live character, game input or claim about native legality.
"""
import argparse
from copy import deepcopy
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import time
import tracemalloc

from tools import joint_build_optimizer as optimizer, run_forever_sim as adapter
from tools.simulation_cache import SimulationCache


def sha(value):
    data = json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True, allow_nan=False)
    return hashlib.sha256(data.encode('ascii')).hexdigest()


def fixture(seed, iterations=200):
    root = Path(__file__).parent / 'fixtures'
    bm = json.loads((root / 'forever-hunter-bm.json').read_bytes())
    mm = json.loads((root / 'forever-hunter-mm.json').read_bytes())
    player = deepcopy(bm['player'])
    player['equipment']['preserveSlots'] = True
    encounter = dict(duration=45, durationVariation=0, executeProportion20=0,
                     executeProportion35=0, targets=[{}])
    request = dict(raid=dict(parties=[dict(players=[player])], buffs=bm['raidBuffs'], debuffs=bm['debuffs']),
                   encounter=encounter, simOptions=dict(iterations=iterations, randomSeed=str(seed)), type='SimTypeIndividual')
    less_gear = deepcopy(player['equipment'])
    less_gear['items'][12] = {}
    less_rotation = deepcopy(player['rotation'])
    # A slower polling APL is still a concrete model input, not a new spell.
    wait = {'action': {'wait': {'duration': {'const': {'val': '0.2'}}}}}
    less_rotation['priorityList'].insert(0, wait)
    pools = [
        ('gear', [player['equipment'], less_gear]),
        ('talents', [player['talentsString'], mm['player']['talentsString']]),
        ('rotation', [player['rotation'], less_rotation]),
        ('timing', [{}, dict(cooldowns=[dict(id=dict(spellId=20572), timings=[20])])]),
        ('consumables', [player['consumables'], {}]),
    ]
    references, axes = {}, []
    for category, values in pools:
        candidates = []
        for value in values:
            entry = dict(category=category, value=value)
            key = sha(entry)
            references[key] = entry
            candidates.append(key)
        axes.append(dict(name=category, category=category, candidates=candidates))
    case = dict(client=dict(family='forever', variant='public-model-fixture', build='9dc32ac', interface=16001),
                character_level=60, class_id=3, specialization='hunter', objective='dps',
                snapshot_id=sha(bm), constraints_sha256=sha({'scope': 'hypothetical public fragments only'}),
                engine_sha256=optimizer.layout.ENGINE_SHA256,
                data_manifest_sha256=optimizer.CATALOGUE_SHA256, encounter_sha256=sha(encounter))
    return dict(case=case, axes=axes, base_request=request, references=references)


def independent_oracle(manifest, engine):
    """Apply the Cartesian product directly, without planner/materialiser/dedup helpers."""
    fields = dict(gear='equipment', talents='talentsString', rotation='rotation', timing='cooldowns', consumables='consumables')
    means = []
    deadline = time.monotonic() + 120
    for selection in itertools.product(*(axis['candidates'] for axis in manifest['axes'])):
        request = deepcopy(manifest['base_request'])
        player = request['raid']['parties'][0]['players'][0]
        for axis, reference in zip(manifest['axes'], selection, strict=True):
            player[fields[axis['category']]] = deepcopy(manifest['references'][reference]['value'])
        result = adapter.run(engine, optimizer.layout.ENGINE_SHA256, request, deadline=deadline)
        means.append(result['mean_dps'])
    return means


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--cache-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not all(p.is_absolute() for p in (args.engine, args.cache_root, args.output)):
        parser.error('Use explicit absolute paths')
    os.environ['GOMAXPROCS'] = '2'
    cache = SimulationCache(args.cache_root)
    runs = []
    for seed in (20261115, 20271115, 20281115):
        manifest = fixture(seed)
        start = time.perf_counter()
        result = optimizer.evaluate_joint_build_case(manifest, args.engine, cache=cache)
        cold_seconds = time.perf_counter() - start
        start = time.perf_counter()
        warm = optimizer.evaluate_joint_build_case(manifest, args.engine, cache=cache)
        warm_seconds = time.perf_counter() - start
        # Memory instrumentation is a separate replay, outside both wall-time measurements.
        tracemalloc.start()
        optimizer.evaluate_joint_build_case(manifest, args.engine, cache=cache)
        _, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        oracle = independent_oracle(manifest, args.engine)
        actual = {int(row['ordinal_decimal']): row['mean_dps'] for row in result['results']}
        assert len(actual) == len(oracle) == 32
        assert all(math.isclose(actual[i], mean, rel_tol=1e-12, abs_tol=1e-10) for i, mean in enumerate(oracle))
        assert [r['mean_dps'] for r in result['results']] == [r['mean_dps'] for r in warm['results']]
        assert all(row['cache_status'] == 'hit' for row in warm['results'])
        assert not result['global_optimum_verified'] and not result['statistical_superiority_verified']
        runs.append(dict(seed=seed, search_id=result['search_id'], coordinates=len(actual),
                         unique_requests=result['unique_requests'], unique_native_iterations=result['unique_native_iterations'],
                         cold_cache_misses=sum(r['cache_status'] == 'miss' for r in result['results']),
                         warm_cache_hits=sum(r['cache_status'] == 'hit' for r in warm['results']),
                         cold_seconds=cold_seconds, warm_seconds=warm_seconds,
                         warm_python_peak_traced_bytes=peak, oracle_match=True,
                         observed_leader_ordinals=[r['ordinal_decimal'] for r in result['results'] if r['rank'] == 1],
                         observed_leader_mean_dps=result['results'][0]['mean_dps']))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(dict(engine_sha256=optimizer.layout.ENGINE_SHA256,
            seeds=runs, game_execution=False, native_legality_verified=False,
            notes='Wall time on this host. Python traced memory excludes native engine. Cold is cold only when misses equal unique requests.'), indent=2)+'\n', encoding='utf-8')
        args.output.with_name('public-case.json').write_text(json.dumps(manifest, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(dict(seeds=len(runs), native_coordinates=sum(r['coordinates'] for r in runs),
                         oracle_match=all(r['oracle_match'] for r in runs), output=str(args.output))))


if __name__ == '__main__':
    main()
