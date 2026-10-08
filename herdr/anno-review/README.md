# Herdr anno review plugin

A Herdr plugin that opens `anno` in a floating popup. Coding agent integrations, such as the Claude Code plugin in [`claude/anno-review`](../../claude/anno-review/README.md), use it to collect annotations without giving up their own terminal.

## Installation

```bash
herdr plugin install rmarganti/anno/herdr/anno-review
# or, from a local checkout:
herdr plugin link ./herdr/anno-review
```

## Pane entrypoint

The plugin provides a single pane entrypoint, `review`, placed as a 90% × 90% popup. It reads its job from the environment:

| Variable | Required | Meaning |
| --- | --- | --- |
| `ANNO_REVIEW_FILE` | yes | Absolute path of the file to review |
| `ANNO_REVIEW_OUTPUT` | yes | Where anno writes its `agent` export |
| `ANNO_REVIEW_DONE` | no | Marker file. `<marker>.started` holds the popup's PID while it runs, and `<marker>` holds anno's exit status once it quits |
| `ANNO_REVIEW_TITLE` | no | Title shown in anno's status bar |
| `ANNO_REVIEW_SYNTAX` | no | Syntax hint for highlighting |
| `ANNO_BIN` | no | anno binary to run (default: `anno` on the popup's `PATH`) |

```bash
herdr plugin pane open --plugin rmarganti.anno-review --entrypoint review --placement popup \
  --env ANNO_REVIEW_FILE="$PWD/notes.md" \
  --env ANNO_REVIEW_OUTPUT=/tmp/notes.agent \
  --env ANNO_REVIEW_DONE=/tmp/notes.done
```

Herdr doesn't report a pane ID for popups, so callers wait on the marker file instead.
