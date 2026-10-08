---
name: anno-herdr-review
description: Review plans, documents, code, or other text by opening it in anno in a new Herdr tab and turning the exported annotations into actionable feedback. Use when asked to review, annotate, mark up, or give feedback on text content while running inside Herdr.
compatibility: Requires the anno binary on PATH and a Herdr pane ($HERDR_ENV=1).
---

# Anno Herdr Review

Use this skill when the user wants interactive review of text content inside `anno` rather than a plain-text review in chat.

## When To Use

- The user asks to review, annotate, mark up, or give feedback on a plan, document, code snippet, or other text.
- You are running inside Herdr (`HERDR_ENV=1`).

The person can also start a review on their own with `/anno-review <path>` or `/anno-last`. Those commands send the export back as their next message, so you never run the launcher for them.

## Running The Launcher

The launcher lives two directories above this skill, at `bin/anno-herdr-review` in the plugin root. It opens anno in a new Herdr tab, blocks until the reviewer quits, and prints anno's `agent` export to stdout.

```bash
<plugin root>/bin/anno-herdr-review <file> [--syntax <syntax>] [--title <title>]
<plugin root>/bin/anno-herdr-review --stdin --name plan.md [--title <title>] < content
```

- `file` -- path to the file to review.
- `--stdin` / `--name` -- review generated content without writing it to disk yourself; `--name` sets the file name (and so the highlighting) anno sees.
- `--syntax` -- syntax hint for highlighting. Use `md`, `rs`, `ts`, `json`, `txt`, etc.
- `--title` -- title shown in anno's status bar (default: `Reviewing: <basename>`).

A review can take a while. Run the launcher with Bash's `run_in_background` so the ten-minute foreground limit never cuts it off, and read its output when it finishes.

Exit codes:

- `0` -- stdout holds the export.
- `3` -- the reviewer quit without exporting (`:q!`). Treat it as a cancelled review and ask how to proceed.
- `1` -- missing dependency, missing file, or the review tab failed. The error says which; fall back to a normal in-chat review.

## Interpreting Output

Exports are in anno's `agent` format, an XML-like structure designed for LLM consumption:

```xml
<annotations file="/tmp/anno-herdr-review.abc123/plan.md" total="2">
The reviewer left 2 annotations on this document.

<annotation type="comment" line="5">
<selected_text>
The selected source text.
</selected_text>
<comment>
This line needs rewording.
</comment>
</annotation>

<annotation type="global_comment">
<comment>
Global feedback about the document.
</comment>
</annotation>

</annotations>
```

- Each annotation is `<annotation type="..." line="N">` or `<annotation type="..." lines="N-M">`, with children such as `<selected_text>`, `<comment>`, `<replacement>`, or `<text>`.
- Annotations appear in document order, with global comments last.
- An export with `total="0"` means the reviewer quit with `:q` and left no annotations. Treat the content as approved unless the conversation suggests otherwise.

## Response Behavior

- If approved (no annotations): report approval and proceed.
- If annotations exist: turn them into concrete revisions. Revise the text directly when the user asked for iterative improvement, then offer another review pass.
