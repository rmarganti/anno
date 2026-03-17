mod annotation;
mod app;
mod highlight;
mod input;
mod keybinds;
mod markdown;
mod tui;

use std::io::IsTerminal;
use std::process;

use clap::Parser;

use input::{FileSource, InputSource, StdinSource};

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
    let _ = (content, source.name());
}
