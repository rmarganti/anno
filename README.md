# anno

A terminal-based TUI for annotating text files. Navigate documents with vim-style keybindings, select text, and create structured annotations (comments, deletions, replacements, insertions) that are exported as agent-friendly XML-like markup or JSON.

## Installation

```bash
# Clone and build from source
git clone https://github.com/rmarganti/anno.git
cd anno
cargo build --release

# The binary will be at target/release/anno
```

## Usage

```bash
# Annotate a file
anno document.md

# Export JSON to a file
anno --export-format json --output-file feedback.json document.md

# Pick a built-in theme and syntax override
anno --theme mocha --theme-mode dark --syntax rust notes.txt

# Set a display title for the status bar
anno --title "API review" document.md

# Pipe from stdin
cat document.md | anno

# Save annotations to a file
anno document.md > feedback.md
```

Anno also reads optional startup settings from `~/.config/anno/settings.json`. CLI flags override config values for `theme`, `theme-mode`, `syntax`, and `title`. When no explicit syntax is set, anno auto-detects from filenames and shebang-style first lines, then falls back to plain text.

## Themes And Startup Settings

Use `--theme`, `--theme-mode`, `--syntax`, and `--title` on the command line, or set the corresponding startup values in `~/.config/anno/settings.json`.

### CLI flags

- `--export-format <agent|json>` chooses the output format produced by `:q`. The default is `agent`.
- `--theme <NAME_OR_PATH>` picks either a built-in theme name or an explicit path to a `.tmTheme` file.
- `--theme-mode <auto|light|dark>` controls automatic built-in theme selection when no explicit theme is set.
- `--syntax <NAME_OR_EXTENSION>` overrides syntax highlighting detection.
- `--title <TEXT>` sets a display-only title in the status bar.
- `--output-file <PATH>` writes the exported annotations to a file instead of stdout after `:q`.
- Bare values like `mocha` or `neverforest` stay in built-in theme resolution; values with `.tmTheme`, `/`, `\\`, or `~/` are treated as file paths.

Examples:

```bash
# Use the dark Catppuccin default selected by theme mode
anno --theme-mode dark notes.md

# Use a built-in theme directly
anno --theme catppuccin-latte notes.md

# Use an external tmTheme file explicitly
anno --theme "~/.config/bat/themes/Catppuccin Mocha.tmTheme" notes.md
```

### settings.json schema

```json
{
  "theme": "catppuccin-mocha",
  "background": "default",
  "theme_mode": "dark",
  "syntax": "rust",
  "title": "API review",
  "app_theme": {
    "cursor": {
      "bg": "#112233"
    },
    "selection": {
      "underlined": true
    },
    "annotation": {
      "fg": "#abcdef"
    }
  }
}
```

Supported top-level keys:

- `theme`: built-in theme name or explicit `.tmTheme` path.
- `background`: `theme` or `default`. `default` uses the terminal's default background color for the main document surface.
- `theme_mode`: `auto`, `light`, or `dark`.
- `syntax`: syntax name, token, or extension.
- `title`: optional display-only title shown in the status bar.
- `app_theme`: optional document-overlay overrides for `cursor`, `selection`, and `annotation`.

The settings parser also accepts `themeMode` / `theme-mode` and `appTheme` / `app-theme` as aliases for the snake_case keys above.

`background` controls the primary document background independently of the resolved syntax theme.
Use `default` if you want anno to fall back to the terminal's default background instead of the
theme's RGB background.

`app_theme` is intentionally narrow: it only affects document overlays layered on top of the
resolved syntax theme. It does not override widget chrome such as the status bar, mode pill,
input box, borders, or titles.

Each supported `app_theme` section supports:

- `fg`: hex RGB color like `#abcdef`
- `bg`: hex RGB color like `#112233`
- `bold`: `true` or `false`
- `italic`: `true` or `false`
- `underlined`: `true` or `false`

### Built-in themes and defaults

Built-in themes are:

- `catppuccin-latte` (aliases include `latte` and `catppuccin latte`)
- `catppuccin-mocha` (aliases include `mocha` and `catppuccin mocha`)
- `neverforest`

Theme selection works like this:

- An explicit CLI theme wins over everything else.
- Otherwise, `settings.json.theme` is used.
- Otherwise, `theme_mode=light` chooses `catppuccin-latte`.
- Otherwise, `theme_mode=dark` chooses `catppuccin-mocha`.
- Otherwise, anno falls back to `neverforest`.
- If automatic Catppuccin resolution ever fails, anno also falls back to `neverforest`.

### Interoperability with bat themes

Anno can load compatible `.tmTheme` files by explicit path, which makes it work with theme files you already use with `bat`.

- Built-in theme names are anno-specific; bat theme names are not auto-resolved.
- Point anno at the actual theme file instead of the bat theme name.
- Relative, absolute, `~/...`, and explicit file-name paths like `custom.tmTheme` are treated as paths.
- A name like `Catppuccin Mocha.tmTheme` counts as a path even without a directory prefix.

Example:

```bash
anno --theme "$(bat --config-dir)/themes/Catppuccin Mocha.tmTheme" notes.rs
```

On exit with `:q`, annotations are exported in the configured format. The default is `agent`, which prints XML-like markup to stdout unless `--output-file` is set. Use `:q!` to quit without output.

## Pi Extension Package

This repository also includes a Pi extension for running anno reviews directly from Pi.

