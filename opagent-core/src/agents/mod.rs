pub mod file;
pub mod git;
pub mod shell;

use crate::config::Config;
use crate::security::Operation;

#[derive(Debug, Clone)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub trait AgentExecutor: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, op: &Operation) -> bool;
    fn execute(&self, op: &Operation, config: &Config) -> Result<AgentResult, String>;
}
