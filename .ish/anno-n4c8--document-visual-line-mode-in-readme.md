---
# anno-n4c8
title: Document Visual Line Mode in README
status: todo
type: task
priority: low
tags:
- visual-mode
- vim-bindings
- docs
created_at: 2026-05-02T02:43:31.573806Z
updated_at: 2026-05-02T02:43:31.573806Z
parent: anno-1ouf
blocked_by:
- anno-1am6
- anno-6x5o
- anno-vzmd
---

## Goal

Update [README.md](../README.md) so the documented modes, keybindings, and
behavior match the implemented Visual Line Mode. This ish lands last so it
captures the actual shipped behavior (including any divergences noted in
`plan.md`).

## Shared Plan

Read [plan.md](../plan.md) first — it has the canonical decisions and any
implementation surprises captured by earlier ishes. Mirror them exactly.

## Blocked By

- `anno-1am6` (`v ↔ V` toggling).
- `anno-6x5o` (`[count]V`).
- `anno-vzmd` (status pill + help copy that the README mirrors).

## Changes (in `README.md`)

1. **Modes table** (around line 161): add a `Visual Line` row.
   - Purpose: "Select whole lines for annotations".
   - Enter: `V`.
   - Exit: `Esc` or `V`.

2. **Help Overlay paragraph** (around line 173): mention that Visual Line
   bindings appear under their own section (or a note pointing to Visual
   Mode), depending on which pattern `anno-vzmd` chose. Match the help
   overlay copy.

3. **Numeric prefixes paragraph** (around line 177): list `[count]V` as a
   supported counted action with an example like `3V` selects three lines.

4. **Normal Mode keybindings table** (around line 199): add a row
   `V` → "Enter visual line mode".

5. **Visual Mode keybindings table** (around line 222): add a row at the
   bottom: `V` → "Switch to visual line mode".

6. **New Visual Line Mode section** immediately after the Visual Mode
   table:
   - Same motions and selection-extension keys.
   - `d` / `c` / `r` create the same annotations, but always over whole
     lines.
   - `v` switches back to charwise Visual.
   - `V` or `Esc` exits to Normal.
   - `[count]V` from Normal selects `count` lines.

7. **Annotation Types table** (around line 277): if any wording about
   "Select text in Visual mode" should be broadened to "Select text in
   Visual or Visual Line mode", update those rows accordingly.

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

Spot-check the README rendering locally if convenient (e.g. `glow` or your
markdown previewer of choice). Confirm tables align and links are valid.

## Closing The Epic

Once this ish completes successfully, mark the epic `anno-1ouf` as
`completed` (`ish update anno-1ouf -s completed`).
