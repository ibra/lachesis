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
    Start,
    Stop,
    List {
        #[arg(short, long)]
        tag: Option<String>,
        #[arg(short = 'd', long)]
        today: bool,
        #[arg(long)]
        date: Option<String>,
        #[arg(short = 'a', long)]
        all_machines: bool,
    },
    Tag {
        process: String,
        #[arg(short, long)]
        add: Option<String>,
        #[arg(short, long)]
        remove: Option<String>,
        #[arg(short, long)]
        list: bool,
    },
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Data {
        #[command(subcommand)]
        action: DataAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Show,
    SetStorePath {
        path: String,
    },
    Autostart {
        toggle: String, // yes or no
    },
    Mode {
        mode: String,
    },
    Whitelist {
        #[command(subcommand)]
        action: FilterListAction,
    },
    Blacklist {
        #[command(subcommand)]
        action: FilterListAction,
    },
}

#[derive(Subcommand)]
pub enum DataAction {
    Export {
        output: String,
        #[arg(long)]
        duration: Option<String>,
        #[arg(short = 'a', long)]
        all_machines: bool,
    },
    Delete {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        duration: Option<String>,
    },
    Reset,
}

#[derive(Subcommand)]
pub enum FilterListAction {
    Add {
        process: String,
        #[arg(short, long)]
        regex: bool,
    },
    Remove {
        process: String,
    },
    List,
    Clear,
}
