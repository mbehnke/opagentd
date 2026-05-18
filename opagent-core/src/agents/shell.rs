use std::process::Command;

use crate::agents::{AgentExecutor, AgentResult};
use crate::config::Config;
use crate::security::Operation;

pub struct ShellAgent;

impl AgentExecutor for ShellAgent {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn can_handle(&self, op: &Operation) -> bool {
        matches!(op, Operation::Shell { .. })
    }

    fn execute(&self, op: &Operation, _config: &Config) -> Result<AgentResult, String> {
        let (command, args) = match op {
            Operation::Shell { command, args } => (command, args),
            _ => return Err("Operation is not a Shell variant".into()),
        };

        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

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
