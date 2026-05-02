# Plan: Visual Line Mode (`V`)

This document is the shared, evolving context for the `V` (vim Visual Line
Mode) feature. Every ish in this work tree links back to this file. **Read
it before starting an ish, and append/update notes as you make decisions or
discover surprises so future ishes inherit the latest understanding.**

## Goal

Add a vim-faithful Visual Line Mode to anno, accessible by pressing `V` from
Normal mode. While in Visual Line Mode, motions extend the selection a full
line at a time (anchor and cursor row are both expanded to whole lines), and
annotation creation (`d`, `c`, `r`) operates on the line-wise selection.

## Vim-Faithful Design Decisions (locked in)

- **Mode toggling**:
  - `v` from Normal → charwise Visual (existing).
  - `V` from Normal → linewise Visual Line.
  - `v` from Visual Line → switch to charwise Visual (keep anchor position).
  - `V` from Visual → switch to linewise Visual Line (keep anchor position).
  - `V` from Visual Line → exit to Normal (vim toggles same-mode key off).
  - `v` from Visual → exit to Normal (existing behavior, unchanged).
- **`[count]V` from Normal**: enter Visual Line and extend the selection
  downward by `count - 1` rows from the anchor row.
- **Selection snapping**: in Visual Line, the active selection always covers
  the full first line through the full last line, regardless of where the
  cursor/anchor columns are. The `TextRange` produced for annotations starts
  at `col=0` of the top row and ends at the last char index of the bottom
  row.
- **`selected_text` semantics**: linewise yank shape; the captured text ends
  with a trailing `\n` (matching vim linewise yank behavior).
- **Status pill label**: `VISUAL LINE` (uses the same color scheme as the
  existing `VISUAL` pill).
- **`o` (anchor swap)**: out of scope for this feature in both Visual and
  Visual Line. May be added later as a separate piece of work.
- **All other Visual-mode bindings** (motions, `d`/`c`/`r`/`Esc`, char-search
  `f F t T`, `;` `,`, search `/ ? n N`, `gj gk`, counts on motions) behave
  the same in Visual Line.

## Architecture Touchpoints

| Concern | File |
|---|---|
| Mode enum | [src/keybinds/mode.rs](src/keybinds/mode.rs) |
| Action enum + key handler | [src/keybinds/handler.rs](src/keybinds/handler.rs) |
| Help content | [src/keybinds/help_content.rs](src/keybinds/help_content.rs) |
| Visual anchor + selection | [src/tui/document_view.rs](src/tui/document_view.rs) |
| Selection text helper | [src/tui/selection.rs](src/tui/selection.rs) |
| Renderer (line/cursor highlight) | [src/tui/renderer.rs](src/tui/renderer.rs) |
| Status bar pill | [src/tui/status_bar.rs](src/tui/status_bar.rs) |
| App-state mode dispatch | [src/app/app_state/mod.rs](src/app/app_state/mod.rs) |
| App-state core types | [src/app/app_state/core.rs](src/app/app_state/core.rs) |
| App-state predicates / `is_visual` | [src/app/mod.rs](src/app/mod.rs) |
| App-state search preservation | [src/app/app_state/search.rs](src/app/app_state/search.rs) |
| Existing visual-mode tests | [src/app/app_state/tests/modes.rs](src/app/app_state/tests/modes.rs), [src/app/app_state/tests/search.rs](src/app/app_state/tests/search.rs), [src/app/app_state/tests/navigation.rs](src/app/app_state/tests/navigation.rs) |
| README | [README.md](README.md) |

## Implementation Strategy

The work is sliced into atomic ishes that compose bottom-up. Each ish
includes its own focused tests; an integration test pass happens implicitly
through the verification loop at every step.

1. **Foundation** — add the `VisualLine` mode variant, add the
   `EnterVisualLineMode` action, route the mode through `KeybindHandler`,
   bind `V` in Normal, and add the new `handle_visual_line` (mirrors
   `handle_visual` for now; `v`/`V` toggles handled later).
