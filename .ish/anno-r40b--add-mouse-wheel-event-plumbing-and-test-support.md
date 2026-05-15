---
# anno-r40b
title: Add mouse wheel event plumbing and test support
status: completed
type: task
priority: high
tags:
- mouse
- input
- testing
created_at: 2026-05-15T15:26:42.930634Z
updated_at: 2026-05-15T16:08:28.797101Z
parent: anno-jml9
blocking:
- anno-banz
- anno-d4rv
---

## Context
This task establishes the mouse wheel plumbing needed by the rest of the feature.

Relevant findings:
- `src/app/mod.rs` currently polls terminal events and handles `Event::Key` only.
- The app appears not to enable mouse capture today.
- Existing tests are key-oriented via `src/app/app_state/test_harness.rs`, so wheel support will be hard to verify cleanly without a small testability extension.

Target v1 behavior constraints:
- vertical wheel only
- focused-context routing, not pointer hit-testing
- 1 navigation/scroll step per wheel event
- silent degradation if the terminal does not deliver wheel events

Likely code touchpoints:
- `src/main.rs` for terminal init/restore mouse capture setup if needed
- `src/app/mod.rs` for reading and forwarding `Event::Mouse`
- `src/app/app_state/*` for a mouse-event entrypoint parallel to `handle_key`
- `src/app/app_state/test_harness.rs` for wheel-event injection helpers in tests

## Dependencies
- Parent: focused-context mouse wheel scrolling feature

## Work
- Add the foundational mouse event path from terminal input to app state.
- Limit the new path to vertical wheel events for now; ignore unsupported mouse input.
- Introduce a testable `AppState` mouse-handling API and extend the test harness so future tasks can inject wheel events directly.
- Keep routing generic enough that follow-on tasks can map wheel events to existing actions without reworking the plumbing.

## Verification
- Add focused tests proving wheel events can be injected and consumed without using keyboard paths.
- Confirm unsupported mouse events are ignored by the new plumbing.
- Run:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo build --all-features`

## Implementation Notes
- Enabled `EnableMouseCapture`/`DisableMouseCapture` in `src/main.rs` so terminals that support wheel events can deliver them during app runtime.
- Updated `src/app/mod.rs` to forward `Event::Mouse` into app state while keeping non-wheel mouse input inert.
- Added `AppState::handle_mouse(MouseEvent) -> bool` plus a `VerticalWheelDirection` foundation for follow-on focused-context routing.
- Added `AppTestHarness` mouse helpers and tests covering supported vertical wheel input plus ignored unsupported mouse events.
- Wheel plumbing currently stops at event recognition; follow-up tasks should implement context-specific routing in `handle_vertical_wheel`.

## Completed Verification
- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
