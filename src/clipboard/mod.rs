use std::io::Write;
use std::process::{Command, Stdio};

pub fn copy(text: &str) -> anyhow::Result<()> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if try_copy(text, "wl-copy", &[]).is_ok() {
            return Ok(());
        }
    }

    if try_copy(text, "xclip", &["-selection", "clipboard"]).is_ok() {
        return Ok(());
    }

    if try_copy(text, "xsel", &["--clipboard", "--input"]).is_ok() {
        return Ok(());
    }

    anyhow::bail!("No clipboard tool found. Install wl-copy, xclip, or xsel.")
}

fn try_copy(text: &str, cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    anyhow::ensure!(status.success(), "{} exited with {}", cmd, status);
    Ok(())
}
