use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Autostart { toggle: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Autostart { toggle } => {
            if let Some(toggle) = toggle {
                if toggle == "on" {
                    println!("Atropos will now boot on startup!")
                } else if toggle == "off" {
                    println!("Stopped Atropos from booting on startup!")
                }
            }
        }
    }
}
