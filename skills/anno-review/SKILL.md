---
name: anno-review
description: Review plans, documents, code, or other text by writing the content to a temporary file, opening it in anno, and turning exported annotations into actionable feedback. Use when asked to review, annotate, mark up, or give feedback on text content.
compatibility: Requires the anno binary on PATH and an interactive terminal session that can launch its TUI.
---

# Anno Review

Use this skill when the user wants interactive review of text content inside `anno` rather than a plain-text review in chat.

## When To Use

- The user asks to review, annotate, mark up, or give feedback on a plan, document, code snippet, or other text.
- The content can be written to a temporary file and reviewed in a terminal UI.
- Structured output is useful for follow-up edits.

## Inputs To Gather

- The content to review.
- A short review context such as `Implementation Plan`, `README Draft`, or `Rust Module`.
- The best syntax hint for highlighting, such as `md`, `rs`, `json`, `ts`, or `txt`.

## Workflow

1. Normalize the review input into a single text buffer.
2. Compute a deterministic temp path so repeated reviews of the same content reuse the same file when practical.
   Example pattern: `/tmp/anno-review-<hash>.md`.
3. Pick an extension that matches the content when possible.
   Use `.md` for markdown, `.rs` for Rust, `.ts` for TypeScript, `.json` for JSON, otherwise `.txt`.
4. Write the review content to that temp file.
5. Launch `anno` against the temp file instead of piping via stdin.
   This keeps syntax detection stronger and causes exported annotations to reference a file path instead of `source="stdin"`.
6. Prefer JSON export when available.
   Run `anno --format json --syntax <syntax> <temp-file>`.
7. If a future `anno` build supports `--title`, include it.
   The current repo supports `--format` and `--syntax`, but not `--title`, so do not assume that flag exists.
8. Let the human review interactively in `anno`.
   `:q` exports annotations to stdout.
   `:q!` exits silently with no review output.
9. Capture stdout after `anno` exits.
10. Interpret the result:
    - Empty stdout means the reviewer exited with `:q!` or otherwise produced no annotations. Treat that as approval or no requested changes.
    - Non-empty stdout means the reviewer left annotations. Parse them and convert them into concrete revision tasks.
11. Apply revisions to the source text when the workflow calls for edits, then summarize what changed.
12. Offer another review pass if the user wants the revised content checked again.

## Command Patterns

Preferred command:

```bash
anno --format json --syntax md /tmp/anno-review-<hash>.md
```

Fallback when JSON is unavailable in the installed binary:

```bash
anno --syntax md /tmp/anno-review-<hash>.md
```

Future-friendly form when `--title` exists:

```bash
anno --title "Reviewing: Implementation Plan" --format json --syntax md /tmp/anno-review-<hash>.md
```

## Parsing Guidance

- Prefer `--format json` and parse the returned JSON object directly.
- Expect the export to identify the temp file path as the review source.
- If JSON is unavailable, parse the default XML-like export.
- Preserve annotation ordering because `anno` exports annotations in document order, with global comments last.
- Treat deletion, replacement, insertion, comment, and global comment annotations as distinct actions.

## Response Behavior

If stdout is empty:

- Report that the reviewer exited without annotations.
- Treat the content as approved unless the surrounding conversation suggests otherwise.
- Proceed with the approved artifact or ask whether to continue.

If stdout contains annotations:

- Summarize the requested changes as actionable feedback.
- Revise the text directly when the user asked for iterative improvement.
- Keep the original review output available in case the user wants to inspect it verbatim.
- Offer a second review pass after revisions.

## Practical Notes

- Use a file path, not stdin, so the exported source and syntax handling stay predictable.
- Keep temp file naming deterministic from the content hash so repeated runs are easy to correlate.
- If syntax is unclear, prefer `md` for prose-heavy content and `txt` as the conservative fallback.
- If `anno` is missing or the environment is non-interactive, fall back to a normal in-chat review and say the skill could not run as designed.