2. **Selection model** — extend `DocumentViewState` with a per-anchor "kind"
   (Char vs Line); make `take_visual_selection` snap the range and append
   the trailing newline when linewise; expand the rendered selection rect
   to full rows when linewise.
3. **App-state integration** — handle the new action in `AppState`, update
   every predicate that checks `Mode::Visual` to also accept
   `Mode::VisualLine` (allowed-action gates, `is_visual` rendering flag,
   search-confirm mode preservation), and route annotation-creation through
   the linewise path.
4. **Mode toggling** — implement `v ↔ V` swapping kinds (keeping the
   existing anchor) and same-key-exits-to-Normal semantics.
5. **`[count]V`** — count-prefix support: entering VisualLine pre-extends
   the cursor `count - 1` rows below the anchor.
6. **UI polish** — status pill `VISUAL LINE` and help overlay updates.
7. **Docs** — README modes table, Normal-mode binding row, and a Visual
   Line section.

## Selection Data Shape (proposed)

Replace the bare `visual_anchor: Option<CursorPosition>` in
`DocumentViewState` with something like:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualKind { Char, Line }

struct VisualAnchor {
    pos: CursorPosition,
    kind: VisualKind,
}

visual_anchor: Option<VisualAnchor>,
```

When promoting Visual → VisualLine (or demoting), keep `pos`, change `kind`.
When exiting to Normal, clear it entirely (existing `clear_visual` covers
this).

## Linewise Range Semantics

For anchor row `a` and cursor row `c` with `top = min(a, c)` and
`bot = max(a, c)`:

- `range.start = TextPosition { line: top, column: 0 }`
- `range.end   = TextPosition { line: bot, column: last_char_index_of(bot) }`
- `selected_text = lines[top..=bot].join("\n") + "\n"`

If `bot` is an empty line, treat `last_char_index_of(bot)` as `0` for
consistency with how charwise selection currently handles empty lines.

## Verification (gating every ish)

Run from the repo root before marking any ish complete:

```bash
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

If you change behavior that other ishes will rely on, update the relevant
section of this file before closing your ish so the next agent picks up the
new shape.

## Open Notes

_Add timestamped notes here as decisions, surprises, or follow-ups come up._

- 2026-05-01 (anno-8o31, foundation): Adding `Mode::VisualLine` forced exhaustive
  matches outside the keybind layer to handle the new variant. To stay
  scoped to plumbing, the new variant was added as a placeholder reusing
  `Mode::Visual` behavior in two places:
  - [src/tui/status_bar.rs](src/tui/status_bar.rs) — pill label and hint
    fall through with `Mode::Visual | Mode::VisualLine`. The UI-polish ish
    (`anno-vzmd`) must change the pill to ` VISUAL LINE ` (and adjust the
    label-width-dependent layout in the `mode_pill_*` tests if any).
  - [src/app/app_state/mod.rs](src/app/app_state/mod.rs) —
    `is_repeatable_navigation_action` accepts `Mode::VisualLine` in the
    same arm as `Mode::Visual` so counted motions flow through
    `dispatch_repeat`. The app-state ish (`anno-7ejd`) must extend the
    other `Mode::Visual` predicates the same way (allowed-action gates,
    `is_visual` rendering flag, search-confirm mode preservation).
- `Action::EnterVisualLineMode` is in `Action::supports_count` so
  `[count]V` already wraps in `Action::Repeat`. The actual count
  pre-extension behavior is implemented in `anno-6x5o`.
- `handle_visual_line` mirrors `handle_visual` exactly. `v` and `V` are
  explicit no-ops there (with `clear_pending`) so the toggle ish
  (`anno-1am6`) can replace them with real behavior without touching
  the fallthrough branch.