For installation, usage, requirements, and limitations, see [`pi/anno-review/README.md`](pi/anno-review/README.md).

## Modes

anno uses vim-inspired modal editing:

| Mode                | Purpose                          | Enter          | Exit      |
| ------------------- | -------------------------------- | -------------- | --------- |
| **Normal**          | Navigate the document            | Startup default | —         |
| **Visual**          | Select text for annotations      | `v`            | `Esc`     |
| **Insert**          | Enter annotation text            | Annotation flow | `Ctrl-S` or `Esc` |
| **Annotation List** | Browse existing annotations      | `Tab`          | `Esc`     |
| **Command**         | Run quit commands                | `:`            | `Esc`     |

## Help Overlay

Press `?` to toggle the in-app help overlay. It shows the same global bindings, mode-specific keys, and commands documented below. While the overlay is open, `?`, `Esc`, and `q` all close it.

Numeric prefixes repeat supported navigation in Normal mode, Visual mode, the annotation list, and scrollable overlays. For example, `2j`, `3w`, `4]a`, and `10j` repeat the existing navigation action. Bare `0` keeps its usual line-start behavior and only contributes to a count after a leading digit. Counted mutation commands such as `4d` and `5dd` are intentionally unsupported.

## Keybindings

### Global

| Key      | Action                      |
| -------- | --------------------------- |
| `?`      | Toggle help                 |
| `:q`     | Quit                        |
| `:q!`    | Quit without saving output  |
| `Ctrl-C` | Force quit                  |
| `W`      | Toggle word wrap            |
| `count` + supported navigation | Repeat supported navigation (`2j`, `3w`, `4]a`, `10j`) |
| bare `0` | Move to line start unless extending an existing count |
| `4d`, `5dd` | Unsupported counted mutation commands |
| `Tab`    | Toggle annotation panel focus |

### Normal Mode

| Key         | Action                            |
| ----------- | --------------------------------- |
| `h/j/k/l`   | Move cursor                       |
| `w/b/e`     | Move by word                      |
| `0/$`       | Move to line start/end            |
| `gg/G`      | Move to document top/bottom       |
| `Ctrl-d/u`  | Move half page down/up            |
| `Ctrl-f/b`  | Move full page down/up            |
| `v`         | Enter visual mode                 |
| `i`         | Create insertion annotation       |
| `gc`        | Create global comment annotation  |
| `]a/[a`     | Jump to next/previous annotation  |
| `Esc`       | Hide annotation panel             |

### Visual Mode

| Key       | Action                         |
| --------- | ------------------------------ |
| `h/j/k/l` | Extend selection               |
| `w/b/e`   | Extend selection by word       |
| `0/$`     | Extend selection to line start/end |
| `d`       | Create deletion annotation     |
| `c`       | Create comment annotation      |
| `r`       | Create replacement annotation  |
| `Esc`     | Cancel selection               |

### Insert Mode

| Key      | Action         |
| -------- | -------------- |
| `Ctrl-S` | Confirm input  |
| `Esc`    | Cancel input   |

### Annotation List

| Key     | Action                        |
| ------- | ----------------------------- |
| `j/k`   | Move selection                |
| `Space` | Inspect selected annotation   |
| `Up/Down` | Scroll inspect text         |
| `PgUp/PgDn` | Page inspect text         |
| `Ctrl-u/d` | Page inspect text          |
| `Enter` | Jump to selected annotation   |
| `Tab`   | Unfocus annotation panel      |
| `dd`    | Delete selected annotation    |
| `Esc`   | Hide annotation panel         |

Deleting an annotation opens a confirmation dialog. Press `y` or `Enter` to confirm, or `n` or `Esc` to cancel.

### Command Mode

| Key    | Action          |
| ------ | --------------- |
| `:q`   | Quit            |
| `:q!`  | Force quit      |
| `Esc`  | Cancel command  |

## Annotation Types

| Type               | How to Create                                           |
| ------------------ | ------------------------------------------------------- |
| **Deletion**       | Select text in Visual mode, press `d`                   |
| **Comment**        | Select text in Visual mode, press `c`, type comment     |
| **Replacement**    | Select text in Visual mode, press `r`, type replacement |
| **Insertion**      | In Normal mode, press `i`, type text to insert          |
| **Global Comment** | In Normal mode, press `gc`, type comment                |

## Output Formats

Anno supports two export formats when quitting with `:q`:

- `agent` (default): structured XML-like output designed for LLM coding agents.
- `json`: structured JSON for programmatic tooling.

### `agent` example

```xml
<annotations file="path/to/file.md" total="5">
The reviewer left 5 annotations on this document.

<delete line="3">
selected text to remove
</delete>

<comment line="8">
This needs more detail.
</comment>

<replace lines="12-14">
<original>
old text
</original>
<replacement>
new text
</replacement>
</replace>

<insert line="20">
new content to insert here
</insert>

<comment>
Global comment not tied to any specific location.
</comment>

</annotations>
```

When the source is piped from stdin, the opening tag uses `source="stdin"` instead of `file="..."`.

### `json` example

```json
{
  "source": "path/to/file.md",
  "total": 2,
  "annotations": [
    {
      "type": "comment",
      "line": 8,
      "selected_text": "old sentence",
      "text": "This needs more detail."
    },
    {
      "type": "global_comment",
      "text": "Overall structure looks good."
    }
  ]
}
```

Annotations are ordered by their position in the document, with global comments appearing last.
