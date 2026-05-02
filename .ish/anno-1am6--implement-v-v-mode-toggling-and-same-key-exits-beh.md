---
# anno-1am6
title: Implement v <-> V mode toggling and same-key-exits behavior
status: todo
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:42:31.922115Z
updated_at: 2026-05-02T02:42:31.922115Z
parent: anno-1ouf
blocked_by:
- anno-7ejd
---

## Goal

Make `v` and `V` toggle between charwise Visual and linewise Visual Line
the way vim does, and make pressing the same mode key while already in that
mode exit to Normal. The mode change preserves the existing visual anchor
position; only its `kind` flips.

## Shared Plan

Read [plan.md](../plan.md) before starting and append notes about anything
the docs / count ishes need to know.

## Blocked By

- `anno-7ejd` (app-state mode dispatch and predicates already accept
  `Mode::VisualLine`).

## Behavior Matrix

| Current mode | Key | Result |
|---|---|---|
| Normal | `v` | Enter Visual (existing) |
| Normal | `V` | Enter Visual Line (existing, from `anno-8o31`) |
| Visual | `v` | Exit to Normal (existing) |
| Visual | `V` | Switch to Visual Line, keep anchor |
| Visual Line | `v` | Switch to Visual, keep anchor |
| Visual Line | `V` | Exit to Normal |

## Changes

### `src/keybinds/handler.rs`

- In `handle_visual`: bind `(KeyCode::Char('V'), KeyModifiers::SHIFT)` →
  `Action::EnterVisualLineMode`. Leave the existing `v` → `ExitToNormal`
  alone (that's the current behavior).
- In `handle_visual_line`: bind `(KeyCode::Char('v'), KeyModifiers::NONE)`
  → `Action::EnterVisualMode`, and `(KeyCode::Char('V'), KeyModifiers::SHIFT)`
  → `Action::ExitToNormal`.
- Counts on `v`/`V` from inside Visual or Visual Line should be ignored
  (vim does not apply counts to mode toggles in this direction). Keep them
  unmarked in `supports_count` and ensure pending counts are cleared when
  one of these toggle keys is consumed.

### `src/app/app_state/mod.rs`

- Update the `EnterVisualMode` dispatch so that, when entering from
  `Mode::VisualLine`, it preserves the existing anchor position and just
  flips its kind to `Char` (rather than re-anchoring at the cursor). This
  matches vim, where `V → v` keeps the same anchor row/col and only the
  selection model changes.
- Update the `EnterVisualLineMode` dispatch to do the symmetric thing when
  coming from `Mode::Visual`: keep the existing anchor position and flip
  its kind to `Line`.
- The cleanest implementation is probably a small helper on
  `DocumentViewState` (e.g. `set_visual_kind(VisualKind)` / a method that
  returns `true` if there's already an anchor). If you change the public
  surface, document it in `plan.md`.

### Optional: `src/tui/document_view.rs`

- Add a helper such as `pub fn set_visual_kind(&mut self, kind: VisualKind)`
  that, if `visual_anchor` is `Some`, only updates its `kind` and otherwise
  installs a new anchor at the current cursor (so the helper works for
  either entry path). Called from app-state.

## Tests

### `src/keybinds/handler.rs`

- From `Mode::Visual`: `V` returns `Action::EnterVisualLineMode`.
- From `Mode::VisualLine`: `v` returns `Action::EnterVisualMode`; `V`
  returns `Action::ExitToNormal`.
- A leading count followed by `v`/`V` (e.g. `3v`) does not produce a
  `Repeat`; the count is dropped and the mode toggles.

### `src/app/app_state/tests/visual_line.rs`

- Enter Visual Line at row 5, move down to row 7, press `v`: mode becomes
  `Mode::Visual`, anchor remains at row 5 (creating an annotation now uses
  charwise semantics with the original anchor column).
- Enter Visual at row 5, move to row 7 col 4, press `V`: mode becomes
  `Mode::VisualLine`, anchor row remains row 5; resulting annotation range
  spans full lines 5-7.
- From `Mode::VisualLine`, press `V`: mode becomes `Mode::Normal` and the
  anchor is cleared.

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```
