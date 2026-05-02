---
# anno-6x5o
title: Implement [count]V to pre-extend Visual Line selection
status: completed
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:42:54.931665Z
updated_at: 2026-05-02T13:31:37.467085Z
parent: anno-1ouf
blocked_by:
- anno-7ejd
---

## Goal

Honor a numeric count prefix on `V`: pressing `3V` from Normal enters
Visual Line and selects 3 lines (the anchor row plus two rows below). This
mirrors vim's `[count]V` semantics.

## Shared Plan

Read [plan.md](../plan.md) before starting and update it if any of this
ish's implementation choices change something a future ish will rely on.

## Blocked By

- `anno-7ejd` (app-state already handles `EnterVisualLineMode`).

Note: this ish is independent of `anno-1am6` (toggle); they can land in
either order.

## Background

`anno-8o31` already added `EnterVisualLineMode` to `Action::supports_count`,
so the keybind handler will produce
`Action::Repeat { action: Box::new(EnterVisualLineMode), count }` for
`[count]V`. We need to handle that wrapped form in app-state without
re-entering the mode `count` times (which would re-anchor on each
iteration).

## Changes

### `src/app/app_state/mod.rs`

- In the `Action::Repeat { action, count }` dispatch path, special-case
  `EnterVisualLineMode`: enter Visual Line once (set the anchor, switch
  mode), then move the cursor down `count - 1` rows via `MoveDown`
  invocations on the `DocumentViewState` (or one batched call if a helper
  exists).
- Be careful not to lose the anchor: the anchor is set on the first
  `EnterVisualLineMode`, then `MoveDown` is dispatched as normal motions
  while in `Mode::VisualLine`; the existing motion plumbing already
  extends the selection without resetting the anchor.

If the existing `Repeat` runtime would naturally execute `EnterVisualLineMode`
followed by `(count - 1)` no-op iterations, you can instead intercept
`Action::Repeat { action, count }` where `*action == EnterVisualLineMode`
and translate it into:

1. one `EnterVisualLineMode` dispatch, then
2. a single `Action::Repeat { action: Box::new(MoveDown), count: count - 1 }`
   dispatch (if `count > 1`).

Either implementation is acceptable; the goal is "anchor at original cursor
row, cursor row = anchor + count - 1, mode = VisualLine".

## Tests

### `src/app/app_state/tests/visual_line.rs`

- From Normal at row 2, dispatch `[3]V` (count=3): mode becomes
  `Mode::VisualLine`, anchor stays at row 2, cursor moves to row 4.
- `1V` (count=1) is identical to plain `V`: anchor and cursor on the same
  row.
- `[N]V` where `N` would push the cursor past the last document row clamps
  the cursor to the last row (matches the existing `MoveDown` clamping
  behavior; assert no panic).
- After `[3]V`, creating a deletion annotation produces a `TextRange`
  covering the 3 selected lines.

### `src/keybinds/handler.rs`

- A regression test confirming `3V` is dispatched as
  `Action::Repeat { action: Box::new(Action::EnterVisualLineMode), count: 3 }`.

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Notes For Plan.md

If you change the `Action::Repeat` runtime in a way that affects other
counted actions (e.g. `[count]v`), record the change under **Open Notes**
in `plan.md` so the docs ish reflects the right behavior.

## Implementation Notes

- `AppState::dispatch_repeat` now intercepts
  `Action::EnterVisualLineMode` and translates `[count]V` into one visual
  line-mode entry plus `count - 1` repeated `MoveDown` actions.
- Reusing the existing `MoveDown` path preserves the original visual
  anchor and inherits the document-view cursor clamping behavior at EOF
  without changing counted motion semantics elsewhere.
- Added app-state coverage for `3V`, `1V`, EOF clamping, and deletion over
  a counted linewise selection in
  `src/app/app_state/tests/visual_line.rs`.

## Verification Results

- `cargo fmt --all -- --check` ✓
- `cargo test --all-features` ✓
- `cargo clippy --all-targets --all-features -- -D warnings` ✓
- `cargo build --all-features` ✓
