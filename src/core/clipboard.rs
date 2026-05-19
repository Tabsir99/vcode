use arboard::Clipboard;

pub struct ClipboardError(String);

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Hidden subcommand used to re-exec ourselves as a detached clipboard
/// daemon on Linux. Intercepted in `main.rs` before clap parsing, so it
/// stays out of `--help`.
pub const DAEMON_SUBCOMMAND: &str = "__clipboard-daemon";

/// Builds `cd <posix-quoted-path>`, copies it to the system clipboard, and
/// returns the exact string that was copied (so callers can echo it back).
pub fn copy_cd_command(path: &str) -> Result<String, ClipboardError> {
    let cmd = format!("cd {}", posix_quote(path));
    copy_text(&cmd)?;
    Ok(cmd)
}

// On Linux the clipboard (X11 and Wayland alike) is owned by the source
// process: when it exits the data goes with it unless a clipboard manager
// has already requested a transfer. `arboard::set_text` returns immediately,
// well before any manager has had a chance to copy the data — which is why a
// naive call "succeeds" but the next paste yields stale content. The fix is
// to keep a process alive holding ownership until somebody else claims it.
// We do that by re-execing ourselves as a detached child that calls
// `Set::wait()` (which blocks until ownership transfers), so the user's
// invocation returns immediately while the data persists in the background.
#[cfg(target_os = "linux")]
fn copy_text(text: &str) -> Result<(), ClipboardError> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    // Probe the clipboard from the parent so headless invocations fail loud
    // with a non-zero exit (per the feature spec) instead of silently
    // spawning a daemon that immediately dies.
    drop(Clipboard::new().map_err(|e| ClipboardError(e.to_string()))?);

    let exe = std::env::current_exe()
        .map_err(|e| ClipboardError(format!("current_exe: {}", e)))?;

    let mut cmd = Command::new(exe);
    cmd.arg(DAEMON_SUBCOMMAND)
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // SAFETY: pre_exec runs between fork() and exec(). setsid is
    // async-signal-safe; we don't allocate or touch the Rust runtime here.
    // setsid detaches the daemon from the parent's controlling terminal so
    // it survives a `Ctrl-D` / terminal close after vcode returns.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    cmd.spawn()
        .map_err(|e| ClipboardError(format!("spawn clipboard daemon: {}", e)))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn copy_text(text: &str) -> Result<(), ClipboardError> {
    // macOS and Windows have system-managed clipboards: data persists past
    // process exit, so no daemon dance is needed.
    let mut cb = Clipboard::new().map_err(|e| ClipboardError(e.to_string()))?;
    cb.set_text(text).map_err(|e| ClipboardError(e.to_string()))?;
    Ok(())
}

/// Linux-only daemon entry point: claims clipboard ownership and blocks
/// until another process takes over (a paste followed by the manager
/// caching it, another `set`, etc.). Returns `!` because it always exits.
#[cfg(target_os = "linux")]
pub fn run_daemon(text: &str) -> ! {
    use arboard::SetExtLinux;
    let mut cb = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => std::process::exit(1),
    };
    let _ = cb.set().wait().text(text.to_string());
    std::process::exit(0);
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
