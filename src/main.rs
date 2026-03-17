mod annotation;
mod app;
mod highlight;
mod input;
mod keybinds;
mod markdown;
mod tui;

use std::io::IsTerminal;
use std::io::Write;
use std::process;

use clap::Parser;

use highlight::syntect::SyntectHighlighter;
use highlight::Highlighter;
use input::{FileSource, InputSource, StdinSource};
use markdown::block::BlockType;

/// Anno — Terminal Markdown Annotation TUI
#[derive(Parser)]
#[command(name = "anno", about = "Annotate markdown files in the terminal")]
struct Cli {
    /// Markdown file to annotate
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let source: Box<dyn InputSource> = if let Some(path) = cli.file {
        Box::new(FileSource::new(path))
    } else if !std::io::stdin().is_terminal() {
        Box::new(StdinSource)
    } else {
        eprintln!("Usage: anno <FILE> or pipe via stdin (e.g. cat file.md | anno)");
        process::exit(1);
    };

    let content = match source.read_content() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    // TODO: Launch TUI with content and source.name()
    let blocks = markdown::parser::parse_markdown_to_blocks(&content);
    let highlighter = SyntectHighlighter::new();

    for block in &blocks {
        match block.block_type {
            BlockType::Heading => {
                let prefix = "#".repeat(block.level);
                let spans = highlighter.highlight_line(&block.content);
                print!("{prefix} ");
                print_spans(&spans);
                println!();
            }
            BlockType::Code => {
                let lang_label = block.language.as_deref().unwrap_or("");
                println!("```{lang_label}");
                let lines =
                    highlighter.highlight_code_block(&block.content, block.language.as_deref());
                for line in &lines {
                    print_spans(line);
                    println!();
                }
                println!("```");
            }
            BlockType::Blockquote => {
                let spans = highlighter.highlight_line(&block.content);
                print!("> ");
                print_spans(&spans);
                println!();
            }
            BlockType::ListItem => {
                let indent = "  ".repeat(block.level);
                let spans = highlighter.highlight_line(&block.content);
                print!("{indent}- ");
                print_spans(&spans);
                println!();
            }
            BlockType::HorizontalRule => {
                println!("---");
            }
            BlockType::Table | BlockType::Paragraph => {
                for line in block.content.lines() {
                    let spans = highlighter.highlight_line(line);
                    print_spans(&spans);
                    println!();
                }
            }
        }
        println!();
    }
}

fn print_spans(spans: &[highlight::StyledSpan]) {
    use ratatui::style::{Color, Modifier};

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for span in spans {
        let mut parts: Vec<String> = Vec::new();

        if span.style.add_modifier.contains(Modifier::BOLD) {
            parts.push("1".into());
        }
        if span.style.add_modifier.contains(Modifier::ITALIC) {
            parts.push("3".into());
        }
        if span.style.add_modifier.contains(Modifier::UNDERLINED) {
            parts.push("4".into());
        }

        match span.style.fg {
            Some(Color::Rgb(r, g, b)) => parts.push(format!("38;2;{r};{g};{b}")),
            Some(Color::Blue) => parts.push("34".into()),
            Some(Color::Yellow) => parts.push("33".into()),
            Some(Color::Red) => parts.push("31".into()),
            Some(Color::Green) => parts.push("32".into()),
            Some(Color::Magenta) => parts.push("35".into()),
            Some(Color::Cyan) => parts.push("36".into()),
            _ => {}
        }

        if parts.is_empty() {
            let _ = write!(out, "{}", span.text);
        } else {
            let seq = parts.join(";");
            let _ = write!(out, "\x1b[{seq}m{}\x1b[0m", span.text);
        }
    }
}
