# Agentic Benchmarks

This folder defines task suites for `scripts/agent_benchmark.py`.

## Files

- `agent_tasks.json`: default benchmark suite.

## Scenario schema

Each scenario is a JSON object:

- `id`: stable scenario id.
- `description`: short goal.
- `tags`: classification labels.
- `weight`: score weight.
- `turns`: list of prompts (multi-turn scenarios supported).
- `checks`: expected behavior checks.

Supported checks:

- `equals`: exact output match.
- `must_contain`: all strings must appear.
- `must_not_contain`: none of the strings may appear.
- `any_of`: at least one string must appear.
- `numbered_steps_min`: minimum count of numbered steps (`1.` or `Paso 1` style).

## Tuning loop

Recommended cycle:

1. Clone OpenClaw workspace into isolated profile.
2. Run benchmark loops with heuristics enabled.
3. Apply quality gate thresholds.
4. Promote only validated changes.
