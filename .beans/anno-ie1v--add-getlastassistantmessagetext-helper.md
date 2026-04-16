---
# anno-ie1v
title: Add getLastAssistantMessageText helper
status: completed
type: task
priority: normal
created_at: 2026-04-16T17:32:11Z
updated_at: 2026-04-16T18:06:24Z
parent: anno-orc1
---

## What

Add a helper function to `pi/anno-review/index.ts` that retrieves the last assistant message text from the Pi session history.

## Details

- Added module-private helpers to extract joined `text` blocks from assistant messages.
- `getLastAssistantMessageText(ctx: ExtensionContext)` walks `ctx.sessionManager.getBranch()` and returns the latest assistant text on the active branch.

## Where

`pi/anno-review/index.ts`

## Checklist

- [x] Add `AssistantTextBlock` and `AssistantMessageLike` types
- [x] Add `isAssistantMessage` type guard
- [x] Add `getTextContent` helper
- [x] Add `getLastAssistantMessageText` function

## Summary of Changes

- Added module-private helpers in `pi/anno-review/index.ts` to locate the latest assistant message on the active branch and extract joined text blocks for review.
- The implementation uses `ctx.sessionManager.getBranch()` so `/anno-last` follows the current conversation branch instead of scanning detached history.
