# rust-apply-patch

Standalone Rust prototype for Codex-style `apply_patch`.

This tool was derived from the structure and patch grammar used by the open-source
[`openai/codex`](https://github.com/openai/codex) repository, but simplified so it
does not depend on Codex runtime crates like `codex-exec-server`.

## What it supports

- `*** Add File:`
- `*** Delete File:`
- `*** Update File:`
- optional `*** Move to:`
- `@@` context markers
- `*** End of File`

## What it does not include

- Codex sandbox integration
- shell/heredoc command detection
- patch approval flow
- unified diff preview generation

## Usage

```bash
cargo run --manifest-path tools/rust-apply-patch/Cargo.toml -- \
  "*** Begin Patch
*** Add File: hello.txt
+hello
*** End Patch"
```

Or:

```bash
cat patch.txt | cargo run --manifest-path tools/rust-apply-patch/Cargo.toml
```

