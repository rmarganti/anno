# Pi anno review package

This directory is the repo-local Pi package for anno-powered review workflows.
It is intentionally separate from the Rust crate root so Pi users can install a focused package with a local path, while the repository itself remains a normal Cargo project.

## Chosen layout

```text
pi/
  anno-review/
    README.md
    package.json
    index.ts
```

Why this layout:

- `pi/anno-review/` is a self-contained package root for `pi install ./pi/anno-review`.
- `package.json` keeps Pi package metadata and any future npm dependencies out of the repository root.
- `index.ts` is the extension entrypoint declared through `package.json -> pi.extensions`.
- Future extension-only helper modules can live beside `index.ts` without affecting the Rust build.

## Installation paths this package is designed to support

### 1. Local package install with Pi

From the repository root:

```bash
pi install ./pi/anno-review
```

This is the preferred path because it lets Pi load the package as a package root, honor `package.json`, and install future npm dependencies if the extension grows beyond a single file.

### 2. Copy or symlink into Pi's extension directories

Global install:

```bash
mkdir -p ~/.pi/agent/extensions/anno-review
ln -sf "$(pwd)/pi/anno-review/index.ts" ~/.pi/agent/extensions/anno-review/index.ts
```

Project-local install:

```bash
mkdir -p .pi/extensions/anno-review
ln -sf "$(pwd)/pi/anno-review/index.ts" .pi/extensions/anno-review/index.ts
```

Copying the file instead of symlinking also works, but symlinks are better during development because `/reload` can pick up edits from an auto-discovered extension location.

## Naming decisions

These names are reserved for the implementation tracked by the follow-up beads:

- Package name: `anno-pi-review`
- Slash command: `/anno-review`
- Planned custom tool name: `anno_review`

The command should be the primary user entrypoint.
The tool name is reserved for a future guarded interactive workflow so the LLM can invoke anno only when Pi is running with a TUI and direct terminal handoff is possible.

## Implementation plan for the follow-up extension

The follow-up implementation should use the direct interactive-subprocess pattern documented in Pi's extension docs and examples:

1. Expose `/anno-review` as the human-facing command.
2. For interactive TUI sessions, use `ctx.ui.custom()` to access `tui.stop()` / `tui.start()`.
3. Spawn `anno` with inherited stdio so it owns the terminal directly while Pi is suspended.
4. Pass `--export-format json --output-file <temp-output>` so Pi can parse review results reliably after anno exits.
5. Resume Pi's TUI, parse the JSON output, and surface the result back to the user and/or model.

## File and temp-data strategy

The implementation should support two inputs:

- **Existing file review**: user supplies a path to an on-disk file.
- **Generated content review**: the extension writes supplied content to a temp file before launching anno.

Planned temp-file flow:

1. Resolve any user-supplied path against `ctx.cwd`.
2. For generated content, create a temp review file under the system temp directory.
3. Create a second temp file for anno's `--output-file` JSON export.
4. Run anno against the resolved real file or generated temp file.
5. In a `finally` block, remove temp files created by the extension.
6. If anno fails before producing output, return a clear error instead of pretending the review succeeded.

## Working-directory-relative path behavior

User-visible path handling should be based on `ctx.cwd`:

- Relative paths passed to `/anno-review <path>` should resolve via `path.resolve(ctx.cwd, path)`.
- The extension should invoke `anno` with the resolved absolute path for reliability.
- Titles should default from the reviewed file basename, but allow explicit override.
- Generated temp files should preserve a useful extension based on the requested syntax or source filename when possible.

## Fallback expectations

The future implementation should fail clearly in these cases:

- `anno` is not on `PATH`
- Pi is running without a TUI / without `ctx.hasUI`
- the command/tool is asked to review a missing file
- anno exits unsuccessfully or emits invalid JSON

In those cases the extension should explain why direct anno handoff is unavailable so agents can fall back to a normal in-chat review or the older tmux-based skill.
