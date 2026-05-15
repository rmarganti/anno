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

## Completed in `anno-d4rv`
- Routed wheel events through overlay precedence before any future document/list handling: help overlay first, confirm dialog ignore, then annotation inspect overlay.
- Mapped wheel up/down to the existing single-step help and annotation inspect scroll helpers instead of introducing parallel scroll behavior.
- Explicitly kept wheel input inert in confirm-dialog, insert, command, and search contexts.
- Added mouse-wheel tests covering help/inspect scrolling, ignored contexts, and boundary no-fallthrough behavior.

## Notes for follow-on tasks after `anno-d4rv`
- `handle_vertical_wheel` now owns focused-context precedence for wheel routing, so document/list support in `anno-banz` should extend the existing `else if` chain after overlay/modal checks.
- Overlay wheel behavior reuses `scroll_help_*` and `scroll_annotation_inspect_*`; future work should keep reusing existing movement/scroll helpers rather than inventing mouse-only semantics.
- Boundary behavior is now covered by mouse-specific tests in `src/app/app_state/tests/overlays.rs`, which should be a good pattern for document/list no-fallthrough tests.

## Validation
- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
