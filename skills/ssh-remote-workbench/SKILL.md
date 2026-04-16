---
name: ssh-remote-workbench
description: Use the plugin-bundled remote_exec wrapper and defaults config to work against remote machines over SSH while editing files through an SSHFS-mounted tree.
---

# SSH Remote Workbench

Use this skill when the user wants Codex to work against a remote machine over SSH.

## Bundled plugin tools

- Remote command wrapper:
  `plugins/ssh-remote-workbench/scripts/remote_exec.py`
- Short entrypoint:
  `plugins/ssh-remote-workbench/bin/rexec`
- Default config:
  `plugins/ssh-remote-workbench/config/defaults.json`

## Workflow

1. Prefer the repository's SSHFS mount workflow first: mount the remote tree locally, then operate on mounted files.
2. Prefer the plugin default config for host and path mapping before asking for repeated remote path details.
3. Use Codex's native local editing tools directly on files inside the mounted tree.
4. Default all command execution to the remote machine through `bin/rexec`, including Python scripts, tests, builds, and other repo commands.
5. Do not run commands locally unless the user explicitly asks for local execution.
6. Use `bin/rexec` for remote command execution instead of raw `ssh ... '...'` one-liners.
7. Prefer named hosts from the user's SSH config or plugin config over ad hoc host strings.
8. Start with read-only inspection tasks.
9. Use bounded commands with clear timeouts.
10. When running from a mounted subdirectory, rely on `remote_exec.py` automatic local-to-remote cwd mapping.
11. Escalate to remote writes only when the user clearly asked for them.
12. Summarize remote effects before taking the next action.

## Guardrails

- Avoid unrestricted interactive shell sessions when a structured tool call can do the job.
- Refuse obviously destructive commands unless the user explicitly requests them.
- Keep remote changes narrow and reviewable.
- Preserve stderr, exit code, and timeout information in every command result.
- Prefer `--dry-run` on `bin/rexec` or `remote_exec.py` when validating quoting or cwd/env forwarding.
- Unless the user clearly says otherwise, interpret command execution requests as remote execution requests.

## Good first tasks

- Check Python, CUDA, or package versions on a remote box.
- Read logs from a known path.
- Compare local and remote config files.
- Stage a patch locally, then upload only the intended file.
