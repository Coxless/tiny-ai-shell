pub mod color;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal,
};
use std::io::{self, Write};

use color::Colors;

#[derive(Debug)]
pub enum Action {
    Execute,
    Cancel,
    Copy,
    Explain,
    Rewrite,
}

/// Show command, optional danger warning, optional explanation, and interactive y/n/c/e/r prompt.
/// Returns the user's chosen action.
pub fn prompt_action(command: &str, explanation: Option<&str>) -> anyhow::Result<Action> {
    let colors = Colors::new();

    let danger = crate::safety::check(command);
    if danger.is_dangerous {
        let msg = danger.message.unwrap_or_default();
        println!(
            "{}  Dangerous command detected: {}",
            colors.red("⚠"),
            colors.red(&msg)
        );
        println!("   {}", colors.cyan(command));
        println!();
        print!("Continue anyway? [y/N]: ");
        io::stdout().flush()?;

        let confirmed = read_bool_key()?;
        if !confirmed {
            return Ok(Action::Cancel);
        }
        println!();
    }

    println!("$ {}", colors.cyan(command));
    if let Some(exp) = explanation {
        println!("  → {}", exp);
    }
    println!();

    print!("[y] Execute  [n] Cancel  [c] Copy  [e] Explain  [r] Rewrite\n> ");
    io::stdout().flush()?;

    loop {
        match read_char_key()? {
            'y' | 'Y' => {
                println!();
                return Ok(Action::Execute);
            }
            'n' | 'N' | '\x1b' | 'q' | 'Q' => {
                println!();
                return Ok(Action::Cancel);
            }
            'c' | 'C' => {
                println!();
                return Ok(Action::Copy);
            }
            'e' | 'E' => {
                println!();
                return Ok(Action::Explain);
            }
            'r' | 'R' => {
                println!();
                return Ok(Action::Rewrite);
            }
            _ => {}
        }
    }
}

fn read_char_key() -> anyhow::Result<char> {
    terminal::enable_raw_mode()?;
    let result = read_char_raw();
    terminal::disable_raw_mode().ok();
    result
}

fn read_char_raw() -> anyhow::Result<char> {
    loop {
        match event::read()? {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    terminal::disable_raw_mode().ok();
                    println!();
                    std::process::exit(130);
                }
                match code {
                    KeyCode::Char(c) => return Ok(c),
                    KeyCode::Esc => return Ok('\x1b'),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn read_bool_key() -> anyhow::Result<bool> {
    terminal::enable_raw_mode()?;
    let result = read_bool_raw();
    terminal::disable_raw_mode().ok();
    result
}

fn read_bool_raw() -> anyhow::Result<bool> {
    loop {
        match event::read()? {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    terminal::disable_raw_mode().ok();
                    println!();
                    std::process::exit(130);
                }
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        println!();
                        return Ok(true);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => {
                        println!();
                        return Ok(false);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
