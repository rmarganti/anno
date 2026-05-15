---
# anno-banz
title: Implement wheel navigation for document and annotation list contexts
status: todo
type: task
priority: high
tags:
- mouse
- input
- navigation
created_at: 2026-05-15T15:26:43.088755Z
updated_at: 2026-05-15T15:26:43.571578Z
parent: anno-jml9
blocking:
- anno-e1jz
blocked_by:
- anno-r40b
---

## Context
Implement mouse wheel behavior for the main navigation contexts after the mouse-input plumbing exists.

Agreed v1 semantics:
- focused-context routing
- wheel up/down should mirror existing keyboard navigation exactly
- document pane in Normal mode: behave like `k` / `j`
- document pane in Visual / Visual Line mode: behave exactly like existing `k` / `j`, including selection changes
- annotation list mode: wheel should move selection up/down like the existing list navigation keys
- one step per wheel event
- at top/bottom, the event should no-op rather than fall through elsewhere

Codebase findings:
- document navigation already exists as `Action`-driven movement in `DocumentViewState`
- annotation list behavior should reuse selection movement and existing auto-scroll-to-selection logic rather than adding independent panel scrolling

Likely code touchpoints:
- `src/app/app_state/mod.rs`
- `src/app/app_state/panel_state.rs` and/or related dispatch code
- `src/tui/document_view.rs`
- tests under `src/app/app_state/tests/`

## Dependencies
- Blocked by the mouse wheel plumbing task.

## Work
- Route wheel events to existing up/down navigation for the document in Normal, Visual, and Visual Line modes.
- Route wheel events to existing annotation-list selection movement when the app is in Annotation List mode.
- Reuse existing actions/behavior rather than introducing parallel movement semantics.
- Add tests that demonstrate exact parity with existing keyboard navigation in these contexts.

## Verification
- Tests cover document wheel navigation in Normal and Visual/Visual Line modes.
- Tests cover annotation list selection movement and boundary no-op behavior.
- Run:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo build --all-features`
