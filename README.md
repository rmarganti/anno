# anno

A terminal-based TUI for annotating text files. Navigate documents with vim-style keybindings, select text, and create structured annotations (comments, deletions, replacements, insertions) that are exported as markdown.

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

# Pick a built-in theme and syntax override
anno --theme mocha --theme-mode dark --syntax rust notes.txt

# Pipe from stdin
cat document.md | anno

# Save annotations to a file
anno document.md > feedback.md
```

Anno also reads optional startup settings from `~/.config/anno/settings.json`. CLI flags override config values. When no explicit syntax is set, anno auto-detects from filenames and shebang-style first lines, then falls back to plain text.

## Themes And Startup Settings

Use `--theme`, `--theme-mode`, and `--syntax` on the command line, or set the same values in `~/.config/anno/settings.json`.

### CLI flags

- `--theme <NAME_OR_PATH>` picks either a built-in theme name or an explicit path to a `.tmTheme` file.
- `--theme-mode <auto|light|dark>` controls automatic built-in theme selection when no explicit theme is set.
- `--syntax <NAME_OR_EXTENSION>` overrides syntax highlighting detection.
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

On exit with `:q`, annotations are printed to stdout as structured markdown. Use `:q!` to quit without output.

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

Press `?` to toggle the in-app help overlay. It shows the same global bindings, mode-specific keys, and commands documented below.

## Keybindings

### Global

| Key      | Action                      |
| -------- | --------------------------- |
| `?`      | Toggle help                 |
| `:q`     | Quit                        |
| `:q!`    | Quit without saving output  |
| `Ctrl-C` | Force quit                  |
| `W`      | Toggle word wrap            |
| `Tab`    | Toggle annotation list      |

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
| `Enter` | Jump to selected annotation   |
| `dd`    | Delete selected annotation    |
| `Esc`   | Exit annotation list          |

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

## Output Format

Annotations are exported as markdown when quitting with `:q`. Example output:

````markdown
# Plan Feedback

I've reviewed this plan and have 3 pieces of feedback:

## 1. Remove this

```
selected text to remove
```

> I don't want this in the plan.

## 2. Feedback on: "some text"

> This needs more detail.

## 3. Change this

**From:**

```
old text
```

**To:**

```
new text
```

---
````

Annotations are ordered by their position in the document, with global comments appearing last.
