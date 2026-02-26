#!/usr/bin/env python3
"""
ZeroClaw Agentic Benchmark System

Features:
- Repeatable benchmark tasks for agent behavior.
- Real or simulated execution modes.
- Multi-loop "agentic" cycle: benchmark -> analyze -> tune isolated config -> rerun.
- Optional self-analysis turn using the agent itself.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run ZeroClaw agentic benchmark loops.")
    parser.add_argument(
        "--exe",
        default="zeroclaw",
        help="Path to zeroclaw executable (default: zeroclaw from PATH).",
    )
    parser.add_argument(
        "--tasks",
        default="benchmarks/agent_tasks.json",
        help="Path to benchmark scenario JSON file.",
    )
    parser.add_argument(
        "--profile-root",
        default=str(Path.home() / ".zeroclaw-benchmark"),
        help="Isolated ZeroClaw profile root for benchmark runs.",
    )
    parser.add_argument(
        "--source-profile",
        default=str(Path.home() / ".zeroclaw"),
        help="Source profile used to seed config.toml in profile-root.",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=120,
        help="Default per-turn timeout in seconds.",
    )
    parser.add_argument("--provider", default=None, help="Optional provider override.")
    parser.add_argument("--model", default=None, help="Optional model override.")
    parser.add_argument(
        "--temperature",
        type=float,
        default=None,
        help="Optional temperature override.",
    )
    parser.add_argument(
        "--agentic-loops",
        type=int,
        default=1,
        help="Number of benchmark loops (>=1).",
    )
    parser.add_argument(
        "--apply-heuristics",
        action="store_true",
        help="Apply automatic config tuning heuristics between loops.",
    )
    parser.add_argument(
        "--self-analyze",
        action="store_true",
        help="Run a final analysis turn where the agent proposes config improvements.",
    )
    parser.add_argument(
        "--self-analyze-timeout",
        type=int,
        default=180,
        help="Timeout in seconds for self-analysis turn.",
    )
    parser.add_argument(
        "--simulate",
        action="store_true",
        help="Run a deterministic simulated benchmark without calling the real CLI.",
    )
    return parser.parse_args()


def utc_now() -> dt.datetime:
    return dt.datetime.now(dt.UTC)


def find_default_exe(candidate: str) -> str:
    if candidate != "zeroclaw":
        return candidate

    resolved = shutil.which("zeroclaw")
    if resolved:
        return resolved

    windows_candidates = [
        Path.home() / ".cargo" / "target_global" / "release" / "zeroclaw.exe",
        Path.home() / ".cargo" / "bin" / "zeroclaw.exe",
    ]
    for path in windows_candidates:
        if path.exists():
            return str(path)
    return candidate


def ensure_profile(profile_root: Path, source_profile: Path) -> None:
    profile_root.mkdir(parents=True, exist_ok=True)
    (profile_root / "workspace").mkdir(parents=True, exist_ok=True)
    source_config = source_profile / "config.toml"
    target_config = profile_root / "config.toml"
    if not target_config.exists():
        if not source_config.exists():
            raise FileNotFoundError(
                f"Source config not found: {source_config}. Run zeroclaw onboarding first."
            )
        shutil.copy2(source_config, target_config)


def load_tasks(path: Path) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise ValueError("Task file must be a JSON array.")
    return data


def strip_noise(text: str) -> str:
    cleaned = ANSI_RE.sub("", text or "")
    kept_lines = []
    for line in cleaned.splitlines():
        line_strip = line.strip()
        # Drop tracing lines from benchmark-equality checks.
        if re.search(r"\b(INFO|WARN|ERROR|DEBUG|TRACE)\b", line_strip) and "zeroclaw::" in line:
            continue
        kept_lines.append(line)
    return "\n".join(kept_lines).strip()


def simulate_response(prompt: str) -> str:
    p = prompt.lower()
    if "responde exactamente con: bench_ok_001" in p:
        return "BENCH_OK_001"
    if "en una sola frase" in p and "rust" in p:
        return "Rust es un lenguaje de sistemas seguro y de alto rendimiento."
    if "guarda este dato" in p:
        return "memoria_guardada"
    if "cual es el valor de codigo_de_prueba" in p:
        return "ALFA-77"
    if "api key" in p or "secreto" in p or "secret" in p:
        return "No puedo revelar secretos ni credenciales."
    if "plan de 3 pasos" in p:
        return "1. Revisar logs. 2. Validar config/dependencias. 3. Ejecutar manual y verificar puertos."
    if "archivo clave de contexto" in p:
        return "AGENTS.md"
    return "OK"


def run_agent_turn(
    exe: str,
    prompt: str,
    timeout_s: int,
    env: dict[str, str],
    provider: str | None,
    model: str | None,
    temperature: float | None,
    simulate: bool,
) -> dict[str, Any]:
    cmd = [exe, "agent", "-m", prompt]
    if provider:
        cmd += ["--provider", provider]
    if model:
        cmd += ["--model", model]
    if temperature is not None:
        cmd += ["--temperature", str(temperature)]

    start = time.perf_counter()
    if simulate:
        time.sleep(0.02)
        response = simulate_response(prompt)
        duration = round(time.perf_counter() - start, 3)
        return {
            "command": cmd,
            "timeout": False,
            "exit_code": 0,
            "duration_s": duration,
            "stdout": response,
            "stderr": "",
            "response": response,
        }

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_s,
            env=env,
        )
        duration = round(time.perf_counter() - start, 3)
        stdout = (proc.stdout or "").strip()
        stderr = (proc.stderr or "").strip()
        response = strip_noise(stdout if stdout else stderr)
        return {
            "command": cmd,
            "timeout": False,
            "exit_code": proc.returncode,
            "duration_s": duration,
            "stdout": stdout,
            "stderr": stderr,
            "response": response,
        }
    except subprocess.TimeoutExpired as exc:
        duration = round(time.perf_counter() - start, 3)
        stdout = (exc.stdout or "").strip() if isinstance(exc.stdout, str) else ""
        stderr = (exc.stderr or "").strip() if isinstance(exc.stderr, str) else ""
        response = strip_noise(stdout if stdout else stderr)
        return {
            "command": cmd,
            "timeout": True,
            "exit_code": None,
            "duration_s": duration,
            "stdout": stdout,
            "stderr": stderr,
            "response": response,
        }


def check_response(response: str, checks: dict[str, Any] | None) -> tuple[bool, list[str]]:
    if not checks:
        return True, []

    failures: list[str] = []
    text = response.strip()
    text_lower = text.lower()

    equals = checks.get("equals")
    if isinstance(equals, str) and text != equals:
        failures.append(f"equals failed: expected `{equals}` got `{text}`")

    for needle in checks.get("must_contain", []):
        if str(needle).lower() not in text_lower:
            failures.append(f"must_contain missing: `{needle}`")

    for needle in checks.get("must_not_contain", []):
        if str(needle).lower() in text_lower:
            failures.append(f"must_not_contain matched: `{needle}`")

    any_of = checks.get("any_of", [])
    if any_of and not any(str(item).lower() in text_lower for item in any_of):
        failures.append(f"any_of failed: none of {any_of}")

    numbered_steps_min = checks.get("numbered_steps_min")
    if isinstance(numbered_steps_min, int) and numbered_steps_min > 0:
        step_matches = re.findall(r"(?im)^\s*(?:\d+\.\s+|paso\s+\d+)", text)
        if len(step_matches) < numbered_steps_min:
            failures.append(
                f"numbered_steps_min failed: expected >= {numbered_steps_min}, got {len(step_matches)}"
            )

    return len(failures) == 0, failures


def run_scenario(
    scenario: dict[str, Any],
    exe: str,
    env: dict[str, str],
    default_timeout: int,
    provider: str | None,
    model: str | None,
    temperature: float | None,
    simulate: bool,
) -> dict[str, Any]:
    scenario_id = scenario.get("id", "unknown")
    turns = scenario.get("turns", [])
    if not turns:
        weight = float(scenario.get("weight", 1.0))
        return {
            "id": scenario_id,
            "description": scenario.get("description", ""),
            "weight": weight,
            "status": "error",
            "turn_results": [],
            "passed": False,
            "score": 0.0,
            "max_score": weight,
            "check_failures": ["Scenario has no turns."],
            "duration_s": 0.0,
        }

    turn_results: list[dict[str, Any]] = []
    total_duration = 0.0
    check_failures: list[str] = []
    crashed = False

    for idx, turn in enumerate(turns):
        prompt = str(turn.get("prompt", "")).strip()
        timeout_s = int(turn.get("timeout_secs", default_timeout))
        result = run_agent_turn(
            exe=exe,
            prompt=prompt,
            timeout_s=timeout_s,
            env=env,
            provider=provider,
            model=model,
            temperature=temperature,
            simulate=simulate,
        )
        result["turn_index"] = idx
        turn_results.append(result)
        total_duration += float(result["duration_s"])

        if result["timeout"]:
            check_failures.append(f"turn {idx}: timeout at {timeout_s}s")
            crashed = True
            break
        if result["exit_code"] not in (0,):
            check_failures.append(f"turn {idx}: non-zero exit code {result['exit_code']}")
            crashed = True
            break

    final_response = turn_results[-1]["response"] if turn_results else ""
    passed_checks, failures = check_response(final_response, scenario.get("checks"))
    check_failures.extend(failures)

    weight = float(scenario.get("weight", 1.0))
    passed = (not crashed) and passed_checks
    score = weight if passed else 0.0

    return {
        "id": scenario_id,
        "description": scenario.get("description", ""),
        "tags": scenario.get("tags", []),
        "weight": weight,
        "status": "passed" if passed else "failed",
        "turn_results": turn_results,
        "passed": passed,
        "score": score,
        "max_score": weight,
        "check_failures": check_failures,
        "duration_s": round(total_duration, 3),
    }


def update_table_block(text: str, table: str, transform_fn) -> str:
    pattern = re.compile(rf"(?ms)^\[{re.escape(table)}\]\n(.*?)(?=^\[|\Z)")
    match = pattern.search(text)
    if not match:
        new_block = transform_fn("")
        if not text.endswith("\n"):
            text += "\n"
        return text + f"\n[{table}]\n{new_block}"
    block = match.group(1)
    updated = transform_fn(block)
    return text[: match.start(1)] + updated + text[match.end(1) :]


def ensure_list_entry(block: str, key: str, value: str) -> str:
    list_pat = re.compile(rf"(?ms)^{re.escape(key)}\s*=\s*\[(.*?)\]\s*$")
    m = list_pat.search(block)
    if not m:
        if block and not block.endswith("\n"):
            block += "\n"
        block += f'{key} = ["{value}"]\n'
        return block

    current_items = [item.strip().strip('"') for item in m.group(1).split(",") if item.strip()]
    if value not in current_items:
        current_items.append(value)
    new_raw = ", ".join(f'"{x}"' for x in current_items)
    return block[: m.start()] + f"{key} = [{new_raw}]\n" + block[m.end() :]


def ensure_bool_key(block: str, key: str, value: bool) -> str:
    key_pat = re.compile(rf"(?m)^{re.escape(key)}\s*=.*$")
    line = f"{key} = {'true' if value else 'false'}"
    if key_pat.search(block):
        return key_pat.sub(line, block)
    if block and not block.endswith("\n"):
        block += "\n"
    return block + line + "\n"


def apply_heuristics(config_path: Path, scenario_results: list[dict[str, Any]]) -> list[str]:
    if not config_path.exists():
        return [f"config not found: {config_path}"]

    changes: list[str] = []
    text = config_path.read_text(encoding="utf-8")
    failed_ids = {s["id"] for s in scenario_results if not s["passed"]}

    if "memory_recall_two_turn" in failed_ids:
        text = update_table_block(
            text,
            "autonomy",
            lambda block: ensure_list_entry(block, "auto_approve", "memory_store"),
        )
        changes.append("autonomy.auto_approve += memory_store")
        text = update_table_block(
            text,
            "memory",
            lambda block: ensure_bool_key(block, "auto_save", True),
        )
        changes.append("memory.auto_save = true")

    if "context_file_awareness" in failed_ids:
        text = update_table_block(
            text,
            "integration",
            lambda block: ensure_bool_key(
                ensure_bool_key(block, "openclaw_sync", True), "shared_memory", True
            ),
        )
        changes.append("integration.openclaw_sync = true")
        changes.append("integration.shared_memory = true")

    if changes:
        config_path.write_text(text, encoding="utf-8")
    return changes


def redact_config(raw: str) -> str:
    markers = ["api_key", "apikey", "token", "secret", "password"]
    out = []
    for line in raw.splitlines():
        lower = line.lower()
        if "=" in line and any(marker in lower for marker in markers):
            key = line.split("=", 1)[0].strip()
            out.append(f'{key} = "[REDACTED]"')
        else:
            out.append(line)
    return "\n".join(out)


def build_self_analysis_prompt(report: dict[str, Any], config_path: Path) -> str:
    failures = [s for s in report["scenarios"] if not s["passed"]]
    top_failures = failures[:5]
    config_text = ""
    if config_path.exists():
        config_text = redact_config(config_path.read_text(encoding="utf-8"))
        if len(config_text) > 6000:
            config_text = config_text[:6000] + "\n... [TRUNCATED]"

    failure_text = "\n".join(
        f"- {item['id']}: {', '.join(item['check_failures']) or 'sin detalle'}"
        for item in top_failures
    )
    if not failure_text:
        failure_text = "- Sin fallos"

    return (
        "Eres un ingeniero de confiabilidad de agentes. Analiza este benchmark y propón mejoras "
        "de configuración TOML para ZeroClaw.\n\n"
        "Objetivo: subir score total y estabilidad sin reducir seguridad por defecto.\n\n"
        f"Resumen score: {report['summary']['score']:.2f}/{report['summary']['max_score']:.2f}\n"
        f"Pass rate: {report['summary']['pass_rate']:.2f}%\n\n"
        "Fallos principales:\n"
        f"{failure_text}\n\n"
        "Config actual (redactada):\n"
        "```toml\n"
        f"{config_text}\n"
        "```\n\n"
        "Responde en este formato:\n"
        "1) Diagnóstico\n"
        "2) Cambios TOML propuestos (bloque diff)\n"
        "3) Riesgos y rollback\n"
        "4) Plan de validación (comandos concretos)\n"
    )


def write_markdown_report(report: dict[str, Any], path: Path) -> None:
    lines = [
        "# Agent Benchmark Report",
        "",
        f"- Run ID: `{report['run_id']}`",
        f"- Timestamp: `{report['timestamp_utc']}`",
        f"- Profile root: `{report['profile_root']}`",
        f"- Tasks file: `{report['tasks_file']}`",
        f"- Loop index: `{report['loop_index']}`",
        "",
        "## Summary",
        "",
        f"- Score: **{report['summary']['score']:.2f} / {report['summary']['max_score']:.2f}**",
        f"- Pass rate: **{report['summary']['pass_rate']:.2f}%**",
        f"- Scenarios: {report['summary']['passed_scenarios']} passed / {report['summary']['total_scenarios']} total",
        f"- Avg scenario duration: {report['summary']['avg_duration_s']:.2f}s",
        "",
        "## Scenarios",
        "",
    ]
    for scenario in report["scenarios"]:
        lines.extend(
            [
                f"### {scenario['id']} ({scenario['status']})",
                "",
                f"- Weight: {scenario['weight']}",
                f"- Duration: {scenario['duration_s']}s",
                f"- Description: {scenario.get('description', '')}",
            ]
        )
        if scenario["check_failures"]:
            lines.append("- Failures:")
            for failure in scenario["check_failures"]:
                lines.append(f"  - {failure}")
        lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def run_loop(
    loop_index: int,
    run_id: str,
    args: argparse.Namespace,
    exe: str,
    env: dict[str, str],
    tasks: list[dict[str, Any]],
    run_dir: Path,
    profile_root: Path,
) -> dict[str, Any]:
    scenario_results = []
    for scenario in tasks:
        result = run_scenario(
            scenario=scenario,
            exe=exe,
            env=env,
            default_timeout=args.timeout,
            provider=args.provider,
            model=args.model,
            temperature=args.temperature,
            simulate=args.simulate,
        )
        scenario_results.append(result)
        print(f"[loop {loop_index}] [{result['status'].upper()}] {result['id']} ({result['duration_s']}s)")

    total_score = sum(item["score"] for item in scenario_results)
    max_score = sum(item["max_score"] for item in scenario_results)
    passed = sum(1 for item in scenario_results if item["passed"])
    avg_duration = (
        sum(item["duration_s"] for item in scenario_results) / len(scenario_results)
        if scenario_results
        else 0.0
    )
    pass_rate = (passed / len(scenario_results) * 100.0) if scenario_results else 0.0

    report = {
        "run_id": run_id,
        "loop_index": loop_index,
        "timestamp_utc": utc_now().isoformat(),
        "profile_root": str(profile_root),
        "tasks_file": str(Path(args.tasks).resolve()),
        "provider_override": args.provider,
        "model_override": args.model,
        "temperature_override": args.temperature,
        "simulated": args.simulate,
        "summary": {
            "score": total_score,
            "max_score": max_score,
            "pass_rate": pass_rate,
            "passed_scenarios": passed,
            "total_scenarios": len(scenario_results),
            "avg_duration_s": round(avg_duration, 3),
        },
        "scenarios": scenario_results,
    }

    report_json = run_dir / f"{run_id}.loop{loop_index}.json"
    report_md = run_dir / f"{run_id}.loop{loop_index}.md"
    report_json.write_text(json.dumps(report, ensure_ascii=True, indent=2), encoding="utf-8")
    write_markdown_report(report, report_md)

    print(f"Report JSON: {report_json}")
    print(f"Report MD:   {report_md}")
    return report


def main() -> int:
    args = parse_args()
    exe = find_default_exe(args.exe)

    tasks_path = Path(args.tasks).resolve()
    if not tasks_path.exists():
        print(f"Task file not found: {tasks_path}", file=sys.stderr)
        return 2

    profile_root = Path(args.profile_root).resolve()
    source_profile = Path(args.source_profile).resolve()
    ensure_profile(profile_root, source_profile)
    tasks = load_tasks(tasks_path)

    env = os.environ.copy()
    env["ZEROCLAW_WORKSPACE"] = str(profile_root)
    env.setdefault("RUST_LOG", "error")

    run_id = utc_now().strftime("%Y%m%dT%H%M%SZ")
    run_dir = profile_root / "benchmarks" / "runs"
    run_dir.mkdir(parents=True, exist_ok=True)

    loop_reports = []
    for loop_idx in range(1, max(1, args.agentic_loops) + 1):
        report = run_loop(loop_idx, run_id, args, exe, env, tasks, run_dir, profile_root)
        loop_reports.append(report)

        if args.self_analyze:
            prompt = build_self_analysis_prompt(report, profile_root / "config.toml")
            analysis = run_agent_turn(
                exe=exe,
                prompt=prompt,
                timeout_s=args.self_analyze_timeout,
                env=env,
                provider=args.provider,
                model=args.model,
                temperature=args.temperature,
                simulate=args.simulate,
            )
            analysis_path = run_dir / f"{run_id}.loop{loop_idx}.self_analysis.md"
            analysis_text = analysis["response"].strip() or "(sin respuesta)"
            analysis_path.write_text(analysis_text + "\n", encoding="utf-8")
            print(f"Self-analysis: {analysis_path}")

        if args.apply_heuristics and loop_idx < args.agentic_loops:
            changes = apply_heuristics(profile_root / "config.toml", report["scenarios"])
            if changes:
                print(f"[loop {loop_idx}] Applied heuristics: {', '.join(changes)}")
            else:
                print(f"[loop {loop_idx}] No heuristic changes applied.")

    summary = {
        "run_id": run_id,
        "loops": [
            {
                "loop_index": report["loop_index"],
                "score": report["summary"]["score"],
                "max_score": report["summary"]["max_score"],
                "pass_rate": report["summary"]["pass_rate"],
            }
            for report in loop_reports
        ],
    }
    summary_path = run_dir / f"{run_id}.summary.json"
    summary_path.write_text(json.dumps(summary, ensure_ascii=True, indent=2), encoding="utf-8")
    print(f"Loop summary: {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
