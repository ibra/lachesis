use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Autostart {
        toggle: String,
    },
    Start,
    Stop,
    Mode {
        mode: String,
    },
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Show only today's usage
        #[arg(short = 'd', long)]
        today: bool,
        /// Date to show usage for (YYYY-MM-DD format)
        #[arg(long)]
        date: Option<String>,
    },
    Tag {
        /// Process name to tag
        process: String,
        /// Tags to add (comma-separated)
        #[arg(short, long)]
        add: Option<String>,
        /// Tags to remove (comma-separated)
        #[arg(short, long)]
        remove: Option<String>,
        /// List tags for a process
        #[arg(short, long)]
        list: bool,
    },
    Reset,
    Delete {
        /// Delete all recorded time
        #[arg(long)]
        all: bool,
        /// Delete data older than duration (e.g., 7d, 30d)
        #[arg(long)]
        duration: Option<String>,
    },
    Whitelist {
        #[command(subcommand)]
        action: ListAction,
    },
    Blacklist {
        #[command(subcommand)]
        action: ListAction,
    },
}

#[derive(Subcommand)]
pub enum ListAction {
    Add {
        /// Process name or regex pattern to add
        process: String,
        /// Treat the pattern as a regex (requires confirmation)
        #[arg(short, long)]
        regex: bool,
    },
    Remove {
        /// Process name or regex pattern to remove
        process: String,
    },
    List,
    Clear,
}
