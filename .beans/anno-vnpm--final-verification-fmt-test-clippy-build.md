---
# anno-vnpm
title: 'Final verification: fmt, test, clippy, build'
status: completed
type: task
priority: normal
created_at: 2026-04-16T17:32:42Z
updated_at: 2026-04-16T18:06:24Z
parent: anno-orc1
blocked_by:
    - anno-xxc4
---

## What

Run the full verification suite required by AGENTS.md.

## Commands (all from repo root)

1. `cargo fmt --all -- --check`
2. `cargo test --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo build --all-features`

## Checklist

- [x] cargo fmt passes
- [x] cargo test passes
- [x] cargo clippy passes
- [x] cargo build passes

## Summary of Changes

- Ran all required repo validations from `AGENTS.md` successfully: `cargo fmt --all -- --check`, `cargo test --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build --all-features`.
