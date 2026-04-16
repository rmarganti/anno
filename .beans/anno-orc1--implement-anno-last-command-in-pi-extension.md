---
# anno-orc1
title: Implement /anno-last command in Pi extension
status: completed
type: feature
priority: normal
created_at: 2026-04-16T17:31:59Z
updated_at: 2026-04-16T18:06:49Z
---

Add an /anno-last slash command to the anno-review Pi extension that annotates the last assistant message. This mirrors Plannotators `/plannotator-last` but uses annos TUI instead of a browser UI. The command extracts the last assistant message from session history, writes it to a temp file, launches anno, and sends the annotation feedback back as a user message to trigger a new agent turn. No changes to annos Rust code are needed.

## Summary of Changes

- Added `/anno-last` to the Pi anno review extension. The command finds the latest assistant message on the active branch, writes it to a temporary markdown file, opens `anno`, and sends the exported annotations back into the conversation as either an immediate user message or a follow-up.
- Updated `pi/anno-review/README.md` with `/anno-last` usage and behavior notes.
- Verified the repository with `cargo fmt --all -- --check`, `cargo test --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build --all-features`.

## Notes for Future Workers

- `/anno-last` currently imports only `text` blocks from the most recent assistant message and ignores non-text assistant content.
- The command uses `ctx.sessionManager.getBranch()` rather than `getEntries()` so it tracks the active branch tip in forked session histories.
