---
# anno-7ejd
title: Wire EnterVisualLineMode into app state and update mode predicates
status: completed
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:42:07.623278Z
updated_at: 2026-05-02T14:05:00.000000Z
parent: anno-1ouf
blocked_by:
- anno-8o31
- anno-xrp9
---

## Goal

Make `Action::EnterVisualLineMode` actually transition the app into
`Mode::VisualLine`, and update every place that branches on
`Mode::Visual` to also accept `Mode::VisualLine` so motions, search,
annotation creation, and rendering all work in line-mode the same way they
do in charwise visual.

## Shared Plan

Read [plan.md](../plan.md) before starting and append notes to it with any
decisions or surprises that downstream ishes need to inherit.

## Blocked By

- `anno-8o31` (mode + action wiring).
- `anno-xrp9` (linewise selection model).

## Changes

### `src/app/app_state/mod.rs`

- In the action-dispatch match (the same arm that currently handles
  `Action::EnterVisualMode` around line 156), add a branch for
  `Action::EnterVisualLineMode`:
  - Set `self.mode = Mode::VisualLine`.
  - Forward the action to `DocumentViewState::handle_action` so the anchor
    is set with `kind = Line` (work done in `anno-xrp9`).
- In the **allowed-actions** predicate around line 67 (the
  `Mode::Normal | Mode::Visual` arm that gates which actions can run in
  which mode), include `Mode::VisualLine` everywhere `Mode::Visual` is
  currently listed so motions, char-search, search, `gj/gk`, and the
  Visual-mode annotation creators are all permitted.
- Annotation creation paths (`CreateDeletion`, `CreateComment`,
  `CreateReplacement`) that call `take_visual_selection` should require
  `Mode::Visual` **or** `Mode::VisualLine`; the snapping inside
  `take_visual_selection` (from `anno-xrp9`) handles the linewise shape.

### `src/app/mod.rs`

- Update the `is_visual` flag (around line 201) from
  `self.state.mode() == Mode::Visual` to
  `matches!(self.state.mode(), Mode::Visual | Mode::VisualLine)` so the
  document view renders selection highlight in both modes.

### `src/app/app_state/search.rs`

- The search-confirm path that preserves the previous mode should treat
  `Mode::VisualLine` the same as `Mode::Visual` — entering search from
  Visual Line and confirming should return to Visual Line, not Normal.
  Mirror the existing `Mode::Visual` handling. Update the matching helper
  / match arms accordingly.

## Tests

Add a new module file `src/app/app_state/tests/visual_line.rs` (registered
in the existing `mod` block at the top of `tests/mod.rs` if present, or
follow whatever pattern the neighbors like `modes.rs` use). Cover:

- `V` (or directly dispatching `Action::EnterVisualLineMode` via the test
  harness) transitions from Normal to `Mode::VisualLine`.
- From `Mode::VisualLine`, motions like `MoveDown` extend the cursor and
  the resulting annotation range from `CreateDeletion` / `CreateComment` /
  `CreateReplacement` covers full lines.
- Confirming a `/search` from `Mode::VisualLine` returns to
  `Mode::VisualLine` (mirror
  `visual_search_confirm_preserves_visual_mode_and_selection`).
- `Esc` from `Mode::VisualLine` returns to `Mode::Normal` and clears the
  visual anchor.
- The `is_visual` rendering flag is true while in `Mode::VisualLine`
  (assert via whichever existing helper exposes the flag, or via a render
  test).

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Notes For Plan.md

If you discover additional `Mode::Visual` predicates (e.g. in panel state
or overlays) that also need to accept `Mode::VisualLine`, list them in the
**Architecture Touchpoints** table of `plan.md` so the toggle and count
ishes know to handle them too.
