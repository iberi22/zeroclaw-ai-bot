#!/usr/bin/env python3
"""
Clone OpenClaw workspace into isolated ZeroClaw profile root, without mutating source.

This is intended for behavior/bootstrap parity while preserving workspace isolation.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Clone OpenClaw workspace into isolated profile.")
    parser.add_argument(
        "--openclaw-config",
        default="",
        help="Path to openclaw.json. If empty, auto-detect ~/.clawdbot/openclaw.json or ~/.openclaw/openclaw.json",
    )
    parser.add_argument(
        "--target-root",
        default=str(Path.home() / ".zeroclaw-openclaw-bridge"),
        help="Target ZeroClaw profile root (contains config.toml + workspace/).",
    )
    parser.add_argument(
        "--source-profile",
        default=str(Path.home() / ".zeroclaw"),
        help="Source ZeroClaw profile to seed config.toml if missing.",
    )
    parser.add_argument("--dry-run", action="store_true", help="Preview only.")
    return parser.parse_args()


def detect_openclaw_config(user_supplied: str) -> Path:
    if user_supplied:
        path = Path(user_supplied).expanduser().resolve()
        if not path.exists():
            raise FileNotFoundError(f"OpenClaw config not found: {path}")
        return path

    candidates = [
        Path.home() / ".clawdbot" / "openclaw.json",
        Path.home() / ".openclaw" / "openclaw.json",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate
    raise FileNotFoundError("openclaw.json not found in ~/.clawdbot or ~/.openclaw")


def load_openclaw_workspace(config_path: Path) -> Path:
    data = json.loads(config_path.read_text(encoding="utf-8"))
    workspace = (
        data.get("agents", {})
        .get("defaults", {})
        .get("workspace", "")
        .strip()
    )
    if not workspace:
        raise ValueError(f"No agents.defaults.workspace found in {config_path}")
    ws_path = Path(workspace).expanduser().resolve()
    if not ws_path.exists():
        raise FileNotFoundError(f"OpenClaw workspace path not found: {ws_path}")
    return ws_path


def ensure_target_profile(target_root: Path, source_profile: Path, dry_run: bool) -> None:
    target_ws = target_root / "workspace"
    source_cfg = source_profile / "config.toml"
    target_cfg = target_root / "config.toml"

    if dry_run:
        return

    target_root.mkdir(parents=True, exist_ok=True)
    target_ws.mkdir(parents=True, exist_ok=True)
    if not target_cfg.exists():
        if not source_cfg.exists():
            raise FileNotFoundError(
                f"Source config.toml not found: {source_cfg}. Run ZeroClaw onboarding first."
            )
        shutil.copy2(source_cfg, target_cfg)


def copy_workspace(src: Path, dst: Path, dry_run: bool) -> tuple[int, int]:
    ignore_dirs = {".git", "node_modules", "__pycache__", ".pytest_cache", "target"}
    ignore_files = {".DS_Store"}
    copied = 0
    skipped = 0

    for root, dirs, files in os.walk(src):
        root_path = Path(root)
        rel = root_path.relative_to(src)

        dirs[:] = [d for d in dirs if d not in ignore_dirs]
        target_dir = dst / rel
        if not dry_run:
            target_dir.mkdir(parents=True, exist_ok=True)

        for filename in files:
            if filename in ignore_files:
                skipped += 1
                continue
            src_file = root_path / filename
            dst_file = target_dir / filename
            if dry_run:
                copied += 1
                continue
            shutil.copy2(src_file, dst_file)
            copied += 1

    return copied, skipped


def ensure_integration_config(target_cfg: Path, dry_run: bool) -> None:
    text = target_cfg.read_text(encoding="utf-8") if target_cfg.exists() else ""
    if "[integration]" in text:
        if "openclaw_sync" not in text:
            text += "\nopenclaw_sync = true\n"
        if "shared_memory" not in text:
            text += "shared_memory = true\n"
    else:
        if text and not text.endswith("\n"):
            text += "\n"
        text += "\n[integration]\nopenclaw_sync = true\nshared_memory = true\n"
    if not dry_run:
        target_cfg.write_text(text, encoding="utf-8")


def main() -> int:
    args = parse_args()
    openclaw_config = detect_openclaw_config(args.openclaw_config)
    openclaw_workspace = load_openclaw_workspace(openclaw_config)
    target_root = Path(args.target_root).expanduser().resolve()
    target_ws = target_root / "workspace"
    source_profile = Path(args.source_profile).expanduser().resolve()

    ensure_target_profile(target_root, source_profile, args.dry_run)

    copied, skipped = copy_workspace(openclaw_workspace, target_ws, args.dry_run)
    ensure_integration_config(target_root / "config.toml", args.dry_run)

    mode = "DRY RUN" if args.dry_run else "DONE"
    print(f"[{mode}] OpenClaw config:   {openclaw_config}")
    print(f"[{mode}] OpenClaw workspace:{openclaw_workspace}")
    print(f"[{mode}] Target root:      {target_root}")
    print(f"[{mode}] Target workspace: {target_ws}")
    print(f"[{mode}] Files copied:     {copied}")
    print(f"[{mode}] Files skipped:    {skipped}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
