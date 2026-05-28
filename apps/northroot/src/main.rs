//! Northroot CLI - Command-line interface for trust kernel operations.

use clap::{Parser, Subcommand};

mod commands;
mod object_store;
mod output;
mod path;
mod workspace;

use commands::{
    append, canonicalize, connect, event_id, list, verify, verify_bundle,
    workspace as workspace_command,
};

#[derive(Parser)]
#[command(name = "northroot")]
#[command(about = "Northroot trust kernel CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show canonical bytes for input JSON
    Canonicalize {
        /// Input JSON file (or stdin if not provided)
        input: Option<String>,
    },
    /// Compute event_id for input JSON
    EventId {
        /// Input JSON file (or stdin if not provided)
        input: Option<String>,
    },
    /// List events in a journal
    List {
        /// Path to journal file
        journal: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Stop after reading N events (default: unlimited)
        #[arg(long)]
        max_events: Option<u64>,
        /// Reject journals larger than SIZE bytes (default: unlimited)
        #[arg(long)]
        max_size: Option<u64>,
    },
    /// Verify all event IDs in a journal
    Verify {
        /// Path to journal file
        journal: String,
        /// Exit with error code if any verification fails
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Stop after reading N events (default: unlimited)
        #[arg(long)]
        max_events: Option<u64>,
        /// Reject journals larger than SIZE bytes (default: unlimited)
        #[arg(long)]
        max_size: Option<u64>,
    },
    /// Verify a portable evidence bundle
    VerifyBundle {
        /// Path to bundle directory
        dir: String,
        /// Output as JSON (default)
        #[arg(long)]
        json: bool,
    },
    /// Append an event to a journal
    Append {
        /// Path to journal file
        journal: String,
        /// Input JSON file (or stdin if not provided)
        input: Option<String>,
        /// Reject events with mismatched event_id (default: false)
        #[arg(long)]
        strict: bool,
        /// Sync file to disk after append (default: false)
        #[arg(long)]
        sync: bool,
    },
    /// Manage local Northroot workspaces and workspace vaults
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
    },
    /// Record provider connection references for a workspace
    Connect {
        #[command(subcommand)]
        command: ConnectCommands,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommands {
    /// Initialize a local workspace and workspace vault
    Init {
        /// Workspace name
        #[arg(long)]
        name: String,
        /// Workspace root directory
        #[arg(long)]
        root: String,
    },
    /// Show workspace and vault status
    Status {
        /// Workspace root directory
        #[arg(long, default_value = ".")]
        root: String,
    },
}

#[derive(Subcommand)]
enum ConnectCommands {
    /// Record a read-only Gmail/Drive connection reference
    Gmail {
        /// Workspace root directory
        #[arg(long)]
        workspace: String,
        /// Connection mode. V0 supports readonly only.
        #[arg(long, default_value = "readonly")]
        mode: String,
        /// Local Google Workspace CLI profile reference
        #[arg(long, default_value = "default")]
        profile: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Canonicalize { input } => canonicalize::run(input),
        Commands::EventId { input } => event_id::run(input),
        Commands::List {
            journal,
            json,
            max_events,
            max_size,
        } => list::run(journal, json, max_events, max_size),
        Commands::Verify {
            journal,
            strict,
            json,
            max_events,
            max_size,
        } => verify::run(journal, strict, json, max_events, max_size),
        Commands::VerifyBundle { dir, json } => verify_bundle::run(dir, json),
        Commands::Append {
            journal,
            input,
            strict,
            sync,
        } => append::run(journal, input, strict, sync),
        Commands::Workspace { command } => match command {
            WorkspaceCommands::Init { name, root } => workspace_command::init(name, root),
            WorkspaceCommands::Status { root } => workspace_command::status(root),
        },
        Commands::Connect { command } => match command {
            ConnectCommands::Gmail {
                workspace,
                mode,
                profile,
            } => connect::gmail(workspace, mode, profile),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
