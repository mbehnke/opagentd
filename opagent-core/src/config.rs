use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub socket: SocketConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    #[serde(default = "default_level")]
    pub default_level: SecurityLevel,
    #[serde(default = "default_allowed_paths")]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub denied_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    Auto,
    Confirm,
    Deny,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SocketConfig {
    #[serde(default = "default_socket_path")]
    pub path: String,
    #[serde(default = "default_socket_perms")]
    pub permissions: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub audit_log: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    #[serde(default = "default_true")]
    pub shell_enabled: bool,
    #[serde(default = "default_true")]
    pub file_enabled: bool,
    #[serde(default = "default_true")]
    pub git_enabled: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            shell_enabled: true,
            file_enabled: true,
            git_enabled: true,
        }
    }
}

// --- Defaults ---

fn default_level() -> SecurityLevel {
    SecurityLevel::Confirm
}
fn default_socket_path() -> String {
    "/run/opagentd/opagentd.sock".into()
}
fn default_socket_perms() -> u32 {
    0o660
}
fn default_log_level() -> String {
    "info".into()
}
fn default_allowed_paths() -> Vec<String> {
    vec!["/home".into(), "/tmp".into(), "/var/tmp".into()]
}
fn default_true() -> bool {
    true
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_level: default_level(),
            allowed_paths: default_allowed_paths(),
            denied_commands: Vec::new(),
        }
    }
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            path: default_socket_path(),
            permissions: default_socket_perms(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            audit_log: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            security: SecurityConfig::default(),
            socket: SocketConfig::default(),
            logging: LoggingConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

// --- Loading ---

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(ConfigError::Io)?;
        toml::from_str(&content).map_err(ConfigError::Parse)
    }

    pub fn load_or_default(paths: &[PathBuf]) -> Config {
        for path in paths {
            if path.exists() {
                match Config::load(path) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        tracing::warn!("Failed to load config {:?}: {}", path, e);
                    }
                }
            }
        }
        tracing::info!("No config found, using defaults");
        Config::default()
    }

    pub fn default_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/etc/opagentd/config.toml"),
            PathBuf::from("./config/opagentd.toml"),
        ]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),
}
