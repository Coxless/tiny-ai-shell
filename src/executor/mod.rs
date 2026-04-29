use anyhow::Result;
use std::process::Stdio;

use crate::ui::color::Colors;

pub fn run(command: &str, dry: bool) -> Result<i32> {
    let colors = Colors::new();

    if dry {
        println!("$ {}", colors.cyan(command));
        return Ok(0);
    }

    println!("{} Executing: {}", "→", colors.cyan(command));

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    let status = std::process::Command::new(&shell)
        .arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    let code = status.code().unwrap_or(1);

    if status.success() {
        println!("{} Done (exit {})", colors.cyan("✓"), code);
    } else {
        println!("{} Failed (exit {})", colors.red("✗"), code);
    }

    Ok(code)
}
