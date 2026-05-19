use crate::ui::{LogType, log};

pub fn copy_cd_to_clipboard(path: &str) {
    let command = format!("cd {}", shell_quote(path));

    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(command.clone())) {
        Ok(()) => log(&format!("✓ Copied: {}", command), LogType::Success),
        Err(e) => {
            log(&format!("⚠ Clipboard unavailable ({}):", e), LogType::Warning);
            println!("{}", command);
        }
    }
}

fn shell_quote(s: &str) -> String {
    let safe = !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '/' | '_' | '-' | '.' | '~' | '@' | '+' | ',' | ':')
        });
    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("/home/user/api"), "/home/user/api");
        assert_eq!(shell_quote("/My Projects/api"), "'/My Projects/api'");
        assert_eq!(shell_quote("/o'reilly"), "'/o'\\''reilly'");
    }
}
