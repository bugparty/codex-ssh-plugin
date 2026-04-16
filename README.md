# SSH Remote Workbench

This plugin is built around a simple remote-dev workflow:

- mount the remote tree locally with SSHFS
- edit files through the mounted directory
- apply patch-style edits with the bundled Rust patch tool
- run remote commands with the bundled SSH wrapper

The plugin includes these building blocks:

- `bin/rexec`: a short entrypoint for remote command execution over SSH.
- `bin/apply_patch`: a prebuilt standalone patch binary for mounted remote files.
- `scripts/remote_exec.py`: the underlying wrapper that forwards argv, cwd, env, and TTY settings over SSH.
- `tools/rust-apply-patch/`: a standalone Rust apply_patch prototype adapted from the open-source `openai/codex` repository.

## Included tools

- `plugins/ssh-remote-workbench/bin/rexec`
- `plugins/ssh-remote-workbench/bin/apply_patch`
- `plugins/ssh-remote-workbench/scripts/remote_exec.py`
- `plugins/ssh-remote-workbench/tools/rust-apply-patch/`
- `plugins/ssh-remote-workbench/skills/ssh-remote-workbench/SKILL.md`
- `plugins/ssh-remote-workbench/config/defaults.json`

## Initialization

Before using the plugin workflow, make sure the bundled `apply_patch` binary is
available for your machine.

Rebuild and refresh `bin/apply_patch` from source:

```bash
cargo build --release \
  --manifest-path plugins/ssh-remote-workbench/tools/rust-apply-patch/Cargo.toml

cp plugins/ssh-remote-workbench/tools/rust-apply-patch/target/release/rust-apply-patch \
  plugins/ssh-remote-workbench/bin/apply_patch

chmod +x plugins/ssh-remote-workbench/bin/apply_patch
```

Then verify both entrypoints:

```bash
plugins/ssh-remote-workbench/bin/apply_patch --help
plugins/ssh-remote-workbench/bin/rexec --dry-run -- pwd
```

## Default remote context

The plugin now includes a default config at
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

## Recommended workflow

### 1. Mount the remote tree with SSHFS

Mount the remote root onto the configured local mount root:

```bash
sshfs bowmanhan@192.168.34.111:/home/bowmanhan/Code \
  /Users/bowmanhan/qoe-boxr-research/incoming/sshfs_mount_test
```

After this, the mounted tree behaves like a local directory for browsing and editing.

### 2. Edit mounted files locally

Work inside the mounted tree:

```bash
cd /Users/bowmanhan/qoe-boxr-research/incoming/sshfs_mount_test
```

If you want Codex-style patch edits, run the bundled patch binary against files
in the mounted tree:

```bash
plugins/ssh-remote-workbench/bin/apply_patch \
  "*** Begin Patch
*** Update File: /Users/bowmanhan/qoe-boxr-research/incoming/sshfs_mount_test/hello.txt
@@
-old
+new
*** End Patch"
```

Because the file lives in the SSHFS mount, the patch applies to the remote file.

If `bin/apply_patch` is missing or incompatible with the current machine,
rebuild it from `plugins/ssh-remote-workbench/tools/rust-apply-patch/` using
the initialization steps above.

### 3. Run remote commands with `rexec`

Run a remote command:

```bash
plugins/ssh-remote-workbench/bin/rexec \
  -- ls -la
```

Preview the resolved host/cwd without executing:

```bash
plugins/ssh-remote-workbench/bin/rexec --dry-run -- pwd
```

If your current directory is inside the SSHFS mount, the wrapper will map it to
the corresponding remote directory automatically.

If you need the underlying script directly, it remains available at
`plugins/ssh-remote-workbench/scripts/remote_exec.py`.

## Security notes

- Start with read-only tools plus a strict host allowlist.
- Require explicit opt-in for remote writes.
- Add per-command timeouts and output size limits.
- Prefer key-based auth from the user's existing SSH agent or config.
- Avoid password prompts in automated SSH command flows.
