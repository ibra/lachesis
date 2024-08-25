use std::str::FromStr;

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

impl FromStr for ListMode {
    type Err = ();

    fn from_str(input: &str) -> Result<ListMode, Self::Err> {
        match input {
            "whitelist" => Ok(ListMode::Whitelist),
            "blacklist" => Ok(ListMode::Blacklist),
            "default" => Ok(ListMode::Default),
            _ => Err(()),
        }
    }
}

impl ListMode {
    pub fn to_str(&self) -> &'static str {
        match self {
            ListMode::Whitelist => "whitelist",
            ListMode::Blacklist => "blacklist",
            ListMode::Default => "default",
        }
    }
}
