use std::{error::Error, path::Path};

use crate::store::{get_machine_id, LachesStore};

pub fn show_config(laches_store: &LachesStore, store_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("Configuration:");
    println!("  Store path: {}", store_path.display());
    println!("  Machine ID: {}", get_machine_id(store_path));
    println!("  Autostart: {}", laches_store.autostart);
    println!("  Update interval: {}s", laches_store.update_interval);

    let mode_str = match laches_store.process_list_options.mode {
        crate::process_list::ListMode::Whitelist => "whitelist",
        crate::process_list::ListMode::Blacklist => "blacklist",
        crate::process_list::ListMode::Default => "none",
    };
    println!("  Filter mode: {}", mode_str);

    if !laches_store.machine_data.is_empty() {
        println!("\nSynced machines:");
        for (machine_id, processes) in &laches_store.machine_data {
            let total_time: u64 = processes.iter().map(|p| p.get_total_usage()).sum();
            let hours = total_time / 3600;
            let minutes = (total_time % 3600) / 60;
            println!(
                "  - {} ({} processes, {}h {}m tracked)",
                machine_id,
                processes.len(),
                hours,
                minutes
            );
        }
    }

    Ok(())
}

#[allow(unused_variables)]
pub fn set_store_path(store_path: &Path, target_path: &str) -> Result<(), Box<dyn Error>> {
    // todo: implement changing of paths
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_show_config() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();
        let store = LachesStore::default();

        let result = show_config(&store, store_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_store_path_guide() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        let result = set_store_path(store_path, "/home/user/Dropbox/laches");
        assert!(result.is_ok());
    }
}
