---
# anno-8o31
title: Add Mode::VisualLine variant and EnterVisualLineMode action wiring
status: completed
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:41:16.772473Z
updated_at: 2026-05-02T02:52:39.138763Z
parent: anno-1ouf
---

## Goal

Lay the foundation for Visual Line Mode by adding the new mode variant, the
new action variant, and the keybind handler that routes them. No selection
or annotation behavior changes happen here — this is pure plumbing so the
follow-up ishes have something to wire into.

## Shared Plan

Read [plan.md](../plan.md) before starting and append notes to it as you
make decisions or discover surprises that downstream ishes need to inherit.

## Changes

### `src/keybinds/mode.rs`

- Add a new `VisualLine` variant to `Mode` (after `Visual`). Keep the
  default value as `Normal`.

### `src/keybinds/handler.rs`

- Add `Action::EnterVisualLineMode`.
- Mark `EnterVisualLineMode` in `Action::supports_count` so `[count]V`
  produces an `Action::Repeat { Box::new(EnterVisualLineMode), count }`
  (the count behavior itself is implemented in a later ish; this just lets
  the count plumbing flow through).
- In `handle_normal`, bind `(KeyCode::Char('V'), KeyModifiers::SHIFT)` →
  `Action::EnterVisualLineMode` via `self.finish_action(...)`.
- Add a new `handle_visual_line` function that, for now, mirrors
  `handle_visual` exactly (motions, char-search, search transitions, `gj`/
  `gk`, `d`/`c`/`r`, `Esc` → `ExitToNormal`). The `v ↔ V` toggle and
  same-key-exit behavior are added in a later ish — leave `v` and `V` in
  `handle_visual_line` unhandled (returning `Action::None`) for now so
  follow-up ishes can replace them with explicit toggles.
- Route `Mode::VisualLine` through `KeybindHandler::handle` to the new
  `handle_visual_line`.

## Tests (in `src/keybinds/handler.rs`)

Add a new `mod visual_line_mode_bindings { ... }` test block (or extend the
existing visual-mode tests) covering:

- `V` (shift) in Normal returns `Action::EnterVisualLineMode`.
- `[count]V` (e.g. `3` then `V`) wraps it in `Action::Repeat { count: 3 }`.
- `Mode::VisualLine` accepts `h/j/k/l`, `w/b/e`, `0/$`, `gg`/`G`, `gj`/`gk`,
  `f/F/t/T`, `;`/`,`, `/` `?` `n` `N`, `d`/`c`/`r`, and `Esc` exactly like
  `Mode::Visual`.
- `Mode::VisualLine` returns `Action::None` for `v` and `V` (placeholders
  until the toggle ish replaces them).

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Follow-Up

Once this is merged, downstream ishes (selection model, app-state
integration, `v ↔ V` toggle, `[count]V`, UI polish, docs) can proceed.
