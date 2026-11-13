use std::path::Path;

use agent_desktop_core::AppError;

use crate::cli::Commands;
use crate::cli_args::{cursor_overlay_action::CursorOverlayAction, trace::TraceAction};

pub(crate) fn validate_optional(path: Option<&Path>) -> Result<(), AppError> {
    let Some(path) = path else {
        return Ok(());
    };
    let text = path.to_string_lossy();
    if agent_desktop_core::is_windows_network_path(&text) {
        return Err(AppError::invalid_input_with_suggestion(
            "File arguments refuse network (UNC) and device-namespace paths: opening one can send the user's NetNTLM credentials to a remote host",
            "Copy the file to a local drive and pass its local path.",
        ));
    }
    Ok(())
}

pub(crate) fn validate_command(command: &Commands) -> Result<(), AppError> {
    match command {
        Commands::Screenshot(args) => validate_optional(args.output_path.as_deref()),
        Commands::ClipboardGet(args) => validate_optional(args.out.as_deref()),
        Commands::ClipboardSet(args) => {
            validate_optional(args.image.as_deref())?;
            args.file_url
                .iter()
                .try_for_each(|path| validate_optional(Some(Path::new(path))))
        }
        Commands::Trace(args) => match &args.action {
            TraceAction::Export(export) => validate_optional(export.out.as_deref()),
            TraceAction::Show(_) => Ok(()),
        },
        Commands::CursorOverlay(args) => match &args.action {
            CursorOverlayAction::Enable(enable) => {
                validate_optional(enable.style.images.image.as_deref())?;
                validate_optional(enable.style.images.pointer_image.as_deref())
            }
            CursorOverlayAction::Disable => Ok(()),
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "file_path_tests.rs"]
mod tests;
