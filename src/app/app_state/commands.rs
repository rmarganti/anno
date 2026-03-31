use super::core::AppState;
use crate::annotation::export::{AgentExporter, AnnotationExporter, JsonExporter};
use crate::app::ExitResult;
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;
use crate::tui::app_command::{AppCommand, QuitKind};
use crate::tui::command_line::CommandLineEvent;

impl AppState {
    pub(super) fn handle_command_line_event(&mut self, event: CommandLineEvent) {
        match event {
            CommandLineEvent::Command(cmd) => self.process_app_command(cmd),
            CommandLineEvent::ExitToNormal => self.mode = Mode::Normal,
            CommandLineEvent::Consumed => {}
        }
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
