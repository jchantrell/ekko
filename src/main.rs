mod cmd;
mod config;
mod daemon;
mod graphiti;
mod groups;
mod mcp;
mod project;
mod session;

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
enum DaemonCommand {
    /// Start the daemon (foreground)
    Start,
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
}

#[derive(Subcommand)]
enum OriginsCommand {
    /// Set name/description for an origin
    Set {
        /// The origin to update
        origin: String,

        /// Display name
        #[arg(long)]
        name: Option<String>,

        /// Short description
        #[arg(long, short)]
        description: Option<String>,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Store a memory in the knowledge graph
    Add {
        /// The text content to remember
        text: String,

        /// Origin label (auto-detected from cwd)
        #[arg(long)]
        origin: Option<String>,

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

        /// Filter by entity type
        #[arg(long, name = "type")]
        entity_type: Option<String>,

        /// Maximum number of nodes to return
        #[arg(long)]
        max: Option<u32>,
    },

    /// List recent memory episodes
    Episodes {
        /// Maximum number of episodes to return
        #[arg(long)]
        max: Option<u32>,
    },

    /// List or manage memory origins (project sources)
    Origins {
        #[command(subcommand)]
        command: Option<OriginsCommand>,

        /// Include entity/episode counts (slower)
        #[arg(long)]
        stats: bool,

        /// Filter origins by substring
        #[arg(long)]
        filter: Option<String>,
    },

    /// Wipe all memories for an origin
    Clear {
        /// Origin name (auto-detected from cwd if omitted)
        origin: Option<String>,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Initialize ekko (set up Graphiti + Neo4j via docker/podman, pull Ollama models)
    Init,

    /// Start the MCP server over STDIO
    Serve,

    /// Health check all services
    Doctor,

    /// Show Graphiti connection status
    Status,

    /// Show the memory processing queue
    Queue,

    /// Update ekko to the latest version
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,
    },

    /// Manage the ekko daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },

    /// Index agent sessions into the knowledge graph
    Sync {
        /// Re-index all sessions (ignore fingerprints)
        #[arg(long)]
        full: bool,

        /// Only index sessions from this agent (claude-code, opencode)
        #[arg(long)]
        agent: Option<String>,

        /// Origin to index (auto-detected from cwd)
        #[arg(long, short)]
        origin: Option<String>,

        /// Index all origins instead of just the current one
        #[arg(long)]
        all: bool,

        /// Only index sessions after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Skip LLM summarization (extractive fallback)
        #[arg(long)]
        no_llm: bool,

        /// Show what would be indexed without doing it
        #[arg(long)]
        dry_run: bool,

        /// Maximum number of sessions to index
        #[arg(long)]
        limit: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { text, origin, name, source } => cmd::add::run(text, origin, name, source).await,
        Commands::Ask { query, nodes, max_facts, max_nodes } => {
            cmd::ask::run(query, nodes, max_facts, max_nodes).await
        }
        Commands::Show { uuid } => cmd::show::run(uuid).await,
        Commands::Rm { target } => match target {
            RmTarget::Fact { uuid } => cmd::rm::run_fact(uuid).await,
            RmTarget::Episode { uuid } => cmd::rm::run_episode(uuid).await,
        },
        Commands::Nodes { query, entity_type, max } => {
            cmd::nodes::run(query, entity_type, max).await
        }
        Commands::Episodes { max } => cmd::episodes::run(max).await,
        Commands::Origins { command, stats, filter } => cmd::origins::run(command, stats, filter).await,
        Commands::Clear { origin, yes } => cmd::clear::run(origin, yes).await,
        Commands::Init => cmd::init::run().await,
        Commands::Serve => cmd::serve::run().await,
        Commands::Doctor => cmd::doctor::run().await,
        Commands::Status => cmd::status::run().await,
        Commands::Queue => cmd::queue::run().await,

        Commands::Update { check } => cmd::update::run(check).await,
        Commands::Sync { full, agent, origin, all, since, no_llm, dry_run, limit } => {
            cmd::sync::run(full, agent, origin, all, since, no_llm, dry_run, limit).await
        }
        Commands::Daemon { command } => match command {
            DaemonCommand::Start => cmd::daemon::start().await,
            DaemonCommand::Stop => cmd::daemon::stop().await,
            DaemonCommand::Status => cmd::daemon::status().await,
        },
    }
}
