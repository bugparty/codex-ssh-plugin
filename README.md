# Codex SSH Remote Workbench

This codex plugin is built around a simple remote-dev workflow:

- mount a remote directory into the current local folder with SSHFS
- edit files locally through the mounted directory
- run remote commands for that mounted code over SSH

The plugin includes these building blocks:

- `bin/rexec`: a short entrypoint for remote command execution over SSH
- `scripts/remote_exec.py`: the underlying wrapper that forwards argv, cwd, env, and TTY settings over SSH

## Included tools

- `plugins/ssh-remote-workbench/bin/rexec`
- `plugins/ssh-remote-workbench/scripts/remote_exec.py`
- `plugins/ssh-remote-workbench/skills/ssh-remote-workbench/SKILL.md`
- `plugins/ssh-remote-workbench/config/defaults.json`

## Default remote context

The plugin includes a default config at
`plugins/ssh-remote-workbench/config/defaults.json`.

It currently defines:

- default host: `bowmanhan@192.168.34.111`
- remote root: `/home/bowmanhan/Code`
- local SSHFS mount root:
  `/Users/bowmanhan/qoe-boxr-research/incoming/sshfs_mount_test`

`bin/rexec` and `remote_exec.py` use this config to:

- infer `--host` when omitted
- infer `--cwd` from the current local directory when it is inside the mounted tree
- fall back to the configured remote root when no local path mapping matches

## Execution rule

When using this plugin, command execution should default to the remote machine.
That includes Python scripts, tests, builds, and other repo commands. Prefer
`bin/rexec` unless the user explicitly asks for local execution.

## Recommended workflow

### 1. Mount a remote directory into the current local folder

From the local folder where you want the remote files to appear, run:

```bash
sshfs bowmanhan@192.168.34.111:/home/bowmanhan/Code .
```

If you prefer the plugin's default mount location, run:

```bash
sshfs bowmanhan@192.168.34.111:/home/bowmanhan/Code \
  /Users/bowmanhan/qoe-boxr-research/incoming/sshfs_mount_test
```

### 2. Edit the mounted code locally

Once the remote directory is mounted, treat it like a normal local codebase.
Use Codex's native local editing tools directly on files in that mounted tree.

### 3. Run remote commands for the mounted code

Run a remote command:

```bash
plugins/ssh-remote-workbench/bin/rexec -- ls -la
```

As a rule, prefer:

```bash
plugins/ssh-remote-workbench/bin/rexec -- python script.py
plugins/ssh-remote-workbench/bin/rexec -- pytest
plugins/ssh-remote-workbench/bin/rexec -- cargo test
```

Do not default to local execution for those commands unless the user clearly
asks for local execution.

Preview the resolved host/cwd without executing:

```bash
plugins/ssh-remote-workbench/bin/rexec --dry-run -- pwd
```

If your current directory is inside the mounted tree, the wrapper will map it to
the corresponding remote directory automatically.

If you need the underlying script directly, it remains available at
`plugins/ssh-remote-workbench/scripts/remote_exec.py`.

The standalone Rust patch tool now lives outside the plugin at:

`/Users/bowmanhan/codes/rust-apply-patch`

## Security notes

- Start with read-only tools plus a strict host allowlist
- Require explicit opt-in for remote writes
- Add per-command timeouts and output size limits
- Prefer key-based auth from the user's existing SSH agent or config
- Avoid password prompts in automated SSH command flows