- 2026-05-02 (anno-xrp9, selection model): `DocumentViewState`'s
  `visual_anchor` now stores `VisualAnchor { pos: CursorPosition, kind:
  VisualKind }` (both private to [src/tui/document_view.rs](src/tui/document_view.rs)).
  Charwise vs linewise routing happens via the `kind` field — the
  `Selection` type in [src/tui/selection.rs](src/tui/selection.rs) was
  left unchanged. `take_visual_selection` and `render_document_view` both
  go through a new private `snap_linewise(start, end)` helper to compute
  the full-row range; `take_visual_selection` then appends a trailing
  `\n` to the linewise yank. Empty target lines clamp `end.col` to `0`,
  matching existing charwise behavior. Future toggling work
  (`anno-1am6`) can promote/demote a selection in place by mutating
  `visual_anchor.as_mut().map(|a| a.kind = …)` without touching `pos`.
- 2026-05-02 (anno-7ejd, app-state integration): `AppState` now promotes
  `Action::EnterVisualLineMode` into `Mode::VisualLine` in the same
  document-view dispatch path that already handled `EnterVisualMode`, so
  `V` now activates linewise selection end-to-end for motions and
  `d`/`c`/`r`. Search-mode preservation required no special branching:
  `enter_search_mode()` already snapshots `self.mode`, so confirming or
  cancelling `/` and `?` from Visual Line returns to `Mode::VisualLine`
  automatically. The rendering gate in [src/app/mod.rs](src/app/mod.rs)
  now uses a shared `is_visual_mode()` helper so future Visual-derived
  modes only need one predicate update.
- 2026-05-02 (anno-1am6, toggle semantics): `DocumentViewState` gained a
  crate-visible `set_visual_kind(VisualKind)` helper so app-state can flip
  between charwise and linewise selection without re-anchoring. `AppState`
  now uses that helper for both `EnterVisualMode` and
  `EnterVisualLineMode`, which preserves the original anchor position when
  toggling `v ↔ V` and still installs a fresh anchor when entering from
  Normal. In the keybind layer, `V` inside `Mode::Visual` and `v`/`V`
  inside `Mode::VisualLine` clear any pending count and dispatch the raw
  toggle/exit action instead of `Action::Repeat`, matching vim's
  non-counted mode-toggle behavior.
- 2026-05-02 (anno-6x5o, `[count]V`): `Action::Repeat {
  EnterVisualLineMode, count }` is now intercepted in
  [src/app/app_state/mod.rs](src/app/app_state/mod.rs) instead of going
  through the generic repeat loop. App-state dispatches one
  `EnterVisualLineMode`, then reuses repeated `MoveDown` actions for
  `count - 1`, which preserves the original anchor, inherits existing
  cursor clamping at EOF, and keeps counted linewise selection behavior
  aligned with normal motion semantics. Downstream docs/UI work should
  describe `3V` as "enter Visual Line once, then extend downward" rather
  than as a generic repeated mode switch.
- 2026-05-02 (anno-vzmd, UI polish):
  [src/tui/status_bar.rs](src/tui/status_bar.rs) now gives
  `Mode::VisualLine` its own ` VISUAL LINE ` pill while reusing the same
  styling as `VISUAL`; the existing single-row status layout had enough
  slack at 80 columns, so no abbreviation or layout change was needed.
  [src/keybinds/help_content.rs](src/keybinds/help_content.rs) chose a
  dedicated `Visual Line Mode` section instead of collapsing into a note
  under `Visual Mode`, so the README ish (`anno-n4c8`) should mirror that
  structure and include the explicit `V` Normal-mode entry.
- 2026-05-02 (anno-n4c8, docs): [README.md](README.md) now mirrors the
  shipped UI and behavior with a dedicated `Visual Line Mode` section,
  an explicit `V` row in Normal mode, a `V` toggle row in Visual mode,
  and `[count]V` called out in both the help-overlay prose and the mode-
  specific docs. Annotation-type instructions now say `Visual or Visual
  Line mode` so linewise annotation creation is documented end-to-end.
