use clap::Parser;
use laches::{
    cli::{Cli, Commands, ConfigAction, DataAction},
    commands::{
        autostart::handle_autostart,
        config::{set_store_path, show_config},
        filtering::{handle_blacklist, handle_whitelist},
        list::list_processes,
        mode::set_mode,
        store_management::{confirm_delete_store, confirm_reset_store, export_store},
        tag::handle_tag_command,
    },
    process::{start_monitoring, stop_monitoring},
    store::{get_machine_id, load_or_create_store, save_store},
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

    let machine_id_path = store_path.join(".machine_id");
    if !machine_id_path.exists() {
        let _ = get_machine_id(&store_path);
    }

    match &cli.command {
        Commands::Start => start_monitoring(&mut laches_store, &store_path),
        Commands::Stop => stop_monitoring(&mut laches_store),
        Commands::List {
            tag,
            today,
            date,
            all_machines,
        } => list_processes(
            &laches_store,
            tag.as_deref(),
            *today,
            date.as_deref(),
            *all_machines,
        ),
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
        Commands::Config { action } => match action {
            ConfigAction::Show => show_config(&laches_store, &store_path),
            ConfigAction::SetStorePath { path } => set_store_path(&store_path, path),
            ConfigAction::Autostart { toggle } => handle_autostart(toggle, &store_path),
            ConfigAction::Mode { mode } => set_mode(mode, &mut laches_store),
            ConfigAction::Whitelist { action } => handle_whitelist(&mut laches_store, action),
            ConfigAction::Blacklist { action } => handle_blacklist(&mut laches_store, action),
        },
        Commands::Data { action } => match action {
            DataAction::Export {
                output,
                duration,
                all_machines,
            } => export_store(&laches_store, output, duration.as_deref(), *all_machines),
            DataAction::Delete { all, duration } => {
                confirm_delete_store(&mut laches_store, *all, duration.as_deref())
            }
            DataAction::Reset => confirm_reset_store(&store_path),
        },
    }?;

    save_store(&laches_store, &store_path)?;

    Ok(())
}
