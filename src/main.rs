mod annotation;
mod app;
mod highlight;
mod input;
mod keybinds;
mod startup;
#[cfg(test)]
mod test_support;
mod tui;

use std::io::{self, IsTerminal, Write};
use std::panic;
use std::process;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use clap::Parser;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::flag;

use app::{App, ExitResult};
use input::{FileSource, InputSource, StdinSource};
use startup::{Cli, StartupSettings};

fn main() {
    let cli = Cli::parse();
    let output_file = cli.output_file.clone();

    let source: Box<dyn InputSource> = if let Some(path) = cli.file.as_ref() {
        Box::new(FileSource::new(path.clone()))
    } else if !io::stdin().is_terminal() {
        Box::new(StdinSource)
    } else {
        eprintln!("Usage: anno <FILE> or pipe via stdin (e.g. cat file.md | anno)");
        process::exit(1);
    };

    let source_metadata = source.metadata();
    let source_name = source_metadata.display_name.clone();
    let content = match source.read_content() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let startup = match StartupSettings::resolve(&cli, &source_metadata, &content) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    if startup::should_log_startup() {
        match startup.startup_log_json(&source_metadata) {
            Ok(log) => eprintln!("{log}"),
            Err(e) => eprintln!("Warning: failed to serialize startup log: {e}"),
        }
    }

    let app = match App::new(source_name, content, startup) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    // Install a panic hook that restores the terminal before printing.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        default_hook(info);
    }));

    // Register signal handlers that restore the terminal on
    // SIGINT, SIGTERM, and SIGHUP before the process exits.
    let signal_flag = Arc::new(AtomicBool::new(false));
    flag::register(SIGINT, Arc::clone(&signal_flag)).ok();
    flag::register(SIGTERM, Arc::clone(&signal_flag)).ok();
    flag::register(SIGHUP, Arc::clone(&signal_flag)).ok();

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal, &signal_flag);
    ratatui::restore();

    match result {
        Ok(ExitResult::QuitWithOutput(output)) => {
            if let Some(path) = output_file.as_ref() {
                if let Err(e) = std::fs::write(path, output.as_bytes()) {
                    eprintln!("Error writing to {path}: {e}");
                    process::exit(1);
                }
            } else {
                let _ = io::stdout().write_all(output.as_bytes());
            }
        }
        Ok(ExitResult::QuitSilent) => {}
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
