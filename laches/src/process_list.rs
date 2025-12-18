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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_list_options_default() {
        let options = ProcessListOptions::default();
        assert!(matches!(options.mode, ListMode::Default));
        assert!(options.whitelist.is_none());
        assert!(options.blacklist.is_none());
        assert!(options.tags.is_none());
    }

    #[test]
    fn test_list_mode_from_str_whitelist() {
        let mode = ListMode::from_str("whitelist").unwrap();
        assert!(matches!(mode, ListMode::Whitelist));
    }

    #[test]
    fn test_list_mode_from_str_blacklist() {
        let mode = ListMode::from_str("blacklist").unwrap();
        assert!(matches!(mode, ListMode::Blacklist));
    }

    #[test]
    fn test_list_mode_from_str_default() {
        let mode = ListMode::from_str("default").unwrap();
        assert!(matches!(mode, ListMode::Default));
    }

    #[test]
    fn test_list_mode_from_str_invalid() {
        let result = ListMode::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_mode_to_str_whitelist() {
        let mode = ListMode::Whitelist;
        assert_eq!(mode.to_str(), "whitelist");
    }

    #[test]
    fn test_list_mode_to_str_blacklist() {
        let mode = ListMode::Blacklist;
        assert_eq!(mode.to_str(), "blacklist");
    }

    #[test]
    fn test_list_mode_to_str_default() {
        let mode = ListMode::Default;
        assert_eq!(mode.to_str(), "default");
    }

    #[test]
    fn test_list_mode_roundtrip() {
        let modes = vec!["whitelist", "blacklist", "default"];

        for mode_str in modes {
            let mode = ListMode::from_str(mode_str).unwrap();
            assert_eq!(mode.to_str(), mode_str);
        }
    }

    #[test]
    fn test_process_list_options_serialization() {
        let mut options = ProcessListOptions::default();
        options.whitelist = Some(vec!["process1".to_string(), "process2".to_string()]);
        options.tags = Some(vec!["work".to_string()]);

        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: ProcessListOptions = serde_json::from_str(&serialized).unwrap();

        assert!(matches!(deserialized.mode, ListMode::Default));
        assert_eq!(deserialized.whitelist.unwrap().len(), 2);
        assert_eq!(deserialized.tags.unwrap().len(), 1);
    }

    #[test]
    fn test_process_list_options_with_blacklist() {
        let mut options = ProcessListOptions::default();
        options.mode = ListMode::Blacklist;
        options.blacklist = Some(vec!["unwanted1".to_string(), "unwanted2".to_string()]);

        assert!(matches!(options.mode, ListMode::Blacklist));
        assert_eq!(options.blacklist.as_ref().unwrap().len(), 2);
        assert_eq!(options.blacklist.as_ref().unwrap()[0], "unwanted1");
    }
}
