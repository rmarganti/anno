---
# anno-jml9
title: Support focused-context mouse wheel scrolling
status: completed
type: feature
priority: high
tags:
- mouse
- input
- tui
created_at: 2026-05-15T15:26:42.532860Z
updated_at: 2026-05-15T16:21:11.178928Z
---

## Context
The requested feature is support for vertical mouse wheel scrolling in anno. The primary use case is the main document pane, but the intended v1 behavior also covers other existing scrolling contexts.

Agreed product decisions from discovery:
- use focused-context routing for v1; do not route by pointer hit-testing yet
- support vertical wheel only; ignore horizontal wheel and other mouse gestures
- each wheel event should perform exactly 1 existing navigation/scroll step
- mirror existing keyboard semantics exactly
- document pane: wheel up/down should behave like `k` / `j`
- Visual and Visual Line modes should inherit the exact existing `j` / `k` behavior
- help overlay: wheel should mirror existing help overlay scroll
- annotation inspect overlay: wheel should mirror existing inspect overlay scroll
- annotation list mode: wheel should move selection up/down like keyboard nav
- ignore wheel in insert/input-box, command, search, and confirm-dialog contexts
- if the focused context cannot move further, wheel should no-op and never fall through
- silently degrade when the terminal does not deliver wheel events
- document the feature in README and in-app help

Codebase findings that matter:
- `src/app/mod.rs` currently reads `Event::Key` only
- there is no visible mouse capture enable/disable today
- document movement is cursor-driven via existing `Action` handling in `DocumentViewState`
- help and annotation inspect overlays already have explicit scroll state and scroll actions
- the annotation list panel currently auto-scrolls to keep selection visible, so wheel should reuse selection motion rather than invent independent panel scrolling

## Dependencies
None.

## Work
- Break the feature into the smallest independently verifiable tasks.
- Preserve the v1 focused-context design; do not mix in pointer hit-testing.
- Ensure every child ish contains enough implementation context, likely files, and verification guidance for a future worker.
- Keep parent/child hierarchy and dependency edges both accurate.

## Verification
- `ish roadmap` shows a coherent hierarchy and dependency graph for the wheel-scrolling work.
- `ish check` passes.

## Implementation Notes
- Completed the feature via four child tasks: mouse event plumbing (`anno-r40b`), overlay/modal routing (`anno-d4rv`), document and annotation-list routing (`anno-banz`), and user-facing documentation (`anno-e1jz`).
- Final shipped v1 behavior is focused-context vertical wheel support only: document navigation mirrors `j`/`k`, annotation list movement mirrors its existing selection navigation, help and annotation inspect reuse their existing scroll helpers, and insert/command/search/confirm-dialog contexts remain inert.
- Mouse-wheel parity and no-fallthrough behavior are now covered in `src/app/app_state/tests/mouse.rs` and `src/app/app_state/tests/overlays.rs`; README and in-app help are aligned on the supported contexts and limitations.

## Completed Verification
- `ish roadmap`
- `ish check`
- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
