mod cmd;
mod config;
mod graphiti;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ekko", version, about = "Persistent memory for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ekko (set up Graphiti, graph DB, Ollama models)
    Init {
        /// Force Docker backend
        #[arg(long)]
        docker: bool,
        /// Force uv/Python backend
        #[arg(long)]
        uv: bool,
    },

    /// Health check all services
    Doctor,

    /// Show Graphiti connection status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { docker, uv } => {
            let backend = match (docker, uv) {
                (true, _) => Some(cmd::init::Backend::Docker),
                (_, true) => Some(cmd::init::Backend::Uv),
                _ => None,
            };
            cmd::init::run(backend).await
        }
        Commands::Doctor => cmd::doctor::run().await,
        Commands::Status => cmd::status::run().await,
    }
}
