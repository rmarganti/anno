# Pi anno review package

Use this Pi extension to open `anno` directly from Pi and bring the exported review back into the conversation.

## Prerequisites

Before using this extension:

- Install `anno` and make sure the `anno` binary is available on `PATH`.
- Run Pi in its interactive TUI. The extension needs terminal control so it can temporarily stop Pi, hand the terminal to anno, then restart Pi.
- Keep this repository available locally if you want to install directly from `./pi/anno-review` or symlink back to the in-repo `index.ts`.

## Installation

### Preferred: install from this repository

From the repository root:

```bash
pi install ./pi/anno-review
```

### Or: copy or symlink into Pi's extension directories

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

Copying also works, but symlinks are convenient during development because `/reload` can pick up edits.

## Entry points

- Package name: `anno-pi-review`
- Slash command: `/anno-review`
- Custom tool: `anno_review`

Use `/anno-review` when you want to review an existing file from Pi chat.
Use `anno_review` for interactive tool-driven review flows.

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

Parameters:

- `path`: review an existing file
- `content`: write generated text to a temp file before opening anno
- `fileName`: optional filename for generated content so anno can infer syntax from the extension
- `syntax`: optional `anno --syntax` override
- `title`: optional `anno --title` value

Generated-content review is useful when an agent needs to review text that is not already saved to disk.

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

In those cases the extension returns a clear explanation so users or agents can choose another review workflow.
