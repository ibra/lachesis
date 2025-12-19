use clap::Parser;
use laches::{
    cli::{Cli, Commands},
    commands::{
        autostart::handle_autostart,
        filtering::{handle_blacklist, handle_whitelist},
        list::list_processes,
        mode::set_mode,
        store_management::{confirm_delete_store, confirm_reset_store, export_store},
        tag::handle_tag_command,
    },
    process::{start_monitoring, stop_monitoring},
    store::{load_or_create_store, save_store},
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let store_path = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => return Err("error: failed to get configuration directory".into()),
    };
    std::fs::create_dir_all(&store_path)?;

    let mut laches_store = load_or_create_store(&store_path)?;
    let cli = Cli::parse();

    match &cli.command {
        Commands::Autostart { toggle } => handle_autostart(toggle, &store_path),
        Commands::Start => start_monitoring(&mut laches_store, &store_path),
        Commands::Stop => stop_monitoring(&mut laches_store),
        Commands::Mode { mode } => set_mode(mode, &mut laches_store),
        Commands::List { tag, today, date } => {
            list_processes(&laches_store, tag.as_deref(), *today, date.as_deref())
        }
        Commands::Tag {
            process,
            add,
            remove,
            list,
        } => handle_tag_command(
            &mut laches_store,
            process,
            add.as_deref(),
            remove.as_deref(),
            *list,
        ),
        Commands::Reset => confirm_reset_store(&store_path),
        Commands::Delete { all, duration } => {
            confirm_delete_store(&mut laches_store, *all, duration.as_deref())
        }
        Commands::Export { output, duration } => {
            export_store(&laches_store, output, duration.as_deref())
        }
        Commands::Whitelist { action } => handle_whitelist(&mut laches_store, action),
        Commands::Blacklist { action } => handle_blacklist(&mut laches_store, action),
    }?;

    save_store(&laches_store, &store_path)?;

    Ok(())
}
