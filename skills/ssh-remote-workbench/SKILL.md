---
name: ssh-remote-workbench
description: Use the plugin-bundled remote_exec wrapper, defaults config, and apply_patch tool to work against remote machines over SSH, while keeping edits narrow and reviewable.
---

# SSH Remote Workbench

Use this skill when the user wants Codex to work against a remote machine over SSH.

## Bundled plugin tools

- Remote command wrapper:
  `plugins/ssh-remote-workbench/scripts/remote_exec.py`
- Short entrypoint:
  `plugins/ssh-remote-workbench/bin/rexec`
- Prebuilt patch binary:
  `plugins/ssh-remote-workbench/bin/apply_patch`
- Default config:
  `plugins/ssh-remote-workbench/config/defaults.json`
- Standalone patch tool:
  `plugins/ssh-remote-workbench/tools/rust-apply-patch/`

## Workflow

1. Prefer the repository's SSHFS mount workflow first: mount the remote tree locally, then operate on mounted files.
2. Prefer the plugin default config for host and path mapping before asking for repeated remote path details.
3. Before relying on `bin/apply_patch`, make sure it has been built or refreshed from `tools/rust-apply-patch/` for the current machine.
4. Use the bundled `bin/apply_patch` binary for patch-style edits against files inside the mounted tree.
5. Use `bin/rexec` for remote command execution instead of raw `ssh ... '...'` one-liners.
6. Prefer named hosts from the user's SSH config or plugin config over ad hoc host strings.
7. Start with read-only inspection tasks.
8. Use bounded commands with clear timeouts.
9. When running from a mounted subdirectory, rely on `remote_exec.py` automatic local-to-remote cwd mapping.
10. Escalate to remote writes only when the user clearly asked for them.
11. Summarize remote effects before taking the next action.

## Guardrails

- Avoid unrestricted interactive shell sessions when a structured tool call can do the job.
- Refuse obviously destructive commands unless the user explicitly requests them.
- Keep remote changes narrow and reviewable.
- Preserve stderr, exit code, and timeout information in every command result.
- Prefer `--dry-run` on `bin/rexec` or `remote_exec.py` when validating quoting or cwd/env forwarding.

## Good first tasks

- Check Python, CUDA, or package versions on a remote box.
- Read logs from a known path.
- Compare local and remote config files.
- Apply a patch to an SSHFS-mounted remote worktree with `bin/apply_patch`.
- Stage a patch locally, then upload only the intended file.
