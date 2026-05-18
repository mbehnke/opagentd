use std::fs;
use std::path::Path;

use crate::agents::{AgentExecutor, AgentResult};
use crate::config::Config;
use crate::security::Operation;

pub struct FileAgent;

impl AgentExecutor for FileAgent {
    fn name(&self) -> &'static str {
        "file"
    }

    fn can_handle(&self, op: &Operation) -> bool {
        matches!(
            op,
            Operation::FileRead { .. } | Operation::FileWrite { .. } | Operation::FileDelete { .. }
        )
    }

    fn execute(&self, op: &Operation, _config: &Config) -> Result<AgentResult, String> {
        match op {
            Operation::FileRead { path } => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read {}: {}", path, e))?;
                Ok(AgentResult {
                    success: true,
                    output: content,
                    error: None,
                })
            }
            Operation::FileWrite { path, content } => {
                if let Some(parent) = Path::new(path).parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
                fs::write(path, content)
                    .map_err(|e| format!("Failed to write {}: {}", path, e))?;
                Ok(AgentResult {
                    success: true,
                    output: "ok".into(),
                    error: None,
                })
            }
            Operation::FileDelete { path } => {
                fs::remove_file(path)
                    .map_err(|e| format!("Failed to delete {}: {}", path, e))?;
                Ok(AgentResult {
                    success: true,
                    output: "deleted".into(),
                    error: None,
                })
            }
            _ => Err("Operation is not a File variant".into()),
        }
    }
}
