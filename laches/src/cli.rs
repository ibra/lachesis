use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "a cli-based automatic time tracking tool for monitoring screentime"
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// start the background monitoring daemon
    Start,

    /// stop the background monitoring daemon
    Stop,

    /// list tracked process usage
    List {
        /// filter by tag name
        #[arg(short, long)]
        tag: Option<String>,

        /// show only today's usage
        #[arg(long)]
        today: bool,

        /// show last 7 days
        #[arg(short, long)]
        week: bool,

        /// show last 30 days
        #[arg(short, long)]
        month: bool,

        /// show usage for a specific date (YYYY-MM-DD)
        #[arg(short, long)]
        date: Option<String>,

        /// date range (YYYY-MM-DD..YYYY-MM-DD)
        #[arg(long)]
        range: Option<String>,

        /// show individual sessions instead of process summaries
        #[arg(short, long)]
        sessions: bool,

        /// show extra columns (active days, avg/day, session count)
        #[arg(short, long)]
        verbose: bool,

        /// include data from all synced machines
        #[arg(short = 'a', long)]
        all_machines: bool,
    },

    /// quick daily overview with comparisons
    Summary,

    /// add, remove, or list tags on a tracked process
    Tag {
        /// name of the process to tag
        process: String,

        /// tag(s) to add (comma-separated)
        #[arg(short, long)]
        add: Option<String>,

        /// tag(s) to remove (comma-separated)
        #[arg(short, long)]
        remove: Option<String>,

        /// list current tags for this process
        #[arg(short, long)]
        list: bool,
    },

    /// manage whitelist patterns (only track matched processes)
    Whitelist {
        #[command(subcommand)]
        action: FilterListAction,
    },

    /// manage blacklist patterns (track everything except matched)
    Blacklist {
        #[command(subcommand)]
        action: FilterListAction,
    },

    /// set the filtering mode
    Mode {
        /// filtering mode to use
        mode: FilterMode,
    },

    /// enable or disable autostart on login
    Autostart {
        /// whether to start on login
        toggle: AutostartToggle,
    },

    /// show or modify configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// export, delete, or reset tracked data
    Data {
        #[command(subcommand)]
        action: DataAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// set a custom data storage path
    StorePath {
        /// target directory path
        path: String,
    },
}

#[derive(Subcommand)]
pub enum DataAction {
    /// export tracked data to a json file
    Export {
        /// output file path
        output: String,

        /// only export data from the last N days (e.g. 7d, 30d)
        #[arg(long)]
        duration: Option<String>,

        /// include data from all synced machines
        #[arg(short = 'a', long)]
        all_machines: bool,
    },

    /// delete tracked data by duration or all
    Delete {
        /// delete all recorded data
        #[arg(long)]
        all: bool,

        /// delete data older than N days (e.g. 7d, 30d)
        #[arg(long)]
        duration: Option<String>,
    },

    /// reset all stored data and configuration
    Reset,
}

#[derive(Subcommand)]
pub enum FilterListAction {
    /// add a process pattern
    Add {
        /// process name or pattern to add
        process: String,

        /// treat the pattern as a regular expression
        #[arg(short, long)]
        regex: bool,
    },

    /// remove a process pattern
    Remove {
        /// process name or pattern to remove
        process: String,
    },

    /// list all patterns
    List,

    /// clear all patterns
    Clear,
}

#[derive(Clone, ValueEnum)]
pub enum FilterMode {
    Whitelist,
    Blacklist,
    Default,
}

#[derive(Clone, ValueEnum)]
pub enum AutostartToggle {
    On,
    Off,
}
