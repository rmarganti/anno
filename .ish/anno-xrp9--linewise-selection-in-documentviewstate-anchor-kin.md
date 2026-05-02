---
# anno-xrp9
title: Linewise selection in DocumentViewState (anchor kind, snap, render)
status: completed
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:41:43.280660Z
updated_at: 2026-05-02T13:09:25.673865Z
parent: anno-1ouf
blocked_by:
- anno-8o31
---

## Goal

Teach `DocumentViewState` (and the document-view renderer) about linewise
selections. After this ish, with the anchor in line-kind, `take_visual_selection`
returns a whole-line range and `selected_text` ends in `\n`, and the visible
selection highlight covers full rows. App-state integration that *triggers*
this code path is handled in a separate ish.

## Shared Plan

Read [plan.md](../plan.md) before starting and update it with any decisions
or surprises so downstream ishes inherit them. Pay particular attention to
the **Selection Data Shape** and **Linewise Range Semantics** sections.

## Blocked By

- `anno-8o31` (provides `Action::EnterVisualLineMode`).

## Changes

### `src/tui/document_view.rs`

- Replace the existing `visual_anchor: Option<CursorPosition>` with a typed
  anchor that records both position and kind:

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  enum VisualKind { Char, Line }

  #[derive(Debug, Clone, Copy)]
  struct VisualAnchor { pos: CursorPosition, kind: VisualKind }

  visual_anchor: Option<VisualAnchor>,
  ```

- Update `Action::EnterVisualMode` handling in `handle_action` to set
  `kind = Char` (preserve current behavior).
- Add handling for `Action::EnterVisualLineMode` in `handle_action` to set
  `kind = Line` with the anchor at the current cursor.
- Update `clear_visual` to clear the new struct (already clears
  `visual_anchor`).
- Update `take_visual_selection` so when `kind == Line` it:
  - Computes `(start, end)` rows in document order from anchor and cursor.
  - Sets `start.col = 0`.
  - Sets `end.col = doc_lines[end.row].chars().count().saturating_sub(1)`
    (clamped to `0` for empty lines, matching existing empty-line behavior).
  - Builds the `TextRange` with those snapped positions.
  - Computes the `selected_text` via `selection::selected_text`, then
    appends a trailing `\n` (linewise yank shape).
- Update `render_document_view` (and any internal helper that materializes
  the selection rect) so when `kind == Line`, the `selection` passed to
  `prepare_visible_lines_from_slices` covers `(start_row, 0)` through
  `(end_row, last_char_index_of(end_row))`. Use the same snapping logic as
  `take_visual_selection`. The renderer call site should still treat both
  Visual and VisualLine as "selection visible" — pass `is_visual = true`
  when either is active.

### `src/tui/selection.rs`

- No code change required — `Selection::range` and `selected_text` already
  do the right thing for the underlying char-positions; the snapping is
  applied by the caller. Add a unit test alongside if helpful, but no
  module changes are mandatory.

## Tests (in `src/tui/document_view.rs`)

- Entering Visual Line via `Action::EnterVisualLineMode` sets `kind = Line`
  and anchors at the cursor.
- `take_visual_selection` after `EnterVisualLineMode` + `MoveDown` returns
  a `TextRange` covering full lines and `selected_text` ending in `\n`.
- `take_visual_selection` after `EnterVisualLineMode` only (no movement)
  covers exactly the cursor row, full-width, with one trailing `\n`.
- `take_visual_selection` after `EnterVisualMode` (charwise) still ends at
  the cursor's exact column with **no** trailing `\n` (regression guard).
- `clear_visual` clears a Line-kind anchor too.
- A render test asserting that with a line-kind anchor active, the rendered
  visible lines highlight every column of every selected row (mirror the
  existing visual selection render test).

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Notes For Plan.md

If `VisualAnchor` ends up named differently or grows fields (e.g. an
"original column" used by future `o` support), update the **Selection Data
Shape** section in `plan.md` so subsequent ishes match.


## Implementation Notes

- Added private `VisualKind` enum and `VisualAnchor` struct in [src/tui/document_view.rs](src/tui/document_view.rs).
- Replaced `visual_anchor: Option<CursorPosition>` with `visual_anchor: Option<VisualAnchor>`.
- `Action::EnterVisualMode` now sets `kind = Char` (preserved behavior).
- Added `Action::EnterVisualLineMode` handling to set `kind = Line`.
- `take_visual_selection` snaps the (start, end) pair via new private helper `snap_linewise` when `kind = Line` and appends a trailing `\n` to selected_text.
- `render_document_view` reuses the same snapping so the visible selection highlight covers full rows.
- `Selection` in [src/tui/selection.rs](src/tui/selection.rs) was left unchanged — snapping is applied by the caller as planned.

## Tests Added

- `enter_visual_line_mode_sets_line_anchor_at_cursor`
- `take_visual_line_selection_only_anchor_covers_full_row_with_newline`
- `take_visual_line_selection_after_move_down_covers_full_lines`
- `take_visual_line_selection_handles_anchor_below_cursor`
- `take_visual_line_selection_on_empty_line_clamps_end_col`
- `charwise_visual_selection_does_not_get_trailing_newline` (regression guard)
- `clear_visual_clears_line_kind_anchor`
- `render_visual_line_highlights_every_column_of_selected_rows`

## Verification

- `cargo fmt --all -- --check` ✓
- `cargo clippy --all-targets --all-features -- -D warnings` ✓
- `cargo build --all-features` ✓
- `cargo test --all-features` ✓ for all 54 `tui::document_view` tests. The 4 pre-existing failures in `highlight::syntect` reproduce on the base branch and are unrelated to this work.
