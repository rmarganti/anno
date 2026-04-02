# OpenCode `/anno-review` plugin

Use this repo-local OpenCode TUI plugin to open `anno` in the current terminal, capture its JSON export, and append the review back into the active OpenCode prompt.

## Prerequisites

Before using the plugin:

- Install `anno` and make sure the `anno` binary is available on `PATH`.
- Run OpenCode in its interactive TUI. The plugin temporarily suspends the renderer, gives terminal control to `anno`, then resumes the TUI.
- Keep this repository available locally if you want to load the plugin from `./opencode/anno-review`.
- Use an `anno` version that supports `--export-format json` and `--output-file`. The plugin also uses `--syntax` when requested and will drop `--title` automatically on older `anno` builds that do not support it.

## Installation and loading

OpenCode loads this integration as a TUI plugin package.

### Repo-local plugin config

This repository already includes a project-local loader config at [`.opencode/tui.json`](../../.opencode/tui.json):

```json
{
  "$schema": "https://opencode.ai/tui.json",
  "plugin": ["../opencode/anno-review"]
}
```

That path is resolved relative to `.opencode/tui.json`, so OpenCode loads the package directory and then imports its `./tui` export.

### Package details

- Package directory: `opencode/anno-review`
- Package name: `anno-opencode-review`
- TUI export: `./tui -> ./index.js`
- Slash command: `/anno-review`

## Usage

The plugin is slash-command only.

```text
/anno-review <path> [--syntax <syntax>] [--title <title>]
```

Examples:

```text
/anno-review README.md
/anno-review docs/api.md --syntax markdown
/anno-review notes.txt --title "API review"
```

Behavior:

- Relative paths resolve from the current OpenCode working directory.
- `/anno-review` is available from the home prompt and active session prompt.
- OpenCode opens a dialog prompt for the arguments, then launches `anno` in the same terminal.
- On success, the plugin appends a structured summary plus fenced JSON export back into the active prompt and tries to submit it automatically.
- If auto-submit fails, the review text stays in the prompt and OpenCode shows a warning asking you to submit manually.

## Interactive limitations

This integration is intentionally interactive.

Important limitations:

- It only works in the OpenCode TUI; headless/background use is unsupported.
- It is a slash command, not a general-purpose custom tool.
- It reviews on-disk files only.
- It requires terminal handoff, so it should not be used from non-interactive automation.

The plugin fails clearly when:

- `anno` is not on `PATH`
- OpenCode is not attached to an interactive terminal
- the current route is not `home` or `session`
- the requested file does not exist
- `anno` exits unsuccessfully
- `anno` exits without exporting JSON, such as after `:q!`
- the exported JSON is malformed or does not match the expected schema

## Automated validation

Run the plugin-focused tests with:

```bash
node --test opencode/anno-review/*.test.mjs
```

Current automated coverage includes:

- command argument tokenization and parsing
- path resolution
- `anno --help` capability gating
- launch planning and `--title` fallback behavior
- missing export treated as cancellation
- malformed or inconsistent JSON export rejection
- import-to-session success and manual-submit fallback
- plugin registration metadata and slash-command dialog wiring

If repository code changed, also run the required project-wide checks from `AGENTS.md`:

```bash
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Manual smoke-test checklist

Use this checklist after changing the plugin or terminal handoff logic.

### 1. Successful review

- Start OpenCode TUI from the repository root.
- Confirm `.opencode/tui.json` points at `../opencode/anno-review`.
- Open the slash command and run:

  ```text
  /anno-review README.md
  ```

- Add at least one annotation in `anno`.
- Quit with `:q`.
- Verify OpenCode resumes cleanly.
- Verify a success toast appears.
- Verify the active prompt receives the review summary plus fenced JSON.

### 2. Cancellation path

- Run:

  ```text
  /anno-review README.md
  ```

- Exit `anno` with `:q!`.
- Verify OpenCode resumes cleanly.
- Verify the plugin reports that `anno` closed without exporting annotations.

### 3. Missing `anno` path

- Start OpenCode with `anno` hidden from `PATH`, or temporarily run OpenCode from an environment where `anno` is unavailable.
- Run `/anno-review README.md`.
- Verify the plugin shows `anno is not available on PATH.` without suspending into a broken launch.

### 4. Malformed export handling

This case is covered primarily by automated tests because it is awkward to induce reliably through the interactive TUI path.

- Run:

  ```bash
  node --test opencode/anno-review/*.test.mjs
  ```

- Verify the malformed-export and schema-validation tests pass.
- If you intentionally change export parsing logic, add or update a focused test before relying on a manual check.

## Developer notes

- Interactive smoke tests are still required because renderer suspend/resume and inherited stdio cannot be fully proven with unit tests.
- The plugin cleans up its temporary export directory in a `finally` block after every launch attempt.
- When `anno --title` is unsupported, the review still runs and OpenCode shows an informational note explaining that the custom title was omitted.
