mod llm;
mod context;
mod safety;
mod ui;
mod executor;
mod clipboard;
mod logger;
mod config;

use clap::Parser;

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
    eprintln!("input: {}", args.input);
    eprintln!("dry: {}", args.dry);
    eprintln!("explain: {}", args.explain);
    eprintln!("no_context: {}", args.no_context);
    eprintln!("model: {}", args.model);
    eprintln!("url: {}", args.url);
}
