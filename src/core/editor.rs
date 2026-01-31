use crate::core::config::{get_config, EditorConfig};
use std::process::{Command, Stdio};

pub fn open_with_editor(
    editor: &str,
    project_path: &str,
    reuse: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config();

    // Try to get editor config, or create a simple one for unknown editors
    let editor_config = config.get_editor(editor).cloned().unwrap_or_else(|| {
        EditorConfig::new(editor.to_string())
    });

    let mut command = Command::new("setsid");
    command.arg(&editor_config.command);

    // Add configured args
    for arg in &editor_config.args {
        command.arg(arg);
    }

    // Add reuse flag if requested and available
    if reuse {
        if let Some(ref flag) = editor_config.reuse_flag {
            command.arg(flag);
        }
    }

    command.arg(project_path);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}

pub fn is_vscode_like_editor(editor: &str) -> bool {
    ["cursor", "code", "vscodium"].contains(&editor)
}
