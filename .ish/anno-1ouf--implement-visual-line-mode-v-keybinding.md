---
# anno-1ouf
title: Implement Visual Line Mode (V) keybinding
status: todo
type: epic
priority: normal
tags:
- visual-mode
- vim-bindings
created_at: 2026-05-02T02:40:56.479077Z
updated_at: 2026-05-02T02:40:56.479077Z
---

## Summary

Add a vim-faithful `V` (Visual Line Mode) keybinding to anno. Users press
`V` from Normal mode to start a linewise selection that always covers whole
lines. While active, all motions extend the selection a row at a time, and
annotation creation (`d`, `c`, `r`) operates on the resulting line range.

## Shared Plan

All decisions and architecture context live in
[plan.md](../plan.md). **Read it before starting any child ish, and update
it whenever you make a decision or discover a surprise that future ishes
need to inherit.**

## Scope (locked-in vim defaults)

- `V` from Normal → Visual Line.
- `v` ↔ `V` toggling between charwise Visual and linewise Visual Line keeps
  the anchor; pressing the same key as the current mode exits to Normal.
- `[count]V` enters Visual Line and extends the selection downward by
  `count - 1` rows from the anchor.
- Linewise `TextRange` snaps to `col=0` of the top row and the last char
  index of the bottom row; `selected_text` ends with a trailing `\n`.
- All other Visual-mode bindings (motions, char-search, search, `gj/gk`,
  `d/c/r/Esc`, counts) behave identically in Visual Line.
- Status pill displays `VISUAL LINE`.

## Out Of Scope

- `o` (swap anchor and cursor). Not currently in charwise Visual either; can
  be added later as a separate effort.
- Any non-linewise visual variants (e.g. `Ctrl-V` block-visual).

## Children

This epic decomposes into atomic tasks; see the linked children. Recommended
execution order follows the blocking graph:

1. Mode + Action wiring (foundation).
2. Linewise selection model in `DocumentViewState`.
3. App-state integration.
4. `v ↔ V` mode toggling.
5. `[count]V` count-prefix support.
6. UI polish (status pill + help overlay).
7. README documentation update.

## Verification

The epic is complete when every child ish is complete and the full
verification loop passes from the repo root:

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

Smoke test from a real session: open a multi-line file with
`cargo run -- README.md`, press `V`, move with `j`/`k`, confirm entire rows
highlight, press `d`, confirm the deletion annotation covers full lines.
