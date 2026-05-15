# Plan

## Mouse wheel scrolling feature
- Parent ish: `anno-jml9` — support focused-context mouse wheel scrolling.
- Selected next task: `anno-banz` because it was the highest-value ready task after the plumbing and overlay routing landed, and it unlocked the remaining user-facing documentation work.

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

## Completed in `anno-banz`
- Extended `AppState::handle_vertical_wheel` to reuse existing `Action::MoveUp` / `Action::MoveDown` dispatch for document navigation in Normal, Visual, and Visual Line modes.
- Routed wheel input in Annotation List mode through the existing list-selection movement path, preserving auto-scroll-to-selection behavior.
- Kept the overlay/modal precedence from `anno-d4rv`: help and inspect still win first, and insert/command/search/confirm-dialog remain inert for wheel input.
- Added mouse-wheel regression tests proving parity with keyboard navigation in Normal, Visual, Visual Line, and Annotation List contexts.
- Added annotation-list boundary tests proving wheel events no-op at the ends instead of falling through to document navigation.

## Notes for follow-on tasks after `anno-banz`
- The only remaining wheel-scrolling child ish is `anno-e1jz`, which is now unblocked and should focus on README/help text only.
- Mouse routing parity is covered in `src/app/app_state/tests/mouse.rs`; future behavior tweaks should keep those parity assertions aligned with the keyboard paths rather than duplicating movement logic.
- `handle_vertical_wheel` now centralizes all v1 wheel-context routing, so any future pointer-hit-testing work should likely branch from there instead of bypassing it.

## Completed in `anno-e1jz`
- Documented vertical mouse wheel support in `README.md`, including focused-context routing, supported contexts, ignored contexts, and silent degradation when terminals do not emit wheel events.
- Added explicit `Wheel ↑/↓` discoverability rows to the README keybinding tables for Normal, Visual, Visual Line, and Annotation List contexts plus a global overview row.
- Updated `src/keybinds/help_content.rs` so the in-app help overlay advertises the same wheel behavior as the README.
- Adjusted `src/tui/help_overlay.rs` tests to use taller render fixtures now that the help overlay contains additional wheel documentation.

## Notes for future workers after `anno-e1jz`
- README and in-app help are now intentionally aligned on wheel wording; update both together if wheel semantics change.
- The help overlay render tests are somewhat height-sensitive because they assert on visible sections; if more help rows are added later, prefer raising fixture heights instead of weakening the assertions.
- With `anno-e1jz` done, the mouse-wheel child tasks under `anno-jml9` are complete, so the parent feature ish is ready for wrap-up.

## Validation
- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
