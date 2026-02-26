#!/usr/bin/env python3
"""
Benchmark quality gate.

Fails (exit 1) when the latest loop score is below configured thresholds.
Useful for CI/CD or scheduled reliability guards.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Apply quality gates to agent benchmark summary.")
    parser.add_argument("--summary", required=True, help="Path to <run>.summary.json")
    parser.add_argument(
        "--min-pass-rate",
        type=float,
        default=70.0,
        help="Minimum pass rate percentage required (default: 70.0)",
    )
    parser.add_argument(
        "--min-score-ratio",
        type=float,
        default=0.70,
        help="Minimum score/max_score ratio required (default: 0.70)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_path = Path(args.summary).resolve()
    if not summary_path.exists():
        print(f"[GATE] summary not found: {summary_path}")
        return 2

    data = json.loads(summary_path.read_text(encoding="utf-8"))
    loops = data.get("loops", [])
    if not loops:
        print("[GATE] summary has no loops")
        return 2

    latest = loops[-1]
    score = float(latest.get("score", 0.0))
    max_score = float(latest.get("max_score", 0.0))
    pass_rate = float(latest.get("pass_rate", 0.0))
    ratio = (score / max_score) if max_score > 0 else 0.0

    print(
        f"[GATE] latest loop={latest.get('loop_index')} "
        f"score={score:.2f}/{max_score:.2f} ratio={ratio:.3f} pass_rate={pass_rate:.2f}%"
    )

    failures = []
    if pass_rate < args.min_pass_rate:
        failures.append(
            f"pass_rate {pass_rate:.2f}% < min_pass_rate {args.min_pass_rate:.2f}%"
        )
    if ratio < args.min_score_ratio:
        failures.append(f"score_ratio {ratio:.3f} < min_score_ratio {args.min_score_ratio:.3f}")

    if failures:
        print("[GATE] FAILED")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("[GATE] PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
