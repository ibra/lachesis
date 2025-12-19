use std::error::Error;

use crate::{process_list::ListMode, store::LachesStore};

pub fn set_mode(mode: &str, laches_store: &mut LachesStore) -> Result<(), Box<dyn Error>> {
    match mode.parse::<ListMode>() {
        Ok(variant) => {
            laches_store.process_list_options.mode = variant;
            println!(
                "info: mode set to: {}",
                laches_store.process_list_options.mode.to_str()
            );
            Ok(())
        }
        Err(_) => Err(format!("error: no match found for mode: '{}'", mode).into()),
    }
}
