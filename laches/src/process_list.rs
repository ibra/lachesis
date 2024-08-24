use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ProcessListOptions {
    pub mode: ListMode,
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl Default for ProcessListOptions {
    fn default() -> Self {
        Self {
            mode: ListMode::Default,
            whitelist: None,
            blacklist: None,
            tags: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum ListMode {
    Whitelist,
    Blacklist,
    Default,
}
