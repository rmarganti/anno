---
# anno-d4rv
title: Implement wheel behavior for overlays and ignored modal contexts
status: todo
type: task
priority: high
tags:
- mouse
- input
- overlay
created_at: 2026-05-15T15:26:43.246153Z
updated_at: 2026-05-15T15:26:43.585978Z
parent: anno-jml9
blocking:
- anno-e1jz
blocked_by:
- anno-r40b
---

## Context
Implement mouse wheel behavior for overlay and modal contexts after the mouse-input plumbing exists.

Agreed v1 semantics:
- help overlay visible: wheel should mirror existing help scroll up/down
- annotation inspect overlay visible: wheel should mirror existing inspect overlay scroll up/down
- insert mode / annotation input box: ignore wheel
- command mode: ignore wheel
- search mode: ignore wheel
- confirm dialog: ignore wheel
- boundary conditions should no-op and never fall through to an underlying context

Codebase findings:
- `src/app/app_state/overlay_state.rs` already centralizes help and annotation-inspect overlay handling and scroll offsets
- confirm dialog handling already intercepts keyboard input before other contexts
- the new wheel behavior should align with the existing overlay-precedence model rather than inventing a new routing stack

Likely code touchpoints:
- `src/app/app_state/overlay_state.rs`
- `src/app/app_state/mod.rs`
- overlay-related tests in `src/app/app_state/tests/overlays.rs`
- any additional test modules needed for insert/command/search ignore behavior

## Dependencies
- Blocked by the mouse wheel plumbing task.

## Work
- Route wheel events to existing help overlay scroll behavior when help is visible.
- Route wheel events to existing annotation inspect overlay scroll behavior when inspect is visible.
- Ensure wheel events are ignored in input-taking and confirm-dialog contexts.
- Preserve the existing focused-context precedence order and no-fallthrough behavior.
- Add tests covering overlay routing, ignored contexts, and boundary no-op behavior.

## Verification
- Tests prove help and inspect overlays scroll correctly from wheel input.
- Tests prove insert, command, search, and confirm-dialog contexts ignore wheel input.
- Tests prove no fallthrough occurs when an overlay is at its boundary.
- Run:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo build --all-features`
