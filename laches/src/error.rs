use std::fmt;

#[derive(Debug)]
pub enum LachesError {
    Config(String),
    Database(rusqlite::Error),
    Io(std::io::Error),
    InvalidInput(String),
}

impl fmt::Display for LachesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "config error: {}", msg),
            Self::Database(e) => write!(f, "database error: {}", e),
            Self::Io(e) => write!(f, "io error: {}", e),
            Self::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
        }
    }
}

impl std::error::Error for LachesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for LachesError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for LachesError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for LachesError {
    fn from(e: toml::de::Error) -> Self {
        Self::Config(e.to_string())
    }
}

impl From<toml::ser::Error> for LachesError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Config(e.to_string())
    }
}

impl From<String> for LachesError {
    fn from(s: String) -> Self {
        Self::InvalidInput(s)
    }
}

impl From<&str> for LachesError {
    fn from(s: &str) -> Self {
        Self::InvalidInput(s.to_string())
    }
}
