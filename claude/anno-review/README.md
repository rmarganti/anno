# Claude Code anno review plugin

Use this Claude Code plugin to open `anno` in a new Herdr tab and bring the exported review back into the conversation.

## Prerequisites

- Install `anno` and make sure the `anno` binary is available on `PATH`.
- Run Claude Code inside a [Herdr](https://herdr.dev) pane. Claude Code plugins can't hand their own terminal to another program, so anno opens in a new Herdr tab instead. No Herdr plugin is needed.

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
- Both commands return right away. anno opens in a focused tab labeled `anno` and stays open for as long as you need. When anno exits, the tab closes and focus returns to the tab you started from.
- When you quit with `:q`, the review is sent to Claude as your next message, containing anno's structured `agent` export. If Claude is busy, the review waits until the current turn finishes.
- Quitting with `:q!` cancels the review and shows a notice instead.

## How it fits together

```
/anno-review or /anno-last           (hooks/register.ts)
  └─ bin/anno-herdr-review     opens a tab, waits, prints the export
       └─ herdr tab create + herdr pane run
            └─ bin/anno-tab-runner   runs anno, records its exit status
```

`bin/anno-herdr-review` also works on its own. It prints the export to stdout and exits `0`, `3` when the reviewer quit without exporting, or `1` on failure. Run it with `--help` for details.

## Limitations

The commands fail with a clear notice when:

- Claude Code isn't running inside a Herdr pane
- `anno` or `herdr` isn't on `PATH`
- the file to review doesn't exist
- anno exits before exporting annotations

## Development

```bash
claude plugin validate claude/anno-review
claude plugin test claude/anno-review
```

A session started with `--plugin-dir` reloads the plugin whenever a file in it changes.
