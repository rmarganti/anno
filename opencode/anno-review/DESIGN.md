# OpenCode `/anno-review` design

## Chosen layout

Use a repo-local **TUI plugin package** under `opencode/anno-review/`.

Planned files:

- `opencode/anno-review/package.json`
- `opencode/anno-review/index.tsx`
- `opencode/anno-review/README.md`
- optional helpers if the implementation wants to split parsing/anno launching/import logic

Planned package shape:

```json
{
  "name": "anno-opencode-review",
  "type": "module",
  "exports": {
    "./tui": "./index.tsx"
  },
  "peerDependencies": {
    "@opencode-ai/plugin": "*",
    "@opentui/core": "*",
    "@opentui/solid": "*"
  }
}
```

Why this layout:

- matches OpenCode's current TUI plugin model (`default export { id, tui }`)
- mirrors the existing repo-local `pi/anno-review/` package shape
- keeps slash-command code, docs, and future packaging in one place
- allows local loading from `.opencode/tui.json` now and npm packaging later

Planned local config entry during implementation:

```json
{
  "$schema": "https://opencode.ai/tui.json",
  "plugin": ["../opencode/anno-review"]
}
```

That path is resolved relative to `.opencode/tui.json`. OpenCode's TUI plugin loader resolves `./tui` from package exports for package/path specs, so we should point at the package directory rather than a loose file.

## Command contract

Slash command:

```text
/anno-review <path> [--syntax <syntax>] [--title <title>]
```

Behavior:

- exactly one positional path is required
- quoted arguments are supported
- relative paths resolve from `api.state.path.directory`
- command is intended for **existing on-disk files only**
- generated-content review is explicitly out of scope for this bead chain; if needed later, add a second command or a temp-source helper in a follow-up

Success path:

1. Parse args.
2. Validate the current TUI context is suitable.
3. Resolve the review target path.
4. Suspend the OpenCode renderer.
5. Run `anno` with inherited stdio.
6. Resume the renderer.
7. Parse exported JSON.
8. Append an import message to the current OpenCode prompt.
9. Submit that prompt so the review lands in the conversation as a user message.
10. Show a success toast.

Import message format:

- same basic structure as the Pi extension:
  - short natural-language summary
  - fenced JSON block containing the structured anno export
- this keeps the imported content easy for the agent to consume without inventing a second schema

## OpenCode APIs to use

### Command registration

Use `api.command.register(() => [...])` from `@opencode-ai/plugin/tui` with a slash definition:

```ts
{
  title: "Anno review",
  value: "anno-review.run",
  category: "Review",
  slash: { name: "anno-review" },
  onSelect: () => { ... }
}
```

Relevant source references:

- `.local/repos/opencode/packages/plugin/src/tui.ts`
- `.local/repos/opencode/packages/opencode/specs/tui-plugins.md`

### Renderer handoff

Use `api.renderer.suspend()` / `api.renderer.resume()` / `api.renderer.requestRender()`.

Behavior should copy the built-in external editor helper pattern from:

- `.local/repos/opencode/packages/opencode/src/cli/cmd/tui/util/editor.ts`

Recommended implementation:

- clear `api.renderer.currentRenderBuffer` before and after handoff
- spawn `anno` with `stdin/stdout/stderr: "inherit"`
- resume in a `finally` block

### Session import

Use the TUI client APIs instead of reconstructing raw `session.prompt(...)` payloads:

- `api.client.tui.appendPrompt({ text })`
- `api.client.tui.submitPrompt()`

Why this path:

- reuses the host prompt flow already wired for home/session routes
- avoids having to rediscover active agent/model/variant selection logic from the prompt component
- preserves current OpenCode semantics for creating a new session from home vs posting into an existing session

Relevant references:

- `.local/repos/opencode/packages/opencode/src/server/routes/tui.ts`
- `.local/repos/opencode/packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx`

## Execution and temp-file strategy

Per invocation, create one temp directory such as:

```text
$TMPDIR/anno-review-XXXXXX/
```

Contents:

- `annotations.json` — anno export target
- optional future temp review file if generated-content review is added later

For the current slash command:

- do **not** copy the reviewed file
- pass the resolved real file path directly to `anno`
- remove the temp directory in `finally`

This keeps the current command simple while leaving one obvious place to add generated-content support later.

## Anno CLI compatibility policy

Preferred invocation against the current repo's anno CLI:

```bash
anno --export-format json --output-file <tmp>/annotations.json [--title <title>] [--syntax <syntax>] <path>
```

Compatibility behavior:

1. **Primary path**: run with `--title` when provided.
2. **Title fallback**: if launch fails specifically because `--title` is unsupported, retry once without `--title`.
3. **Syntax flag**: pass through unchanged when provided. If `--syntax` is unsupported, fail clearly rather than guessing.
4. **JSON export flags** (`--export-format json --output-file`) are mandatory for this integration. If unsupported, fail with an upgrade message instead of trying to scrape stdout.

Reasoning:

- `--title` is cosmetic and safe to degrade.
- JSON export is required for deterministic import back into OpenCode.
- keeping export handling strict prevents ambiguous partial-success states.

## Explicit failure cases

The command should show a toast/error and avoid prompt submission when:

- current route is neither `home` nor `session`
- `api.state.path.directory` is not available yet
- the target file does not exist
- `anno` is not on `PATH`
- stdin/stdout are not interactive TTYs
- `anno` exits without producing the JSON export file
- `anno` produces invalid JSON
- `anno` exits non-zero
- the user cancels review (treat as info, not error)

Cancellation heuristic:

- if `anno` exits successfully but no export file exists, treat it like a cancelled review (`:q!`-style exit)

## Validation steps for implementation beads

### `anno-shy.2`

- load the plugin locally from `.opencode/tui.json`
- verify `/anno-review README.md` opens anno and returns to the TUI cleanly
- verify renderer resumes after normal exit and after launch failure
- verify quoted path parsing and `--title` / `--syntax` parsing

### `anno-shy.3`

- verify a successful review appends/submits a prompt back into OpenCode
- verify the import lands in the current session when run from a session route
- verify running from home creates/submits through the home prompt flow
- verify cancelled review does not submit a prompt
- verify invalid JSON / missing export file produce clear toasts

### `anno-shy.4`

- document install/load instructions and interactive limitations
- smoke-test missing-`anno` and missing-file failures
- smoke-test the title fallback path if an older anno binary is available; otherwise document as manually verified by code inspection
- record final usage examples and failure modes in package README

## Notes for future implementation

- Keep this as a **human-in-the-loop slash command**, not an LLM tool.
- Do not build this as a server plugin; the required renderer suspend/resume APIs are on the TUI side.
- If generated-content review becomes necessary later, add it without changing the current slash command contract for path review.
