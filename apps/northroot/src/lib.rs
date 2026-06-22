//! Northroot CLI - Command-line interface for trust kernel operations.

use clap::{Parser, Subcommand};

pub mod commands;
pub mod output;
pub mod path;
#[cfg(test)]
mod test_support;

use commands::{
    append, canonicalize, event_id, journal, node, read, record, steward, verify, verify_bundle,
    work,
};

#[derive(Parser)]
#[command(name = "nr")]
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
    /// Read events from a journal
    Read {
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
    #[command(hide = true)]
    VerifyBundle {
        /// Path to bundle directory
        dir: String,
        /// Output as JSON (default)
        #[arg(long)]
        json: bool,
    },
    /// Work ledger ingestion, projection, and verification
    #[command(hide = true)]
    Work {
        #[command(subcommand)]
        command: work::WorkCommand,
    },
    /// Structural segmented journal and checkpoint operations
    #[command(hide = true)]
    Journal {
        #[command(subcommand)]
        command: journal::JournalCommand,
    },
    /// Record stream import/export operations
    #[command(hide = true)]
    Record {
        #[command(subcommand)]
        command: record::RecordCommand,
    },
    /// Node custody and storage initialization
    Node {
        #[command(subcommand)]
        command: node::NodeCommand,
    },
    /// Steward custody profile operations
    Steward {
        #[command(subcommand)]
        command: steward::StewardCommand,
    },
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Canonicalize { input } => canonicalize::run(input),
        Commands::EventId { input } => event_id::run(input),
        Commands::Append {
            journal,
            input,
            strict,
            sync,
        } => append::run(journal, input, strict, sync),
        Commands::Read {
            journal,
            json,
            max_events,
            max_size,
        } => read::run(journal, json, max_events, max_size),
        Commands::Verify {
            journal,
            strict,
            json,
            max_events,
            max_size,
        } => verify::run(journal, strict, json, max_events, max_size),
        Commands::VerifyBundle { dir, json } => verify_bundle::run(dir, json),
        Commands::Work { command } => work::run(command),
        Commands::Journal { command } => journal::run(command),
        Commands::Record { command } => record::run(command),
        Commands::Node { command } => node::run(command),
        Commands::Steward { command } => steward::run(command),
    }
}

pub fn main() {
    if let Err(err) = run_cli() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
