---
# anno-vzmd
title: Status bar pill and help overlay for Visual Line Mode
status: completed
type: task
priority: normal
tags:
- visual-mode
- vim-bindings
- ui
created_at: 2026-05-02T02:43:13.039447Z
updated_at: 2026-05-02T13:35:06.303073Z
parent: anno-1ouf
blocked_by:
- anno-8o31
---

## Goal

Surface Visual Line Mode in the UI: render a `VISUAL LINE` pill in the
status bar when active, and document the new bindings in the in-app help
overlay (`H`).

## Shared Plan

Read [plan.md](../plan.md) before starting and update it if you change
copy that the README ish should mirror.

## Blocked By

- `anno-8o31` (provides `Mode::VisualLine`).

## Changes

### `src/tui/status_bar.rs`

- Add a `Mode::VisualLine` arm wherever the existing mode pill is computed
  for `Mode::Visual`. The label is `VISUAL LINE`. Reuse the same color
  scheme as the existing `VISUAL` pill (vim shows them in the same color);
  if status_bar internally uses a width / fixed-width pill, make sure
  `VISUAL LINE` does not overflow the layout — adjust the pill area or use
  a shorter abbreviation only if there is no other option (and note the
  decision in `plan.md`).

### `src/keybinds/help_content.rs`

- Add a row in the Normal-mode section: `V` → "Enter visual line mode".
- Add a new "Visual Line Mode" section that lists:
  - `h/j/k/l`, `w/b/e`, `0/$`, `f/F/t/T`, `;`/`,`, `/`/`?`, `n`/`N` —
    "Extend selection by line/motion".
  - `d` — "Create deletion annotation".
  - `c` — "Create comment annotation".
  - `r` — "Create replacement annotation".
  - `v` — "Switch to charwise visual".
  - `V` — "Exit visual line mode".
  - `Esc` — "Cancel selection".
- If the help renderer collapses identical sections, you can instead add a
  one-line note under the existing Visual Mode section like "All Visual
  bindings also apply in Visual Line Mode (entered with `V`); use `v` to
  switch to charwise selection."  — pick whichever pattern matches the
  existing help style. Note the choice in `plan.md` so the README ish can
  match.

## Tests

### `src/tui/status_bar.rs`

- Add a unit / render test asserting the pill text equals `VISUAL LINE`
  when `Mode::VisualLine` is active.

### `src/keybinds/help_content.rs`

- Test (or snapshot, if a snapshot pattern exists) verifying a new entry
  containing `V` and "visual line" appears in the help sections returned
  by `help_sections()`.

## Verification

```
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

## Implementation Notes

- `src/tui/status_bar.rs` now renders a dedicated ` VISUAL LINE ` pill for
  `Mode::VisualLine` while keeping the existing visual-mode hint text and
  styling.
- `src/keybinds/help_content.rs` adds a dedicated `Visual Line Mode`
  section instead of collapsing the bindings into a note under `Visual
  Mode`; the Normal-mode section also now lists `V` explicitly.
- The hard-coded help-section order test was updated to keep `Visual Line
  Mode` adjacent to `Visual Mode`, which the README follow-up should mirror.

## Verification Results

- `cargo fmt --all -- --check` ✓
- `cargo test --all-features` ✓
- `cargo clippy --all-targets --all-features -- -D warnings` ✓
- `cargo build --all-features` ✓
