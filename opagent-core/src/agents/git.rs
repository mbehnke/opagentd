use std::process::Command;

use crate::agents::{AgentExecutor, AgentResult};
use crate::config::Config;
use crate::security::Operation;

pub struct GitAgent;

impl AgentExecutor for GitAgent {
    fn name(&self) -> &'static str {
        "git"
    }

    fn can_handle(&self, op: &Operation) -> bool {
        matches!(op, Operation::Git { .. })
    }

    fn execute(&self, op: &Operation, _config: &Config) -> Result<AgentResult, String> {
        let (command, repo_path) = match op {
            Operation::Git {
                command,
                repo_path,
            } => (command, repo_path),
            _ => return Err("Operation is not a Git variant".into()),
        };

        let output = Command::new("git")
            .args(command.split_whitespace())
            .current_dir(repo_path)
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(AgentResult {
            success: output.status.success(),
            output: if output.status.success() {
                stdout
            } else {
                stderr.clone()
            },
            error: if output.status.success() {
                None
            } else {
                Some(stderr)
            },
        })
    }
}
