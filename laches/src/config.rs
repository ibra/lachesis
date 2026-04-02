use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;
use uuid::Uuid;

const CONFIG_NAME: &str = "config.toml";
const PID_FILE: &str = ".daemon_pid";

/// Top-level configuration, stored as config.toml.
/// This is separate from the data (SQLite) -- config is small, rarely changes,
/// and should not be mixed with time-series data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub filtering: FilteringConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaemonConfig {
    /// How often the daemon checks the focused window (seconds).
    pub check_interval: u64,
    /// Seconds of no input before the user is considered idle.
    pub idle_timeout: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilteringConfig {
    /// "default", "whitelist", or "blacklist".
    pub mode: String,
    #[serde(default)]
    pub whitelist: Vec<String>,
    #[serde(default)]
    pub blacklist: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                check_interval: 2,
                idle_timeout: 300,
            },
            filtering: FilteringConfig {
                mode: "default".to_string(),
                whitelist: Vec::new(),
                blacklist: Vec::new(),
            },
        }
    }
}

/// Load config from disk, or create the default if it doesn't exist.
pub fn load_or_create_config(config_dir: &Path) -> Result<Config, Box<dyn Error>> {
    let config_path = config_dir.join(CONFIG_NAME);

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        let config = Config::default();
        save_config(&config, config_dir)?;
        println!("info: created default config at {}", config_path.display());
        Ok(config)
    }
}

/// Write config to disk.
pub fn save_config(config: &Config, config_dir: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(config_dir)?;
    let config_path = config_dir.join(CONFIG_NAME);
    let content = toml::to_string_pretty(config)?;
    fs::write(&config_path, content)?;
    Ok(())
}

/// Read the daemon PID from the pid file. Returns None if not running.
pub fn read_daemon_pid(config_dir: &Path) -> Option<u32> {
    let pid_path = config_dir.join(PID_FILE);
    let content = fs::read_to_string(&pid_path).ok()?;
    content.trim().parse().ok()
}

/// Write the daemon PID to the pid file.
pub fn write_daemon_pid(config_dir: &Path, pid: u32) -> Result<(), Box<dyn Error>> {
    let pid_path = config_dir.join(PID_FILE);
    fs::write(&pid_path, pid.to_string())?;
    Ok(())
}

/// Remove the daemon PID file.
pub fn clear_daemon_pid(config_dir: &Path) {
    let pid_path = config_dir.join(PID_FILE);
    let _ = fs::remove_file(&pid_path);
}

/// Get the hostname of the current machine.
pub fn get_hostname() -> String {
    if let Ok(hostname) = std::env::var("COMPUTERNAME") {
        return hostname;
    }
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        return hostname;
    }

    #[cfg(unix)]
    {
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            let trimmed = hostname.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "unknown".to_string()
}

/// Get a stable machine identifier. Generated once, stored in .machine_id.
pub fn get_machine_id(config_dir: &Path) -> String {
    let machine_id_file = config_dir.join(".machine_id");

    if let Ok(existing_id) = std::fs::read_to_string(&machine_id_file) {
        let trimmed = existing_id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let hostname = get_hostname();
    let uuid = Uuid::new_v4();
    let machine_id = format!("{}_{}", hostname, uuid);

    if let Err(e) = std::fs::create_dir_all(config_dir) {
        eprintln!("warning: failed to create config directory: {}", e);
    }
    if let Err(e) = std::fs::write(&machine_id_file, &machine_id) {
        eprintln!("warning: failed to write machine id file: {}", e);
    }

    machine_id
}

/// Get the data directory for per-machine database files.
pub fn data_dir(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join("data")
}

/// Get the database path for the current machine.
pub fn machine_db_path(config_dir: &Path, machine_id: &str) -> std::path::PathBuf {
    data_dir(config_dir).join(format!("{}.db", machine_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.daemon.check_interval, 2);
        assert_eq!(config.daemon.idle_timeout, 300);
        assert_eq!(config.filtering.mode, "default");
        assert!(config.filtering.whitelist.is_empty());
        assert!(config.filtering.blacklist.is_empty());
    }

    #[test]
    fn test_save_and_load_config() {
        let tmp = TempDir::new().unwrap();
        let config = Config::default();

        save_config(&config, tmp.path()).unwrap();
        let loaded = load_or_create_config(tmp.path()).unwrap();

        assert_eq!(loaded.daemon.check_interval, config.daemon.check_interval);
        assert_eq!(loaded.filtering.mode, config.filtering.mode);
    }

    #[test]
    fn test_load_creates_default() {
        let tmp = TempDir::new().unwrap();

        let config = load_or_create_config(tmp.path()).unwrap();
        assert_eq!(config.daemon.check_interval, 2);
        assert!(tmp.path().join(CONFIG_NAME).exists());
    }

    #[test]
    fn test_config_roundtrip_with_data() {
        let tmp = TempDir::new().unwrap();

        let mut config = Config::default();
        config.filtering.mode = "whitelist".to_string();
        config.filtering.whitelist = vec!["firefox".to_string(), "code".to_string()];
        config.daemon.idle_timeout = 600;

        save_config(&config, tmp.path()).unwrap();
        let loaded = load_or_create_config(tmp.path()).unwrap();

        assert_eq!(loaded.filtering.mode, "whitelist");
        assert_eq!(loaded.filtering.whitelist, vec!["firefox", "code"]);
        assert_eq!(loaded.daemon.idle_timeout, 600);
    }

    #[test]
    fn test_daemon_pid_lifecycle() {
        let tmp = TempDir::new().unwrap();

        assert!(read_daemon_pid(tmp.path()).is_none());

        write_daemon_pid(tmp.path(), 12345).unwrap();
        assert_eq!(read_daemon_pid(tmp.path()), Some(12345));

        clear_daemon_pid(tmp.path());
        assert!(read_daemon_pid(tmp.path()).is_none());
    }

    #[test]
    fn test_machine_db_path() {
        let tmp = TempDir::new().unwrap();
        let path = machine_db_path(tmp.path(), "IBBY_abc123");
        assert!(path.to_str().unwrap().contains("data"));
        assert!(path.to_str().unwrap().ends_with("IBBY_abc123.db"));
    }
}
