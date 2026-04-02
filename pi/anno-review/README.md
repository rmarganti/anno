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

## Prerequisites

Before using this extension:

- Install `anno` and make sure the `anno` binary is available on `PATH`.
- Run Pi in its interactive TUI. The extension needs terminal control so it can temporarily stop Pi, hand the terminal to anno, then restart Pi.
- Keep this repository available locally if you want to install directly from `./pi/anno-review` or symlink back to the in-repo `index.ts`.

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

## Exposed entrypoints

- Package name: `anno-pi-review`
- Slash command: `/anno-review`
- Custom tool: `anno_review`

The slash command is the primary human-facing entrypoint.
The tool is intentionally guarded by its description and behavior for interactive use only.

## Implemented behavior

The extension now uses Pi's direct interactive-subprocess pattern:

1. `ctx.ui.custom()` suspends Pi's TUI with `tui.stop()`.
2. `anno` is launched with inherited stdio so it owns the terminal directly.
3. The extension passes `--export-format json --output-file <temp-output>`.
4. After anno exits, Pi's TUI is restored with `tui.start()`.
5. The exported JSON is parsed and surfaced back to Pi.

## Slash command usage

Review an existing file:

```bash
/anno-review path/to/file.md
/anno-review docs/api.md --syntax markdown
/anno-review notes.txt --title "API review"
```

Behavior:

- Relative paths resolve from `ctx.cwd`.
- Successful reviews are sent back into the Pi conversation as a user message containing the structured JSON export.
- If the agent is busy, the imported review is queued as a follow-up message.

## Tool usage

The `anno_review` tool supports both file review and generated-content review.

Supported parameters:

- `path`: review an existing file
- `content`: write generated text to a temp file before opening anno
- `fileName`: optional filename for generated content so anno can infer syntax from the extension
- `syntax`: optional `anno --syntax` override
- `title`: optional `anno --title` value

Generated-content review is intended for agent-driven workflows that need a temporary review file instead of an existing path.

## File and temp-data strategy

The extension supports two inputs:

- **Existing file review**: user supplies a path to an on-disk file.
- **Generated content review**: the extension writes supplied content to a temp file before launching anno.

Temp-file flow:

1. Resolve any user-supplied path against `ctx.cwd`.
2. For generated content, create a temp review file under the system temp directory.
3. Create a second temp file for anno's `--output-file` JSON export.
4. Run anno against the resolved real file or generated temp file.
5. In a `finally` block, remove temp files created by the extension.

## Interactive limitations and fallback behavior

This integration is intentionally interactive.

Important limitations:

- It only works when Pi has a live TUI (`ctx.hasUI`).
- It is not suitable for headless/background execution where Pi cannot give terminal control to anno.
- The slash command always reviews an on-disk file path; only the tool supports writing generated content to a temp file first.

The extension fails clearly when:

- `anno` is not on `PATH`
- Pi is running without a TUI / without `ctx.hasUI`
- the command/tool is asked to review a missing file
- anno exits unsuccessfully
- anno exits without exporting JSON (for example after `:q!`)
- anno emits invalid JSON

In those cases the extension returns a clear explanation so users or agents can fall back to a normal in-chat review or the older tmux-based skill.

## Migrating from the tmux-based review skill

The repository still includes the older [`skills/anno-tmux-review/`](../../skills/anno-tmux-review/) workflow. Use this table to choose between them:

| Workflow | Best for | Requirements | How it launches anno | Review result |
| --- | --- | --- | --- | --- |
| Pi extension (`pi/anno-review`) | Pi users already working inside Pi's interactive TUI | `anno` on `PATH`, Pi TUI available | Suspends Pi and runs anno directly in the current terminal | JSON export is parsed and imported back into the Pi conversation |
| tmux skill (`skills/anno-tmux-review`) | Agent workflows that need anno even when Pi cannot hand over the current terminal | `anno` on `PATH`, active tmux session (`$TMUX`) | Opens a separate tmux window and blocks until anno exits | Prints anno's default `agent` XML-like output to stdout |

Migration guidance:

1. Prefer `pi install ./pi/anno-review` when you want the smoothest Pi-native experience.
2. Update any human instructions from `scripts/anno-review.sh <file>` to `/anno-review <path>` when the review starts from an existing file inside Pi.
3. Keep the tmux skill as a fallback for non-TUI environments, remote automation, or sessions that already depend on tmux window management.
4. Expect different output shapes: the extension imports JSON review data back into Pi, while the tmux skill emits the legacy `agent` format to stdout.
