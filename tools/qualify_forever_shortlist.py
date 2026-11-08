"""Offline staged-shortlist audits against fixed runs of an explicit gear pool."""
import argparse
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
from time import monotonic

from .rank_forever_slot import audit_shortlist
from .run_forever_sim import compare, compare_bulk, read_object, verify_engine


def audit_pool(requests, seeds, top, engine, engine_sha256, bulk_engine, bulk_engine_sha256):
    if not isinstance(requests, list) or not 2 <= len(requests) <= 129:
        raise ValueError("Supply 2 to 129 explicit gear requests including the baseline")
    if (not isinstance(seeds, list) or not 1 <= len(seeds) <= 3
            or any(type(seed) is not int or not 1 <= seed <= 2**32 - 1 for seed in seeds)
            or len(set(seeds)) != len(seeds)):
        raise ValueError("Supply one to three distinct positive 32-bit seeds")
    if type(top) is not int or not 1 <= top <= 10:
        raise ValueError("Shortlist size must be 1 to 10")
    if any(not isinstance(request, dict) or not isinstance(request.get('simOptions'), dict)
           or type(request['simOptions'].get('iterations')) is not int
           or not 200 <= request['simOptions']['iterations'] <= 5000 for request in requests):
        raise ValueError("Every request requires 200 to 5000 fixed iterations")
    normalised = deepcopy(requests)
    for request in normalised:
        request['simOptions']['randomSeed'] = str(seeds[0])
    canonical = {json.dumps(request, sort_keys=True, separators=(',', ':'), allow_nan=False): request
                 for request in normalised}
    if sum(request['simOptions']['iterations'] for request in canonical.values()) > 100_000:
        raise ValueError("Fixed comparison exceeds the 100000-iteration budget")
    # Verify both executable identities before paying for any simulation.
    verify_engine(engine, engine_sha256)
    verify_engine(bulk_engine, bulk_engine_sha256)
    digest = hashlib.sha256(json.dumps(requests, sort_keys=True, separators=(',', ':'),
                                       allow_nan=False).encode()).hexdigest()
    runs = []
    for seed in seeds:
        pool = deepcopy(requests)
        for request in pool:
            request['simOptions']['randomSeed'] = str(seed)
        start = monotonic()
        staged = compare_bulk(bulk_engine, bulk_engine_sha256, pool, top)
        staged_seconds = monotonic() - start
        start = monotonic()
        fixed = compare(engine, engine_sha256, pool)
        fixed_seconds = monotonic() - start
        runs.append({
            'seed': seed, **audit_shortlist(staged, fixed, pool, top),
            'fixed_wall_seconds': fixed_seconds, 'staged_wall_seconds': staged_seconds,
            'stage_metrics': staged.get('stage_metrics', []) if staged is not None else [],
        })
    return {
        'basis': 'fixed_point_estimate_shortlist_recall_not_optimality_proof',
        'request_pool_sha256': digest, 'requested_gear_sets': len(requests),
        'engine_sha256': engine_sha256.lower(), 'bulk_engine_sha256': bulk_engine_sha256.lower(),
        'top': top, 'runs': runs,
        'runs_with_missed_fixed_top': sum(bool(run['fixed_top_missing_from_shortlist']) for run in runs),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pool', type=Path, required=True, help='JSON object containing requests array')
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--engine-sha256', required=True)
    parser.add_argument('--bulk-engine', type=Path, required=True)
    parser.add_argument('--bulk-engine-sha256', required=True)
    parser.add_argument('--seed', type=int, action='append', required=True)
    parser.add_argument('--top', type=int, default=5)
    args = parser.parse_args()
    try:
        result = audit_pool(read_object(args.pool, 16 * 1024 * 1024).get('requests'),
                            args.seed, args.top, args.engine, args.engine_sha256,
                            args.bulk_engine, args.bulk_engine_sha256)
    except (ValueError, OSError, KeyError, TypeError, IndexError, AttributeError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Shortlist audit failed: {error}\n")
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
