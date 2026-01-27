use std::process::{Command, Stdio};

pub fn open_with_editor(
    editor: &str,
    project_path: &str,
    reuse: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("setsid");
    command.arg(editor);

    if ["cursor", "code", "vscodium"].contains(&editor) {
        command.arg("--no-sandbox");
    }

    if reuse {
        command.arg("-r");
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
