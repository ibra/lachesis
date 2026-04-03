use clap::Parser;
use laches::{
    cli::{AutostartToggle, Cli, Commands, ConfigAction, DataAction, FilterMode},
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

    // ensure machine id exists for this machine
    get_machine_id(&store_path);

    let mut skip_save = false;

    match &cli.command {
        Commands::Start => start_monitoring(&store_path),
        Commands::Stop => stop_monitoring(&store_path),
        Commands::List {
            tag,
            today,
            date,
            all_machines,
        } => list_processes(
            &laches_store,
            &store_path,
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
            &store_path,
            process,
            add.as_deref(),
            remove.as_deref(),
            *list,
        ),
        Commands::Whitelist { action } => handle_whitelist(&mut laches_store, &store_path, action),
        Commands::Blacklist { action } => handle_blacklist(&mut laches_store, &store_path, action),
        Commands::Mode { mode } => {
            let mode_str = match mode {
                FilterMode::Whitelist => "whitelist",
                FilterMode::Blacklist => "blacklist",
                FilterMode::Default => "default",
            };
            set_mode(mode_str, &mut laches_store)
        }
        Commands::Autostart { toggle } => {
            let toggle_str = match toggle {
                AutostartToggle::On => "yes",
                AutostartToggle::Off => "no",
            };
            handle_autostart(&mut laches_store, toggle_str, &store_path)
        }
        Commands::Config { action } => match action {
            Some(ConfigAction::StorePath { path }) => set_store_path(&store_path, path),
            None => show_config(&laches_store, &store_path),
        },
        Commands::Data { action } => match action {
            DataAction::Export {
                output,
                duration,
                all_machines,
            } => export_store(
                &laches_store,
                &store_path,
                output,
                duration.as_deref(),
                *all_machines,
            ),
            DataAction::Delete { all, duration } => {
                confirm_delete_store(&mut laches_store, &store_path, *all, duration.as_deref())
            }
            DataAction::Reset => {
                skip_save = true;
                confirm_reset_store(&store_path)
            }
        },
    }?;

    if !skip_save {
        save_store(&laches_store, &store_path)?;
    }

    Ok(())
}
