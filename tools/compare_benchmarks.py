"""Alternate two compiled synthetic benchmarks; record binary hashes and every run."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pairs", type=int, default=4)
    args = parser.parse_args()
    if not 2 <= args.pairs <= 20:
        parser.error("Expected 2 to 20 pairs")
    paths = {key: getattr(args, key).resolve(strict=True) for key in ("baseline", "candidate")}
    results = {key: [] for key in paths}
    order = []
    for pair in range(args.pairs):
        for key in (["baseline", "candidate"] if pair % 2 == 0 else ["candidate", "baseline"]):
            run = subprocess.run([str(paths[key])], check=True, capture_output=True, timeout=60)
            value = json.loads(run.stdout)
            expected = {"snapshots": 200, "items_per_snapshot": 180, "list_runs": 20,
                        "inspect_runs": 200, "profile": "release", "cache": "warm"}
            if any(value.get(name) != expected_value for name, expected_value in expected.items()):
                raise ValueError("Benchmark fixtures or measurement settings differ")
            results[key].append(value)
            order.append(key)
    medians = {key: {metric: statistics.median(run[metric]["median"] for run in values)
                     for metric in ("list_ms", "inspect_ms")} for key, values in results.items()}
    improvement = {metric: 100 * (1 - medians["candidate"][metric] / medians["baseline"][metric])
                   for metric in ("list_ms", "inspect_ms")}
    evidence = {"binary_sha256": {key: hashlib.sha256(path.read_bytes()).hexdigest() for key, path in paths.items()},
                "run_order": order, "runs": results, "median_of_run_medians": medians,
                "reduction_percent": improvement,
                "limitations": ["Synthetic warm-cache Windows/local filesystem workload",
                                "Different temporary store per invocation; no claim about hosted Linux latency"]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"medians": medians, "reduction_percent": improvement}))


if __name__ == "__main__":
    main()
