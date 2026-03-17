use super::block::{Block, BlockType};

/// Strip YAML frontmatter delimited by `---` from the beginning of the input.
/// Returns the markdown content after the closing `---`.
fn strip_frontmatter(markdown: &str) -> &str {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return markdown;
    }

    // Find the closing `---` (must be on its own line after the opening).
    match trimmed[3..].find("\n---") {
        Some(end) => {
            let after = &trimmed[3 + end + 4..]; // skip past "\n---"
            after.trim_start_matches('\n')
        }
        None => markdown, // No closing delimiter — not valid frontmatter.
    }
}

/// Parse a markdown string into a sequence of [`Block`]s.
///
/// Matches the block types produced by Plannotator's `parseMarkdownToBlocks`:
/// `Heading`, `Code`, `Blockquote`, `ListItem`, `HorizontalRule`, `Table`,
/// and `Paragraph` (the default fallback).
///
/// Frontmatter (`---`-delimited YAML) is stripped before parsing.
pub fn parse_markdown_to_blocks(markdown: &str) -> Vec<Block> {
    let clean = strip_frontmatter(markdown);
    let lines: Vec<&str> = clean.split('\n').collect();
    let mut blocks: Vec<Block> = Vec::new();
    let mut next_id: usize = 0;

    let mut buffer: Vec<&str> = Vec::new();
    let mut buffer_start_line: usize = 1;

    let mut i = 0;

    let flush = |blocks: &mut Vec<Block>, buffer: &mut Vec<&str>, next_id: &mut usize, buffer_start_line: usize| {
        if !buffer.is_empty() {
            let content = buffer.join("\n");
            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::Paragraph,
                content,
                level: 0,
                start_line: buffer_start_line,
                language: None,
                checked: None,
            });
            *next_id += 1;
            buffer.clear();
        }
    };

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let current_line_num = i + 1; // 1-based

        // --- Headings ---
        if trimmed.starts_with('#') {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            let level = trimmed.bytes().take_while(|&b| b == b'#').count();
            let content = trimmed[level..].trim_start().to_string();
            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::Heading,
                content,
                level,
                start_line: current_line_num,
                language: None,
                checked: None,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- Horizontal Rule ---
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::HorizontalRule,
                content: String::new(),
                level: 0,
                start_line: current_line_num,
                language: None,
                checked: None,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- List Items ---
        if is_list_item(trimmed) {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);

            // Calculate indentation level from leading whitespace.
            let leading_spaces = line.len() - line.trim_start().len();
            let list_level = leading_spaces / 2;

            // Remove the list marker.
            let after_marker = strip_list_marker(trimmed);

            // Check for checkbox syntax: [ ] or [x] or [X]
            let (checked, content) = parse_checkbox(after_marker);

            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::ListItem,
                content: content.to_string(),
                level: list_level,
                start_line: current_line_num,
                language: None,
                checked,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- Blockquotes ---
        if trimmed.starts_with('>') {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            let content = trimmed.strip_prefix('>').unwrap_or("").trim_start().to_string();
            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::Blockquote,
                content,
                level: 0,
                start_line: current_line_num,
                language: None,
                checked: None,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- Fenced Code Blocks ---
        if trimmed.starts_with("```") {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            let code_start_line = current_line_num;
            let lang_tag = trimmed[3..].trim();
            let language = if lang_tag.is_empty() {
                None
            } else {
                Some(lang_tag.to_string())
            };

            let mut code_lines: Vec<&str> = Vec::new();
            i += 1; // skip opening fence
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            // i now points at the closing fence (or past the end); skip it.

            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::Code,
                content: code_lines.join("\n"),
                level: 0,
                start_line: code_start_line,
                language,
                checked: None,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- Tables ---
        if is_table_line(trimmed) {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            let table_start_line = current_line_num;
            let mut table_lines: Vec<&str> = vec![line];
            while i + 1 < lines.len() {
                let next_trimmed = lines[i + 1].trim();
                if is_table_line(next_trimmed) {
                    i += 1;
                    table_lines.push(lines[i]);
                } else {
                    break;
                }
            }
            blocks.push(Block {
                id: format!("block-{next_id}"),
                block_type: BlockType::Table,
                content: table_lines.join("\n"),
                level: 0,
                start_line: table_start_line,
                language: None,
                checked: None,
            });
            next_id += 1;
            i += 1;
            continue;
        }

        // --- Empty lines separate paragraphs ---
        if trimmed.is_empty() {
            flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);
            i += 1;
            continue;
        }

        // --- Accumulate paragraph text ---
        if buffer.is_empty() {
            buffer_start_line = current_line_num;
        }
        buffer.push(line);
        i += 1;
    }

    // Final flush.
    flush(&mut blocks, &mut buffer, &mut next_id, buffer_start_line);

    blocks
}

