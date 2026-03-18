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
        /// Use container runtime (auto-detects docker/podman)
        #[arg(long)]
        container: bool,
        /// Use uv/Python backend
        #[arg(long)]
        uv: bool,
    },

    /// Health check all services
    Doctor,

    /// Show Graphiti connection status
    Status,

    /// Update ekko to the latest version
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { container, uv } => {
            let backend = match (container, uv) {
                (true, _) => Some(cmd::init::Backend::Container),
                (_, true) => Some(cmd::init::Backend::Uv),
                _ => None,
            };
            cmd::init::run(backend).await
        }
        Commands::Doctor => cmd::doctor::run().await,
        Commands::Status => cmd::status::run().await,
        Commands::Update { check } => cmd::update::run(check).await,
    }
}
