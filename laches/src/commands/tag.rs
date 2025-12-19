use std::error::Error;

use crate::store::LachesStore;

pub fn handle_tag_command(
    laches_store: &mut LachesStore,
    process_name: &str,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
    list_tags: bool,
) -> Result<(), Box<dyn Error>> {
    let process = laches_store
        .process_information
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