/// Returns `true` if `trimmed` looks like a list item marker.
fn is_list_item(trimmed: &str) -> bool {
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return true;
    }
    // Ordered list: one or more digits followed by `. `
    let mut chars = trimmed.chars();
    if let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            for c in chars {
                if c == '.' {
                    return trimmed[trimmed.find('.').unwrap() + 1..].starts_with(' ');
                }
                if !c.is_ascii_digit() {
                    return false;
                }
            }
        }
    }
    false
}

/// Strip the list marker (`- `, `* `, `1. `, etc.) from the front of a trimmed line.
fn strip_list_marker(trimmed: &str) -> &str {
    if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
        return rest;
    }
    // Ordered list marker
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            return &trimmed[dot_pos + 2..];
        }
    }
    trimmed
}

/// Parse checkbox syntax (`[x]`, `[X]`, `[ ]`) at the start of content.
/// Returns `(checked, remaining_content)`.
fn parse_checkbox(content: &str) -> (Option<bool>, &str) {
    if let Some(rest) = content.strip_prefix("[ ] ") {
        (Some(false), rest)
    } else if let Some(rest) = content
        .strip_prefix("[x] ")
        .or_else(|| content.strip_prefix("[X] "))
    {
        (Some(true), rest)
    } else {
        (None, content)
    }
}

