mod llm;
mod context;
mod safety;
mod ui;
mod executor;
mod clipboard;
mod logger;
mod config;

use clap::Parser;
use llm::{LlmClient, resolve_language};
use ui::Action;

#[derive(Parser, Debug)]
#[command(
    name = "ta",
    version,
    about = "Natural language to shell command using local LLM"
)]
struct Args {
    /// Natural language instruction
    input: String,

    /// Show command without executing
    #[arg(long)]
    dry: bool,

    /// Show explanation immediately after generating command
    #[arg(long)]
    explain: bool,

    /// Skip sending current directory context to LLM
    #[arg(long)]
    no_context: bool,

    /// Ollama model to use (default: "mistral", env: TA_MODEL, config: ~/.config/ta/config.toml)
    #[arg(long)]
    model: Option<String>,

    /// Ollama API base URL (default: "http://localhost:11434", env: TA_OLLAMA_URL, config: ~/.config/ta/config.toml)
    #[arg(long)]
    url: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let cfg = config::load();

    // Priority: CLI flag > env var > config file > default
    let model = args.model
        .or_else(|| std::env::var("TA_MODEL").ok())
        .unwrap_or(cfg.model);

    let url = args.url
        .or_else(|| std::env::var("TA_OLLAMA_URL").ok())
        .unwrap_or(cfg.ollama_url);

    config::ensure_config_dir();

    let ctx = if args.no_context {
        context::default_context()
    } else {
        context::gather()
    };

    let env_lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let language = resolve_language(cfg.language.as_deref(), &env_lang).to_string();

    let client = LlmClient::new(model, url, language);

    if let Err(e) = client.check_connectivity().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let mut command = match client.generate(&args.input, &ctx).await {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.dry {
        let exit_code = executor::run(&command, true).unwrap_or(0);
        logger::record(&args.input, &command, "dry-run", Some(exit_code), &ctx.pwd, &ctx.os);
        std::process::exit(exit_code);
    }

    // --explain: show explanation immediately before entering interactive flow
    let initial_explanation = if args.explain {
        match client.explain(&command).await {
            Ok(exp) => Some(exp),
            Err(e) => {
                eprintln!("Error getting explanation: {}", e);
                None
            }
        }
    } else {
        None
    };

    let mut last_explanation: Option<String> = initial_explanation;

    'main: loop {
        let action = match ui::prompt_action(&command, last_explanation.as_deref()) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        match action {
            Action::Execute => {
                let exit_code = match executor::run(&command, false) {
                    Ok(code) => code,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        1
                    }
                };
                logger::record(&args.input, &command, "executed", Some(exit_code), &ctx.pwd, &ctx.os);
                std::process::exit(exit_code);
            }
            Action::Cancel => {
                logger::record(&args.input, &command, "cancelled", None, &ctx.pwd, &ctx.os);
                println!("Cancelled.");
                std::process::exit(0);
            }
            Action::Copy => {
                match clipboard::copy(&command) {
                    Ok(_) => println!("✓ Copied to clipboard"),
                    Err(e) => eprintln!("Error: {}", e),
                }
                logger::record(&args.input, &command, "copied", None, &ctx.pwd, &ctx.os);
                std::process::exit(0);
            }
            Action::Explain => {
                match client.explain(&command).await {
                    Ok(exp) => {
                        last_explanation = Some(exp);
                    }
                    Err(e) => {
                        eprintln!("Error getting explanation: {}", e);
                    }
                }
                // loop continues: show prompt again with explanation
            }
            Action::Rewrite => {
                let instruction = match ui::prompt_rewrite_instruction() {
                    Ok(s) if s.is_empty() => continue 'main,
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        continue 'main;
                    }
                };
                println!("→ Rewriting...");
                match client.rewrite(&args.input, &command, &instruction).await {
                    Ok(new_cmd) => {
                        command = new_cmd;
                        last_explanation = None;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }
    }
}
