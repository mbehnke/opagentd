use serde::{Deserialize, Serialize};

use crate::{config::SecurityLevel, security::Operation};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    #[serde(default = "uuid")]
    pub id: String,
    pub command: Command,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Command {
    Submit {
        operation: Operation,
    },
    Approve {
        task_id: String,
    },
    Deny {
        task_id: String,
    },
    Status,
    Pending,
    Logs {
        #[serde(default)]
        count: Option<usize>,
    },
    Validate {
        operation: Operation,
    },
    Exec {
        prompt: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseData {
    #[serde(rename = "submit")]
    Submit {
        task_id: String,
        level: SecurityLevel,
        action: String,
    },
    #[serde(rename = "status")]
    Status {
        running: bool,
        uptime_secs: u64,
        tasks_pending: usize,
    },
    #[serde(rename = "pending")]
    Pending {
        tasks: Vec<PendingTask>,
    },
    #[serde(rename = "logs")]
    Logs {
        entries: Vec<String>,
    },
    #[serde(rename = "validation")]
    Validation {
        operation: String,
        level: SecurityLevel,
    },
    #[serde(rename = "approved")]
    Approved {
        task_id: String,
    },
    #[serde(rename = "denied")]
    Denied {
        task_id: String,
    },
    #[serde(rename = "executed")]
    Executed {
        task_id: String,
        success: bool,
        output: String,
    },
    #[serde(rename = "exec")]
    Exec {
        reasoning: String,
        results: Vec<ExecStep>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecStep {
    pub operation: String,
    pub level: String,
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTask {
    pub task_id: String,
    pub operation: Operation,
    pub level: SecurityLevel,
    pub created_at: u64,
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}", ts)
}
