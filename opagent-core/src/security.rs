use serde::{Deserialize, Serialize};

use crate::config::{Config, SecurityLevel};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    Shell {
        command: String,
        args: Vec<String>,
    },
    FileRead {
        path: String,
    },
    FileWrite {
        path: String,
        content: String,
    },
    FileDelete {
        path: String,
    },
    Git {
        command: String,
        repo_path: String,
    },
}

impl Operation {
    pub fn agent_name(&self) -> &'static str {
        match self {
            Operation::Shell { .. } => "shell",
            Operation::FileRead { .. }
            | Operation::FileWrite { .. }
            | Operation::FileDelete { .. } => "file",
            Operation::Git { .. } => "git",
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Operation::Shell { command, args } => {
                format!("shell: {} {}", command, args.join(" "))
            }
            Operation::FileRead { path } => format!("file-read: {}", path),
            Operation::FileWrite { path, .. } => format!("file-write: {}", path),
            Operation::FileDelete { path } => format!("file-delete: {}", path),
            Operation::Git { command, repo_path } => {
                format!("git: {} (repo: {})", command, repo_path)
            }
        }
    }

    pub fn security_level(&self, config: &Config) -> SecurityLevel {
        match self {
            Operation::Shell { command, args } => {
                let full_cmd = format!("{} {}", command, args.join(" "));
                for denied in &config.security.denied_commands {
                    if full_cmd.contains(denied) || command == denied {
                        return SecurityLevel::Deny;
                    }
                }
                config.security.default_level.clone()
            }
            Operation::FileRead { .. }
            | Operation::FileWrite { .. }
            | Operation::FileDelete { .. } => {
                let path = self.target_path();
                Self::path_security_level(path, config)
            }
            Operation::Git { command, .. } => {
                let dangerous: &[&str] = &[
                    "push --force",
                    "hard reset",
                    "rebase --onto",
                    "clean -fd",
                ];
                for d in dangerous {
                    if command.contains(d) {
                        return SecurityLevel::Confirm;
                    }
                }
                SecurityLevel::Auto
            }
        }
    }

    fn target_path(&self) -> &str {
        match self {
            Operation::FileRead { path }
            | Operation::FileWrite { path, .. }
            | Operation::FileDelete { path } => path,
            _ => "",
        }
    }

    fn path_security_level(path: &str, config: &Config) -> SecurityLevel {
        if path.is_empty() {
            return config.security.default_level.clone();
        }

        let check_str = if let Ok(canonical) = std::fs::canonicalize(path) {
            canonical.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let allowed = config
            .security
            .allowed_paths
            .iter()
            .any(|ap| check_str.starts_with(ap));

        if !allowed {
            return SecurityLevel::Deny;
        }
        config.security.default_level.clone()
    }
}
