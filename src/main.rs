mod cmd;
mod config;
mod graphiti;
mod mcp;
mod project;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ekko", version, about = "Persistent memory for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum RmTarget {
    /// Delete a fact by UUID
    Fact { uuid: String },
    /// Delete an episode by UUID
    Episode { uuid: String },
}

#[derive(Subcommand)]
enum Commands {
    /// Store a memory in the knowledge graph
    Add {
        /// The text content to remember
        text: String,

        /// Project group ID (auto-detected from cwd)
        #[arg(long)]
        group: Option<String>,

        /// Name for this memory episode
        #[arg(long)]
        name: Option<String>,

        /// Source type (default: "cli")
        #[arg(long)]
        source: Option<String>,
    },

    /// Search the knowledge graph for facts and entities
    Ask {
        /// Search query
        query: String,

        /// Project group ID (auto-detected from cwd)
        #[arg(long)]
        group: Option<String>,

        /// Include entity nodes in results
        #[arg(long)]
        nodes: bool,

        /// Maximum number of facts to return
        #[arg(long)]
        max_facts: Option<u32>,

        /// Maximum number of nodes to return
        #[arg(long)]
        max_nodes: Option<u32>,
    },

    /// Inspect a specific fact by UUID
    Show {
        /// Fact UUID
        uuid: String,
    },

    /// Delete a fact or episode by UUID
    Rm {
        #[command(subcommand)]
        target: RmTarget,
    },

    /// List or search entities in the knowledge graph
    Nodes {
        /// Optional search query (lists all if omitted)
        query: Option<String>,

        /// Project group ID (auto-detected from cwd)
        #[arg(long)]
        group: Option<String>,

        /// Filter by entity type
        #[arg(long, name = "type")]
        entity_type: Option<String>,

        /// Maximum number of nodes to return
        #[arg(long)]
        max: Option<u32>,
    },

    /// List recent memory episodes
    Episodes {
        /// Project group ID (auto-detected from cwd)
        #[arg(long)]
        group: Option<String>,

        /// Maximum number of episodes to return
        #[arg(long)]
        max: Option<u32>,
    },

    /// Wipe all memories for a project group
    Clear {
        /// Project name (auto-detected from cwd if omitted)
        group: Option<String>,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Initialize ekko (set up Graphiti + FalkorDB via docker/podman, pull Ollama models)
    Init,

    /// Start the MCP server over STDIO
    Serve,

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
        Commands::Add { text, group, name, source } => cmd::add::run(text, group, name, source).await,
        Commands::Ask { query, group, nodes, max_facts, max_nodes } => {
            cmd::ask::run(query, group, nodes, max_facts, max_nodes).await
        }
        Commands::Show { uuid } => cmd::show::run(uuid).await,
        Commands::Rm { target } => match target {
            RmTarget::Fact { uuid } => cmd::rm::run_fact(uuid).await,
            RmTarget::Episode { uuid } => cmd::rm::run_episode(uuid).await,
        },
        Commands::Nodes { query, group, entity_type, max } => {
            cmd::nodes::run(query, group, entity_type, max).await
        }
        Commands::Episodes { group, max } => cmd::episodes::run(group, max).await,
        Commands::Clear { group, yes } => cmd::clear::run(group, yes).await,
        Commands::Init => cmd::init::run().await,
        Commands::Serve => cmd::serve::run().await,
        Commands::Doctor => cmd::doctor::run().await,
        Commands::Status => cmd::status::run().await,

        Commands::Update { check } => cmd::update::run(check).await,
    }
}
