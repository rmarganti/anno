# Plan

## Mouse wheel scrolling feature
- Parent ish: `anno-jml9` — support focused-context mouse wheel scrolling.
- Selected next task: `anno-r40b` because it is the foundational, unblocked prerequisite for the remaining wheel-routing tasks.

## Completed in `anno-r40b`
- Enabled terminal mouse capture during app runtime and disabled it during normal and panic cleanup.
- Extended the main event loop to forward `Event::Mouse` into `AppState`.
- Added `AppState::handle_mouse(MouseEvent) -> bool` as the testable mouse entrypoint.
- Limited plumbing to vertical wheel events only (`ScrollUp` / `ScrollDown`); all other mouse input is ignored.
- Added test harness helpers for injecting mouse events directly.
- Added focused tests proving vertical wheel events are accepted without disturbing pending keyboard parsing and that unsupported mouse events are ignored.

## Notes for follow-on tasks
- The wheel plumbing intentionally does not map scroll events to navigation yet; follow-up tasks should implement focused-context routing in `handle_vertical_wheel`.
- `handle_mouse` already returns whether an event was recognized as supported wheel input, which should help future routing tests.
- Current cleanup disables mouse capture both on the normal exit path and in the panic hook.

## Validation
- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
