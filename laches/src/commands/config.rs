use std::{error::Error, fs, path::Path};

use crate::store::{get_machine_id, LachesStore, STORE_NAME};

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

pub fn set_store_path(store_path: &Path, target_path: &str) -> Result<(), Box<dyn Error>> {
    let target = Path::new(target_path);

    if target == store_path {
        println!("info: store path is already set to '{}'", target_path);
        return Ok(());
    }

    fs::create_dir_all(target)?;

    let source_store = store_path.join(STORE_NAME);
    let target_store = target.join(STORE_NAME);

    if source_store.exists() {
        fs::copy(&source_store, &target_store)?;
        println!(
            "info: copied store from '{}' to '{}'",
            source_store.display(),
            target_store.display()
        );
    }

    let source_machine_id = store_path.join(".machine_id");
    let target_machine_id = target.join(".machine_id");

    if source_machine_id.exists() {
        fs::copy(&source_machine_id, &target_machine_id)?;
    }

    println!(
        "info: store path set to '{}'. you can remove the old directory at '{}'",
        target_path,
        store_path.display()
    );

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
    fn test_set_store_path_copies_data() {
        let source_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let source_path = source_dir.path();
        let target_path = target_dir.path().join("new_store");

        // Create a store file in the source
        let store = LachesStore::default();
        crate::store::save_store(&store, source_path).unwrap();
        assert!(source_path.join(STORE_NAME).exists());

        let result = set_store_path(source_path, target_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(target_path.join(STORE_NAME).exists());
    }

    #[test]
    fn test_set_store_path_same_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        let result = set_store_path(store_path, store_path.to_str().unwrap());
        assert!(result.is_ok());
    }
}