/// Returns `true` if the line looks like part of a markdown table.
fn is_table_line(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('|') {
        return true;
    }
    // Loose table syntax: contains `|` and matches the pattern `col|col|col`
    if trimmed.contains('|') {
        // Simple heuristic matching Plannotator: at least two `|`-separated segments.
        let segments: Vec<&str> = trimmed.split('|').collect();
        return segments.len() >= 3;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Frontmatter ----

    #[test]
    fn strips_frontmatter() {
        let md = "---\ntitle: Hello\n---\n# Heading";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Heading);
        assert_eq!(blocks[0].content, "Heading");
    }

    #[test]
    fn no_frontmatter_passes_through() {
        let md = "# Heading";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Heading");
    }

    #[test]
    fn unclosed_frontmatter_is_not_stripped() {
        let md = "---\ntitle: Hello\n# Heading";
        let blocks = parse_markdown_to_blocks(md);
        // The `---` is treated as a horizontal rule since frontmatter is unclosed.
        assert!(blocks.iter().any(|b| b.block_type == BlockType::HorizontalRule));
    }

    // ---- Headings ----

    #[test]
    fn parses_headings_with_levels() {
        let md = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 6);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.block_type, BlockType::Heading);
            assert_eq!(block.level, i + 1);
            assert_eq!(block.content, format!("H{}", i + 1));
        }
    }

    #[test]
    fn heading_tracks_source_line() {
        let md = "# First\n\n# Second";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks[0].start_line, 1);
        assert_eq!(blocks[1].start_line, 3);
    }

    // ---- Code Blocks ----

    #[test]
    fn parses_fenced_code_block_with_language() {
        let md = "```rust\nfn main() {}\n```";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Code);
        assert_eq!(blocks[0].language, Some("rust".to_string()));
        assert_eq!(blocks[0].content, "fn main() {}");
    }

    #[test]
    fn parses_fenced_code_block_without_language() {
        let md = "```\nhello\nworld\n```";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Code);
        assert_eq!(blocks[0].language, None);
        assert_eq!(blocks[0].content, "hello\nworld");
    }

    #[test]
    fn code_block_preserves_inner_blank_lines() {
        let md = "```\na\n\nb\n```";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks[0].content, "a\n\nb");
    }

    // ---- Blockquotes ----

    #[test]
    fn parses_blockquote() {
        let md = "> This is a quote";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Blockquote);
        assert_eq!(blocks[0].content, "This is a quote");
    }

    #[test]
    fn parses_nested_blockquote_prefix() {
        // Each `>` line is its own block; nesting is not tracked beyond stripping one `>`.
        let md = "> outer\n>> inner";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "outer");
        assert_eq!(blocks[1].content, "> inner");
    }

    // ---- List Items ----

    #[test]
    fn parses_unordered_list_dash() {
        let md = "- item one\n- item two";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, BlockType::ListItem);
        assert_eq!(blocks[0].content, "item one");
        assert_eq!(blocks[0].level, 0);
        assert_eq!(blocks[1].content, "item two");
    }

    #[test]
    fn parses_unordered_list_asterisk() {
        let md = "* item";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks[0].block_type, BlockType::ListItem);
        assert_eq!(blocks[0].content, "item");
    }

    #[test]
    fn parses_ordered_list() {
        let md = "1. first\n2. second";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, BlockType::ListItem);
        assert_eq!(blocks[0].content, "first");
    }

    #[test]
    fn parses_indented_list_items() {
        let md = "- top\n  - nested\n    - deep";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].level, 0);
        assert_eq!(blocks[1].level, 1);
        assert_eq!(blocks[2].level, 2);
    }

    #[test]
    fn parses_checkbox_list_items() {
        let md = "- [ ] unchecked\n- [x] checked\n- [X] also checked";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks[0].checked, Some(false));
        assert_eq!(blocks[0].content, "unchecked");
        assert_eq!(blocks[1].checked, Some(true));
        assert_eq!(blocks[2].checked, Some(true));
    }

    // ---- Horizontal Rules ----

    #[test]
    fn parses_horizontal_rules() {
        for marker in &["---", "***", "___"] {
            let blocks = parse_markdown_to_blocks(marker);
            assert_eq!(blocks.len(), 1, "failed for {marker}");
            assert_eq!(blocks[0].block_type, BlockType::HorizontalRule);
            assert_eq!(blocks[0].content, "");
        }
    }

    // ---- Tables ----

    #[test]
    fn parses_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Table);
        assert_eq!(blocks[0].content, "| A | B |\n|---|---|\n| 1 | 2 |");
    }

    // ---- Paragraphs ----

    #[test]
    fn parses_paragraph() {
        let md = "Hello world";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert_eq!(blocks[0].content, "Hello world");
    }

    #[test]
    fn multi_line_paragraph() {
        let md = "Line one\nLine two";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert_eq!(blocks[0].content, "Line one\nLine two");
    }

    #[test]
    fn blank_line_splits_paragraphs() {
        let md = "Para one\n\nPara two";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "Para one");
        assert_eq!(blocks[1].content, "Para two");
    }

    // ---- Empty Input ----

    #[test]
    fn empty_input_returns_no_blocks() {
        let blocks = parse_markdown_to_blocks("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn whitespace_only_returns_no_blocks() {
        let blocks = parse_markdown_to_blocks("   \n\n   ");
        assert!(blocks.is_empty());
    }

    // ---- Mixed Content ----

    #[test]
    fn mixed_content_document() {
        let md = "\
# Title

Some intro text
spanning two lines.

- item a
- item b

> a quote

```python
print('hi')
```

---

Final paragraph.";

        let blocks = parse_markdown_to_blocks(md);

        assert_eq!(blocks[0].block_type, BlockType::Heading);
        assert_eq!(blocks[0].content, "Title");

        assert_eq!(blocks[1].block_type, BlockType::Paragraph);
        assert_eq!(blocks[1].content, "Some intro text\nspanning two lines.");

        assert_eq!(blocks[2].block_type, BlockType::ListItem);
        assert_eq!(blocks[2].content, "item a");

        assert_eq!(blocks[3].block_type, BlockType::ListItem);
        assert_eq!(blocks[3].content, "item b");

        assert_eq!(blocks[4].block_type, BlockType::Blockquote);
        assert_eq!(blocks[4].content, "a quote");

        assert_eq!(blocks[5].block_type, BlockType::Code);
        assert_eq!(blocks[5].language, Some("python".to_string()));
        assert_eq!(blocks[5].content, "print('hi')");

        assert_eq!(blocks[6].block_type, BlockType::HorizontalRule);

        assert_eq!(blocks[7].block_type, BlockType::Paragraph);
        assert_eq!(blocks[7].content, "Final paragraph.");
    }

    // ---- Block IDs are sequential ----

    #[test]
    fn block_ids_are_sequential() {
        let md = "# A\n\nText\n\n- item";
        let blocks = parse_markdown_to_blocks(md);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.id, format!("block-{i}"));
        }
    }
}
