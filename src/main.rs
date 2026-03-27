mod annotation;
mod app;
mod highlight;
mod input;
mod keybinds;
mod startup;
mod tui;

use std::io::{self, IsTerminal, Write};
use std::process;

use clap::Parser;

use app::{App, ExitResult};
use input::{FileSource, InputSource, StdinSource};
use startup::{Cli, StartupSettings};

fn main() {
    let cli = Cli::parse();

    let source: Box<dyn InputSource> = if let Some(path) = cli.file.as_ref() {
        Box::new(FileSource::new(path.clone()))
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

    let startup = match StartupSettings::resolve(&cli, &source_name) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let app = match App::new(source_name, content, startup) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

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
