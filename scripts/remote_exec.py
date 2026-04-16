#!/usr/bin/env python3
"""Run a remote command over SSH with explicit argv, cwd, env, and TTY options."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
from typing import Any
from typing import Sequence


DEFAULT_HOST_ENV_VAR = "REMOTE_EXEC_HOST"
CONFIG_ENV_VAR = "REMOTE_EXEC_CONFIG"


def default_config_path() -> Path:
    return Path(__file__).resolve().parent.parent / "config" / "defaults.json"


def load_config() -> dict[str, Any]:
    config_path = Path(os.environ.get(CONFIG_ENV_VAR, default_config_path()))
    if not config_path.exists():
        return {}
    with config_path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise SystemExit(f"Invalid config format in {config_path}")
    return data


def resolve_remote_cwd(
    explicit_cwd: str | None,
    config: dict[str, Any],
    local_cwd: Path,
) -> str | None:
    if explicit_cwd:
        return explicit_cwd

    for mapping in config.get("path_mappings", []):
        if not isinstance(mapping, dict):
            continue
        local_prefix = mapping.get("local_prefix")
        remote_prefix = mapping.get("remote_prefix")
        if not isinstance(local_prefix, str) or not isinstance(remote_prefix, str):
            continue

        local_prefix_path = Path(local_prefix).resolve()
        try:
            relative = local_cwd.resolve().relative_to(local_prefix_path)
        except ValueError:
            continue

        remote_path = Path(remote_prefix)
        if str(relative) == ".":
            return str(remote_path)
        return str(remote_path / relative)

    remote_root = config.get("remote_root")
    if isinstance(remote_root, str) and remote_root:
        return remote_root

    return None


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    config = load_config()
    parser = argparse.ArgumentParser(
        description=(
            "Forward a command to a remote host over SSH. "
            "Use -- to separate wrapper options from the remote argv."
        )
    )
    parser.add_argument(
        "--host",
        default=os.environ.get(DEFAULT_HOST_ENV_VAR) or config.get("default_host"),
        help=f"Remote SSH host. Defaults to ${DEFAULT_HOST_ENV_VAR} if set.",
    )
    parser.add_argument(
        "--cwd",
        help="Remote working directory to cd into before exec'ing the command.",
    )
    parser.add_argument(
        "--env",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="Environment variable to export on the remote side. Repeatable.",
    )
    parser.add_argument(
        "--tty",
        action="store_true",
        help="Request a remote TTY with ssh -tt.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the generated SSH command instead of executing it.",
    )
    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help="Remote argv. Prefix with --, for example: remote_exec.py -- ls -la",
    )
    args = parser.parse_args(argv)
    args._config = config

    if not args.host:
        parser.error(
            f"--host is required unless ${DEFAULT_HOST_ENV_VAR} is set."
        )

    if args.command and args.command[0] == "--":
        args.command = args.command[1:]

    if not args.command:
        parser.error("missing remote command after --")

    for item in args.env:
        if "=" not in item:
            parser.error(f"invalid --env entry {item!r}; expected KEY=VALUE")
        key, _ = item.split("=", 1)
        if not key:
            parser.error(f"invalid --env entry {item!r}; empty key")

    return args


def shell_quote(value: str) -> str:
    return shlex.quote(value)


def build_remote_script(command: Sequence[str], cwd: str | None, env_items: Sequence[str]) -> str:
    parts: list[str] = ["set -e"]

    if cwd:
        parts.append(f"cd {shell_quote(cwd)}")

    if env_items:
        exports = []
        for item in env_items:
            key, value = item.split("=", 1)
            exports.append(f"{key}={shell_quote(value)}")
        parts.append("export " + " ".join(exports))

    parts.append("exec " + shlex.join(command))
    return "; ".join(parts)


def build_ssh_argv(host: str, remote_script: str, tty: bool) -> list[str]:
    ssh_argv = ["ssh"]
    if tty:
        ssh_argv.append("-tt")
    ssh_argv.append(host)
    ssh_argv.append(shlex.join(["bash", "-lc", remote_script]))
    return ssh_argv


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    remote_cwd = resolve_remote_cwd(
        args.cwd,
        args._config,
        Path.cwd(),
    )
    remote_script = build_remote_script(args.command, remote_cwd, args.env)
    ssh_argv = build_ssh_argv(args.host, remote_script, args.tty)

    if args.dry_run:
        print(shlex.join(ssh_argv))
        print(f"# resolved host: {args.host}")
        print(f"# resolved cwd: {remote_cwd}")
        print("# remote script")
        print(remote_script)
        return 0

    completed = subprocess.run(ssh_argv)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
