---
# anno-xxc4
title: Register /anno-last slash command
status: completed
type: task
priority: normal
created_at: 2026-04-16T17:32:28Z
updated_at: 2026-04-16T18:06:24Z
parent: anno-orc1
blocked_by:
    - anno-ie1v
---

## What

Register the `/anno-last` command in the `annoReviewExtension` function in `pi/anno-review/index.ts`.

## Details

Added a `pi.registerCommand("anno-last", { ... })` call inside `annoReviewExtension` that:

1. Validates zero-argument usage.
2. Reads the latest assistant message via `getLastAssistantMessageText(ctx)`.
3. Launches `runReview({ content, fileName: "last-message.md", title: "Last Agent Message" }, ctx)`.
4. Reuses the existing success, cancellation, and idle vs follow-up message delivery flow from `/anno-review`.

## Where

`pi/anno-review/index.ts`

## Checklist

- [x] Add `pi.registerCommand('anno-last', ...)` with handler
- [x] Handler retrieves last assistant message via `getLastAssistantMessageText`
- [x] Handler calls `runReview` with content mode
- [x] Handler sends feedback as user message (idle vs follow-up)

## Summary of Changes

- Registered `/anno-last` in `annoReviewExtension` with zero-argument validation, idle vs follow-up delivery, and the same import flow used by `/anno-review`.
- The command writes the last assistant message to a temporary markdown file via the existing content-review path and feeds the exported annotations back into Pi as a user message.
