use arboard::Clipboard;

pub struct ClipboardError(String);

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Builds `cd <posix-quoted-path>`, copies it to the system clipboard, and
/// returns the exact string that was copied (so callers can echo it back).
pub fn copy_cd_command(path: &str) -> Result<String, ClipboardError> {
    let cmd = format!("cd {}", posix_quote(path));
    let mut cb = Clipboard::new().map_err(|e| ClipboardError(e.to_string()))?;
    cb.set_text(&cmd).map_err(|e| ClipboardError(e.to_string()))?;
    Ok(cmd)
}

// POSIX single-quote escaping. Always quoted so we don't need to inspect the
// path for shell-special characters — wrapping a simple path in `'...'` is
// harmless, and the `'\''` dance handles embedded single quotes correctly.
fn posix_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str(r"'\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::posix_quote;

    #[test]
    fn quotes_plain_path() {
        assert_eq!(posix_quote("/home/me/proj"), "'/home/me/proj'");
    }

    #[test]
    fn quotes_path_with_spaces() {
        assert_eq!(posix_quote("/home/me/my proj"), "'/home/me/my proj'");
    }

    #[test]
    fn escapes_single_quote() {
        assert_eq!(posix_quote("/tmp/it's/proj"), r"'/tmp/it'\''s/proj'");
    }

    #[test]
    fn quotes_dollar_and_backslash_verbatim() {
        // Inside single quotes everything is literal in POSIX shells, so $ and
        // \ should pass through untouched.
        assert_eq!(posix_quote("/tmp/$VAR\\x"), "'/tmp/$VAR\\x'");
    }
}
