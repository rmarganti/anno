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

Anno also reads optional startup settings from `~/.config/anno/settings.json`. Supported keys are `theme`, `theme_mode`, and `syntax`, with CLI flags taking precedence over config values. When no explicit syntax is set, anno auto-detects from filenames and shebang-style first lines, then falls back to plain text.

On exit with `:q`, annotations are printed to stdout as structured markdown. Use `:q!` to quit without output.

## Modes

anno uses vim-inspired modal editing:

| Mode                | Purpose                       | Enter         | Exit          |
| ------------------- | ----------------------------- | ------------- | ------------- |
| **Normal**          | Navigate the document         | —             | —             |
| **Visual**          | Select text for annotation    | `v`           | `Esc`         |
| **Insert**          | Type annotation text          | _(automatic)_ | `Esc`/`Enter` |
| **Annotation List** | Browse and manage annotations | `Tab`         | `Tab`/`Esc`   |
| **Command**         | Execute commands              | `:`           | `Esc`         |

## Keybindings

### Normal Mode

| Key                 | Action                     |
| ------------------- | -------------------------- |
| `h` `j` `k` `l`     | Move left/down/up/right    |
| `w` / `b`           | Word forward / backward    |
| `0` / `$`           | Line start / end           |
| `gg` / `G`          | Document top / bottom      |
| `Ctrl-d` / `Ctrl-u` | Half page down / up        |
| `Ctrl-f` / `Ctrl-b` | Full page down / up        |
| `v`                 | Enter Visual mode          |
| `i`                 | Create insertion at cursor |
| `gc`                | Create global comment      |
| `]a` / `[a`         | Next / previous annotation |
| `Tab`               | Open annotation list       |
| `W`                 | Toggle word wrap           |
| `?`                 | Toggle help                |
| `:`                 | Enter Command mode         |

### Visual Mode

| Key             | Action                   |
| --------------- | ------------------------ |
| `h` `j` `k` `l` | Extend selection         |
| `w` / `b`       | Extend by word           |
| `0` / `$`       | Extend to line start/end |
| `d`             | Create deletion          |
| `c`             | Create comment           |
| `r`             | Create replacement       |
| `Esc`           | Cancel selection         |

### Annotation List Mode

| Key           | Action             |
| ------------- | ------------------ |
| `j` / `k`     | Navigate list      |
| `Enter`       | Jump to annotation |
| `dd`          | Delete annotation  |
| `Tab` / `Esc` | Exit list          |

### Commands

| Command | Action                        |
| ------- | ----------------------------- |
| `:q`    | Quit and print annotations    |
| `:q!`   | Quit without output           |
| `:w`    | Preview annotations to stderr |

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
