mod llm;
mod context;
mod safety;
mod ui;
mod executor;
mod clipboard;
mod logger;
mod config;

use clap::Parser;
use llm::LlmClient;
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

    /// Ollama model to use
    #[arg(long, default_value = "mistral")]
    model: String,

    /// Ollama API base URL
    #[arg(long, default_value = "http://localhost:11434")]
    url: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let ctx = if args.no_context {
        context::default_context()
    } else {
        context::gather()
    };

    let base_url = llm::resolve_base_url(&args.url);
    let client = LlmClient::new(args.model.clone(), base_url);

    let command = match client.generate(&args.input, &ctx).await {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.dry {
        let exit_code = executor::run(&command, true).unwrap_or(0);
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

    loop {
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
                std::process::exit(exit_code);
            }
            Action::Cancel => {
                println!("Cancelled.");
                std::process::exit(0);
            }
            Action::Copy => {
                match clipboard::copy(&command) {
                    Ok(_) => println!("✓ Copied to clipboard"),
                    Err(e) => eprintln!("Error: {}", e),
                }
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
                // Step 9
                eprintln!("Rewrite will be implemented in step 9.");
                std::process::exit(0);
            }
        }
    }
}
