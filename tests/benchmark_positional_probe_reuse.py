"""Compare actual baseline/candidate adapters on identical cached native requests."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import types

from tests.benchmark_joint_build_optimizer import fixture
from tools import joint_build_optimizer as joint, run_forever_sim as candidate
from tools.simulation_cache import SimulationCache

BASELINE = '8174d3eb343a7adb4b821139acd49a58b00e145c'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--cache-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not all(path.is_absolute() for path in (args.engine, args.cache_root, args.output)):
        parser.error('Explicit absolute paths are necessary')
    os.environ['GOMAXPROCS'] = '2'
    source = subprocess.run(['git', 'show', BASELINE + ':tools/run_forever_sim.py'],
                            check=True, capture_output=True, timeout=15).stdout
    baseline = types.ModuleType('tools._probe_reuse_baseline')
    baseline.__package__ = 'tools'
    exec(compile(source, 'baseline_run_forever_sim.py', 'exec'), baseline.__dict__)
    original_adapter = joint.adapter
    cache = SimulationCache(args.cache_root)
    rows = []
    try:
        for seed in (20261115, 20271115, 20281115):
            outputs = []
            for name, adapter in [('baseline', baseline), ('candidate', candidate)]:
                joint.adapter = adapter
                original_verify, original_run = adapter.verify_engine, subprocess.run
                counters = dict(engine_hash_checks=0, capability_processes=0)

                def verify(*values, **kwargs):
                    counters['engine_hash_checks'] += 1
                    return original_verify(*values, **kwargs)

                def run(command, **kwargs):
                    if 'azeroth-equipment-contract' in command:
                        counters['capability_processes'] += 1
                    return original_run(command, **kwargs)

                adapter.verify_engine = verify
                subprocess.run = run
                try:
                    start = time.perf_counter()
                    result = joint.evaluate_joint_build_case(fixture(seed), args.engine, cache=cache)
                    elapsed = time.perf_counter() - start
                finally:
                    adapter.verify_engine = original_verify
                    subprocess.run = original_run
                assert all(row['cache_status'] == 'hit' for row in result['results'])
                outputs.append(result)
                rows.append(dict(seed=seed, adapter=name, wall_seconds=elapsed,
                                 search_id=result['search_id'], cache_hits=32, **counters))
            assert outputs[0] == outputs[1], 'A performance change altered returned results'
    finally:
        joint.adapter = original_adapter
    record = dict(baseline_commit=BASELINE, baseline_source_sha256=hashlib.sha256(source).hexdigest(),
                  candidate_source_sha256=hashlib.sha256(Path(candidate.__file__).read_bytes()).hexdigest(),
                  candidate_source_lf_sha256=hashlib.sha256(Path(candidate.__file__).read_bytes().replace(b'\r\n', b'\n')).hexdigest(),
                  engine_sha256=joint.layout.ENGINE_SHA256, GOMAXPROCS='2', runs=rows,
                  identical_results=True, game_execution=False,
                  notes='Alternating baseline then candidate on the same host and complete cached public case. '
                        'No tracemalloc. Counters wrap real calls. This measures adapter overhead, not simulator accuracy or competitors.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(record))


if __name__ == '__main__':
    main()
