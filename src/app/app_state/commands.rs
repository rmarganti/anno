use super::core::AppState;
use crate::annotation::export::{AgentExporter, AnnotationExporter, JsonExporter};
use crate::app::ExitResult;
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;
use crate::tui::app_command::{AppCommand, QuitKind};

impl AppState {
    pub(super) fn clear_command_buffer(&mut self) {
        self.command_buffer.clear();
    }

    pub(super) fn handle_command_char(&mut self, c: char) {
        self.command_buffer.push(c);
    }

    pub(super) fn handle_command_backspace(&mut self) {
        self.command_buffer.pop();
        if self.command_buffer.is_empty() {
            self.mode = Mode::Normal;
        }
    }

    pub(super) fn handle_command_confirm(&mut self) {
        let command = self.command_buffer.clone();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        let cmd = match command.as_str() {
            "q" | "wq" => AppCommand::Quit(QuitKind::WithOutput),
            "q!" => AppCommand::Quit(QuitKind::Silent),
            _ => return,
        };

        self.process_app_command(cmd);
    }

    fn process_app_command(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Quit(QuitKind::WithOutput) => {
                let output = match self.export_format {
                    ExportFormat::Agent => {
                        AgentExporter.export(&self.annotations, self.source_name())
                    }
                    ExportFormat::Json => {
                        JsonExporter.export(&self.annotations, self.source_name())
                    }
                };
                self.exit_result = Some(ExitResult::QuitWithOutput(output));
                self.should_quit = true;
            }
            AppCommand::Quit(QuitKind::Silent) => {
                self.exit_result = Some(ExitResult::QuitSilent);
                self.should_quit = true;
            }
        }
    }
}
