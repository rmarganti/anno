---
# anno-e1jz
title: Document mouse wheel behavior in README and in-app help
status: todo
type: task
priority: normal
tags:
- mouse
- docs
- ux
created_at: 2026-05-15T15:26:43.411068Z
updated_at: 2026-05-15T15:26:43.411068Z
parent: anno-jml9
blocked_by:
- anno-banz
- anno-d4rv
---

## Context
Document the new mouse wheel support after the behavior lands.

Agreed v1 user-facing behavior:
- vertical wheel only
- focused-context routing
- wheel mirrors existing keyboard navigation/scroll semantics
- document pane, annotation list, help overlay, and annotation inspect overlay are supported
- wheel is intentionally ignored in insert/input, command, search, and confirm-dialog contexts
- environments that do not deliver mouse wheel events should silently degrade

Likely documentation touchpoints:
- `README.md`
- help content in `src/keybinds/help_content.rs`
- any overlay footer hints or related discoverability text that mention scrolling/navigation

## Dependencies
- Blocked by the document/list wheel task.
- Blocked by the overlay/modal wheel task.

## Work
- Update README documentation for mouse wheel support and scope.
- Update in-app help/discoverability text so users can learn the feature without reading commit history.
- Keep wording aligned with the shipped v1 behavior and its intentional limitations.

## Verification
- README and in-app help both mention wheel support consistently.
- Any changed help text remains accurate for supported and unsupported contexts.
- Run:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo build --all-features`
