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
    Autostart { toggle: String },
    Start,
    Stop,
    List,
    Reset,
}
