mod annotation;
mod app;
mod highlight;
mod input;
mod keybinds;
mod markdown;
mod tui;

use std::io::{self, IsTerminal, Write};
use std::process;

use clap::Parser;

use app::{App, ExitResult};
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
    } else if !io::stdin().is_terminal() {
        Box::new(StdinSource)
    } else {
        eprintln!("Usage: anno <FILE> or pipe via stdin (e.g. cat file.md | anno)");
        process::exit(1);
    };

    let source_name = source.name().to_string();
    let content = match source.read_content() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let app = App::new(source_name, content);

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();

    match result {
        Ok(ExitResult::QuitWithOutput(output)) => {
            let _ = io::stdout().write_all(output.as_bytes());
        }
        Ok(ExitResult::QuitSilent) => {}
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
