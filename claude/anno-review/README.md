# Claude Code anno review plugin

Use this Claude Code plugin to open `anno` in a floating Herdr popup and bring the exported review back into the conversation.

## Prerequisites

- Install `anno` and make sure the `anno` binary is available on `PATH`.
- Run Claude Code inside a [Herdr](https://herdr.dev) pane. Claude Code plugins can't hand their own terminal to another program, so anno opens in a Herdr popup over the session instead.
- Install the companion Herdr plugin, which provides the popup:

```bash
herdr plugin install rmarganti/anno/herdr/anno-review
# or, from a local checkout:
herdr plugin link ./herdr/anno-review
```

## Installation

At a Claude Code prompt:

```
/plugin install anno-review --marketplace rmarganti/anno
```

Answer `y` to add the marketplace, then pick a scope.

To run the plugin from a local checkout instead:

```bash
claude --plugin-dir ./claude/anno-review
```

## Entry points

- Slash command: `/anno-review <path> [--syntax <syntax>] [--title <title>]`
- Slash command: `/anno-last`
- Skill: `anno-herdr-review`

Use `/anno-review` when you want to review an existing file.
Use `/anno-last` when you want to annotate the most recent assistant response.
The skill lets Claude start a review on its own, for example when you ask it to "have me review the plan in anno".

## Slash command usage

Review an existing file:

```
/anno-review path/to/file.md
/anno-review docs/api.md --syntax markdown
/anno-review notes.txt --title "API review"
```

Annotate the last assistant message:

```
/anno-last
```

Behavior:

- Relative paths resolve from the session's working directory.
- `/anno-last` sends the last assistant response's text to anno as `last-message.md`.
- Both commands return right away. anno stays open in the popup for as long as you need, and Claude Code stays usable underneath.
- When you quit with `:q`, the review is sent to Claude as your next message, containing anno's structured `agent` export. If Claude is busy, the review waits until the current turn finishes.
- Quitting with `:q!` cancels the review and shows a notice instead.

## How it fits together

```
/anno-review or /anno-last           (hooks/register.ts)
  └─ bin/anno-herdr-review           opens the popup, waits, prints the export
       └─ herdr plugin pane open --placement popup
            └─ herdr/anno-review/bin/anno-popup   runs anno, records its exit status
```

`bin/anno-herdr-review` also works on its own. It prints the export to stdout and exits `0`, `3` when the reviewer quit without exporting, or `1` on failure. Run it with `--help` for details.

## Limitations

The commands fail with a clear notice when:

- Claude Code isn't running inside a Herdr pane
- `anno` or `herdr` isn't on `PATH`
- the `rmarganti.anno-review` Herdr plugin isn't installed
- the file to review doesn't exist
- anno exits before exporting annotations

## Development

```bash
claude plugin validate claude/anno-review
claude plugin test claude/anno-review
```

A session started with `--plugin-dir` reloads the plugin whenever a file in it changes.
