use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub socket: SocketConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("security", &self.security)
            .field("socket", &self.socket)
            .field("logging", &self.logging)
            .field("agent", &self.agent)
            .field("llm", &self.llm)
            .finish()
    }
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

// --- LLM ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Deepseek,
    OpenAI,
    Ollama,
    Custom,
}

impl LlmProvider {
    pub fn default_base_url(&self) -> &str {
        match self {
            LlmProvider::Deepseek => "https://api.deepseek.com",
            LlmProvider::OpenAI => "https://api.openai.com/v1",
            LlmProvider::Ollama => "http://localhost:11434",
            LlmProvider::Custom => "",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: LlmProvider,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("enabled", &self.enabled)
            .finish()
    }
}

fn default_llm_provider() -> LlmProvider {
    LlmProvider::Deepseek
}

fn default_model() -> String {
    "deepseek-chat".into()
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: default_model(),
            base_url: None,
            api_key: None,
            temperature: default_temperature(),
            max_tokens: None,
            enabled: true,
        }
    }
}

impl LlmConfig {
    pub fn base_url_or_default(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| self.provider.default_base_url().into())
    }

    pub fn resolve_api_key(&self) -> Option<String> {
        if let Some(ref key) = self.api_key {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
        match self.provider {
            LlmProvider::Deepseek => std::env::var("DEEPSEEK_API_KEY").ok(),
            LlmProvider::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
            LlmProvider::Ollama => None,
            LlmProvider::Custom => std::env::var("OPAGENTD_API_KEY").ok(),
        }
    }
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
            llm: LlmConfig::default(),
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
