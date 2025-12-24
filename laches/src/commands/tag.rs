use std::error::Error;

use crate::store::LachesStore;

pub fn handle_tag_command(
    laches_store: &mut LachesStore,
    process_name: &str,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
    list_tags: bool,
) -> Result<(), Box<dyn Error>> {
    let current_machine_processes = laches_store.get_current_machine_processes_mut();
    let process = current_machine_processes
        .iter_mut()
        .find(|p| p.title == process_name);

    if process.is_none() {
        return Err(format!("error: process '{}' not found", process_name).into());
    }

    let process = process.unwrap();

    if list_tags {
        if process.tags.is_empty() {
            println!("Process '{}' has no tags", process_name);
        } else {
            println!("Tags for '{}': {}", process_name, process.tags.join(", "));
        }
        return Ok(());
    }

    if let Some(tags_str) = add_tags {
        let new_tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for tag in new_tags {
            if !process.tags.contains(&tag) {
                process.tags.push(tag.clone());
                println!("Added tag '{}' to '{}'", tag, process_name);
            }
        }
    }

    if let Some(tags_str) = remove_tags {
        let remove_tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for tag in remove_tags {
            if let Some(pos) = process.tags.iter().position(|t| t == &tag) {
                process.tags.remove(pos);
                println!("Removed tag '{}' from '{}'", tag, process_name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Process;

    #[test]
    fn test_handle_tag_command_add_single_tag() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(&mut store, "test_process", Some("work"), None, false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1);
        assert_eq!(process.tags[0], "work");
    }

    #[test]
    fn test_handle_tag_command_add_multiple_tags() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(
            &mut store,
            "test_process",
            Some("work,personal,dev"),
            None,
            false,
        );
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 3);
        assert!(process.tags.contains(&"work".to_string()));
        assert!(process.tags.contains(&"personal".to_string()));
        assert!(process.tags.contains(&"dev".to_string()));
    }

    #[test]
    fn test_handle_tag_command_add_tags_with_spaces() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(
            &mut store,
            "test_process",
            Some("work , personal , dev"),
            None,
            false,
        );
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 3);
        assert!(process.tags.contains(&"work".to_string()));
        assert!(process.tags.contains(&"personal".to_string()));
        assert!(process.tags.contains(&"dev".to_string()));
    }

    #[test]
    fn test_handle_tag_command_add_duplicate_tag() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("work".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(&mut store, "test_process", Some("work"), None, false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1); // Should not add duplicate
        assert_eq!(process.tags[0], "work");
    }

    #[test]
    fn test_handle_tag_command_remove_tag() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("work".to_string());
        process.tags.push("personal".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(&mut store, "test_process", None, Some("work"), false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1);
        assert_eq!(process.tags[0], "personal");
    }

    #[test]
    fn test_handle_tag_command_remove_multiple_tags() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("work".to_string());
        process.tags.push("personal".to_string());
        process.tags.push("dev".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(&mut store, "test_process", None, Some("work,dev"), false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1);
        assert_eq!(process.tags[0], "personal");
    }

    #[test]
    fn test_handle_tag_command_remove_nonexistent_tag() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("work".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result =
            handle_tag_command(&mut store, "test_process", None, Some("nonexistent"), false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1); // Tag unchanged
        assert_eq!(process.tags[0], "work");
    }

    #[test]
    fn test_handle_tag_command_add_and_remove_simultaneously() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("old_tag".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(
            &mut store,
            "test_process",
            Some("new_tag"),
            Some("old_tag"),
            false,
        );
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 1);
        assert_eq!(process.tags[0], "new_tag");
    }

    #[test]
    fn test_handle_tag_command_process_not_found() {
        let mut store = LachesStore::default();

        let result =
            handle_tag_command(&mut store, "nonexistent_process", Some("work"), None, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("process 'nonexistent_process' not found"));
    }

    #[test]
    fn test_handle_tag_command_empty_tag_string() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let process = Process::new("test_process".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(&mut store, "test_process", Some(""), None, false);
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 0); // Empty strings filtered out
    }

    #[test]
    fn test_handle_tag_command_list_tags_empty() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let process = Process::new("test_process".to_string());
        store.machine_data.insert(hostname, vec![process]);

        let result = handle_tag_command(&mut store, "test_process", None, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_tag_command_list_tags_with_tags() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let mut process = Process::new("test_process".to_string());
        process.tags.push("work".to_string());
        process.tags.push("dev".to_string());
        store.machine_data.insert(hostname, vec![process]);

        let result = handle_tag_command(&mut store, "test_process", None, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_tag_command_tags_with_commas_and_spaces() {
        let mut store = LachesStore::default();
        let hostname = crate::store::get_hostname();
        let process = Process::new("test_process".to_string());
        store.machine_data.insert(hostname.clone(), vec![process]);

        let result = handle_tag_command(
            &mut store,
            "test_process",
            Some(" , tag1 ,, tag2 , "),
            None,
            false,
        );
        assert!(result.is_ok());

        let process = &store.machine_data.get(&hostname).unwrap()[0];
        assert_eq!(process.tags.len(), 2);
        assert!(process.tags.contains(&"tag1".to_string()));
        assert!(process.tags.contains(&"tag2".to_string()));
    }
}
