---
name: anno-tmux-review
description: Review plans, documents, code, or other text by writing the content to a temporary file, opening it in anno, and turning exported annotations into actionable feedback. Use when asked to review, annotate, mark up, or give feedback on text content.
compatibility: Requires the anno binary on PATH and an active tmux session ($TMUX env var set). The agent spawns anno in a new tmux window so no direct TTY is needed.
---

# Anno Review

Use this skill when the user wants interactive review of text content inside `anno` rather than a plain-text review in chat.

## When To Use

- The user asks to review, annotate, mark up, or give feedback on a plan, document, code snippet, or other text.
- The content can be written to a temporary file and reviewed in a terminal UI.

## Running The Script

The bundled script handles the entire tmux lifecycle: temp files, launching anno, blocking until the reviewer exits, printing annotations, and cleanup.

```bash
scripts/anno-review.sh <file> [--syntax <syntax>] [--title <title>]
```

- `file` -- path to the file to review (required).
- `--syntax` -- syntax hint for highlighting (default: `md`). Use `rs`, `ts`, `json`, `txt`, etc.
- `--title` -- title shown in anno's status bar (default: `Reviewing: <basename>`).

The script prints annotations to stdout and exits. If there are no annotations, stdout is empty.

If the content to review is not already on disk (e.g. a generated plan), write it to a temp file first, then pass that file to the script.

## Interpreting Output

If stdout is **empty**, the reviewer exited without annotations. Treat the content as approved unless the surrounding conversation suggests otherwise.

If stdout contains **annotations**, they are in anno's default `agent` format -- an XML-like structure designed for LLM consumption:

```xml
<annotations file="/tmp/anno-review-abc123.md" total="2">
The reviewer left 2 annotations on this document.

<comment line="5">
This line needs rewording.
</comment>

<comment>
Global feedback about the document.
</comment>

</annotations>
```

- Each annotation is a child element: `<comment line="N">`, `<deletion>`, `<replacement>`, `<insertion>`, or `<comment>` (no line attribute for global comments).
- Annotations appear in document order, with global comments last.
- Parse them and convert them into concrete revision tasks.

## Response Behavior

- If approved (no annotations): report approval and proceed.
- If annotations exist: summarize the requested changes as actionable feedback. Revise the text directly when the user asked for iterative improvement. Offer another review pass after revisions.

## Practical Notes

- If `anno` is missing or `$TMUX` is not set, the script exits with an error. Fall back to a normal in-chat review.
- If syntax is unclear, prefer `md` for prose-heavy content and `txt` as the conservative fallback.
