use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use opagent_core::agents::file::FileAgent;
use opagent_core::agents::git::GitAgent;
use opagent_core::agents::shell::ShellAgent;
use opagent_core::agents::{AgentExecutor, AgentResult};
use opagent_core::config::{Config, SecurityLevel};
use opagent_core::message::{Command, ExecStep, PendingTask, Request, Response, ResponseData};
use opagent_core::security::Operation;

type AgentBox = Arc<dyn AgentExecutor>;

struct Daemon {
    config: Config,
    agents: Vec<AgentBox>,
    pending: Mutex<Vec<PendingTask>>,
    started: Instant,
}

impl Daemon {
    fn new(config: Config) -> Self {
        let agents: Vec<AgentBox> = vec![
            Arc::new(ShellAgent),
            Arc::new(FileAgent),
            Arc::new(GitAgent),
        ];
        Self {
            config,
            agents,
            pending: Mutex::new(Vec::new()),
            started: Instant::now(),
        }
    }

    fn find_agent(&self, op: &Operation) -> Option<&AgentBox> {
        self.agents.iter().find(|a| a.can_handle(op))
    }

    async fn handle_request(&self, req: Request) -> Response {
        let id = req.id;
        match req.command {
            Command::Submit { operation } => self.handle_submit(id, operation).await,
            Command::Approve { task_id } => self.handle_approve(id, task_id).await,
            Command::Deny { task_id } => self.handle_deny(id, task_id).await,
            Command::Status => self.handle_status(id),
            Command::Pending => self.handle_pending(id).await,
            Command::Logs { count } => self.handle_logs(id, count),
            Command::Validate { operation } => self.handle_validate(id, operation),
            Command::Exec { prompt } => self.handle_exec(id, prompt).await,
        }
    }

    async fn handle_submit(&self, id: String, op: Operation) -> Response {
        let level = op.security_level(&self.config);
        let desc = op.describe();

        info!(operation = %desc, level = ?level, "Submit");

        match level {
            SecurityLevel::Deny => {
                warn!(operation = %desc, "Denied");
                Response {
                    id,
                    status: "error".into(),
                    data: None,
                    message: Some(format!("Operation denied by policy: {}", desc)),
                }
            }
            SecurityLevel::Auto => {
                info!(operation = %desc, "Auto-executing");
                let task_id = gen_task_id();
                match self.execute_operation(&op) {
                    Ok(result) => Response {
                        id,
                        status: "ok".into(),
                        data: Some(ResponseData::Executed {
                            task_id,
                            success: result.success,
                            output: result.output,
                        }),
                        message: None,
                    },
                    Err(e) => Response {
                        id,
                        status: "error".into(),
                        data: None,
                        message: Some(e),
                    },
                }
            }
            SecurityLevel::Confirm => {
                info!(operation = %desc, "Queued for approval");
                let task_id = gen_task_id();
                self.pending.lock().await.push(PendingTask {
                    task_id: task_id.clone(),
                    operation: op,
                    level: level.clone(),
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
                Response {
                    id,
                    status: "ok".into(),
                    data: Some(ResponseData::Submit {
                        task_id,
                        level,
                        action: "Awaiting approval".into(),
                    }),
                    message: None,
                }
            }
        }
    }

    async fn handle_approve(&self, id: String, task_id: String) -> Response {
        let mut pending = self.pending.lock().await;
        if let Some(pos) = pending.iter().position(|t| t.task_id == task_id) {
            let task = pending.remove(pos);
            let desc = task.operation.describe();
            info!(task_id = %task.task_id, operation = %desc, "Approving and executing");
            match self.execute_operation(&task.operation) {
                Ok(result) => Response {
                    id,
                    status: "ok".into(),
                    data: Some(ResponseData::Executed {
                        task_id: task.task_id,
                        success: result.success,
                        output: result.output,
                    }),
                    message: None,
                },
                Err(e) => Response {
                    id,
                    status: "error".into(),
                    data: None,
                    message: Some(e),
                },
            }
        } else {
            Response {
                id,
                status: "error".into(),
                data: None,
                message: Some(format!("Task {} not found in pending queue", task_id)),
            }
        }
    }

    async fn handle_deny(&self, id: String, task_id: String) -> Response {
        let mut pending = self.pending.lock().await;
        if let Some(pos) = pending.iter().position(|t| t.task_id == task_id) {
            let task = pending.remove(pos);
            info!(task_id = %task.task_id, operation = %task.operation.describe(), "Denied");
            Response {
                id,
                status: "ok".into(),
                data: Some(ResponseData::Denied {
                    task_id: task.task_id,
                }),
                message: None,
            }
        } else {
            Response {
                id,
                status: "error".into(),
                data: None,
                message: Some(format!("Task {} not found in pending queue", task_id)),
            }
        }
    }

    fn handle_status(&self, id: String) -> Response {
        Response {
            id,
            status: "ok".into(),
            data: Some(ResponseData::Status {
                running: true,
                uptime_secs: self.started.elapsed().as_secs(),
                tasks_pending: 0,
            }),
            message: None,
        }
    }

    async fn handle_pending(&self, id: String) -> Response {
        let pending = self.pending.lock().await;
        Response {
            id,
            status: "ok".into(),
            data: Some(ResponseData::Pending {
                tasks: pending.clone(),
            }),
            message: None,
        }
    }

    fn handle_logs(&self, id: String, _count: Option<usize>) -> Response {
        Response {
            id,
            status: "ok".into(),
            data: Some(ResponseData::Logs {
                entries: vec!["Audit logging not yet implemented".into()],
            }),
            message: None,
        }
    }

    fn handle_validate(&self, id: String, op: Operation) -> Response {
        let level = op.security_level(&self.config);
        let desc = op.describe();
        Response {
            id,
            status: "ok".into(),
            data: Some(ResponseData::Validation {
                operation: desc,
                level,
            }),
            message: None,
        }
    }

    async fn handle_exec(&self, id: String, prompt: String) -> Response {
        if !self.config.llm.enabled {
            return Response {
                id,
                status: "error".into(),
                data: None,
                message: Some("LLM is disabled in config (llm.enabled = false)".into()),
            };
        }

        let api_key = match self.config.llm.resolve_api_key() {
            Some(k) => k,
            None => {
                return Response {
                    id,
                    status: "error".into(),
                    data: None,
                    message: Some("No API key configured. Set api_key in [llm] or DEEPSEEK_API_KEY env var.".into()),
                };
            }
        };

        info!(prompt = %prompt, "LLM exec");

        let llm_response = match call_llm(&self.config.llm.model,
                                            &self.config.llm.base_url_or_default(),
                                            &api_key,
                                            &prompt).await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("LLM call failed: {}", e);
                return Response {
                    id,
                    status: "error".into(),
                    data: None,
                    message: Some(format!("LLM call failed: {}", e)),
                };
            }
        };

        let ops: Vec<Operation> = match parse_operations(&llm_response) {
            Ok(ops) => ops,
            Err(e) => {
                return Response {
                    id,
                    status: "error".into(),
                    data: None,
                    message: Some(format!("Failed to parse LLM response: {} — raw: {}", e, llm_response)),
                };
            }
        };

        let mut results: Vec<ExecStep> = Vec::new();
        for op in &ops {
            let desc = op.describe();
            let level = op.security_level(&self.config);
            info!(operation = %desc, level = ?level, "LLM planned");

            match level {
                SecurityLevel::Deny => {
                    results.push(ExecStep {
                        operation: desc,
                        level: "deny".into(),
                        success: false,
                        output: "Blocked by security policy".into(),
                    });
                }
                SecurityLevel::Auto => {
                    let step_result = match self.execute_operation(op) {
                        Ok(r) => ExecStep {
                            operation: desc,
                            level: "auto".into(),
                            success: r.success,
                            output: r.output,
                        },
                        Err(e) => ExecStep {
                            operation: desc,
                            level: "auto".into(),
                            success: false,
                            output: e,
                        },
                    };
                    results.push(step_result);
                }
                SecurityLevel::Confirm => {
                    let task_id = gen_task_id();
                    self.pending.lock().await.push(PendingTask {
                        task_id,
                        operation: op.clone(),
                        level,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    });
                    results.push(ExecStep {
                        operation: desc,
                        level: "confirm".into(),
                        success: false,
                        output: "Queued for approval".into(),
                    });
                }
            }
        }

        Response {
            id,
            status: "ok".into(),
            data: Some(ResponseData::Exec {
                reasoning: if ops.is_empty() {
                    "No operations generated".into()
                } else {
                    format!("{} operation(s) planned by LLM", ops.len())
                },
                results,
            }),
            message: None,
        }
    }

    fn execute_operation(&self, op: &Operation) -> Result<AgentResult, String> {
        let agent = self
            .find_agent(op)
            .ok_or_else(|| format!("No agent available for: {:?}", op))?;
        agent.execute(op, &self.config)
    }
}

fn gen_task_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task_{:016x}", ts)
}

// --- LLM Integration ---

const SYSTEM_PROMPT: &str = r#"You are a Linux system agent. Given a user's request in natural language, respond with a JSON array of operations to execute.

Available operations:
- {"Shell": {"command": "...", "args": [...]}} — run a shell command
- {"FileRead": {"path": "..."}} — read a file
- {"FileWrite": {"path": "...", "content": "..."}} — write a file
- {"FileDelete": {"path": "..."}} — delete a file
- {"Git": {"command": "...", "repo_path": "..."}} — run a git command

Rules:
1. Output ONLY the JSON array of operations, nothing else.
2. Break complex tasks into minimal, focused steps.
3. Use absolute paths when possible.
4. For file reads, you may use read-only safety: prefer less dangerous paths.
5. Never include rm -rf /, dd, mkfs, or > /dev/sda.
6. If the task cannot be done as operations, respond with an empty array [] and a short explanation in a comment.
"#;

async fn call_llm(model: &str, base_url: &str, api_key: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.3,
        "max_tokens": 2048,
    });

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;

    if !status.is_success() {
        return Err(format!("API {} {}: {}", status.as_u16(), status.canonical_reason().unwrap_or(""), text));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON parse: {}", e))?;

    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected API response structure: {}", text))?;

    Ok(content.to_string())
}

fn parse_operations(content: &str) -> Result<Vec<Operation>, String> {
    let extracted = extract_json_array(content)?;
    let ops: Vec<Operation> =
        serde_json::from_str(&extracted).map_err(|e| format!("Parse: {}", e))?;
    Ok(ops)
}

fn extract_json_array(text: &str) -> Result<String, String> {
    // The LLM might wrap JSON in markdown code blocks. Extract just the JSON array.
    let trimmed = text.trim();

    // Strip markdown code fences if present
    let stripped = if let Some(inner) = trimmed.strip_prefix("```json") {
        inner.strip_suffix("```").unwrap_or(inner).trim()
    } else if let Some(inner) = trimmed.strip_prefix("```") {
        inner.strip_suffix("```").unwrap_or(inner).trim()
    } else {
        trimmed
    };

    // Find the JSON array
    let start = stripped.find('[').ok_or_else(|| format!("No JSON array found in: {}", text))?;
    let end = stripped.rfind(']').ok_or_else(|| format!("Unclosed JSON array in: {}", text))?;

    Ok(stripped[start..=end].to_string())
}

// --- Connection handling ---

async fn handle_connection(stream: UnixStream, daemon: Arc<Daemon>) {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                error!("Socket read error: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let resp = Response {
                    id: "unknown".into(),
                    status: "error".into(),
                    data: None,
                    message: Some(format!("Invalid JSON: {}", e)),
                };
                let mut json = serde_json::to_string(&resp).unwrap_or_default();
                json.push('\n');
                let _ = writer.write_all(json.as_bytes()).await;
                continue;
            }
        };

        let response = daemon.handle_request(request).await;
        let mut json = serde_json::to_string(&response).unwrap_or_default();
        json.push('\n');
        if let Err(e) = writer.write_all(json.as_bytes()).await {
            error!("Socket write error: {}", e);
            break;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let paths = Config::default_paths();
    let config = Config::load_or_default(&paths);
    let socket_path = config.socket.path.clone();

    info!("Loading config from {:?}", paths);

    let daemon = Arc::new(Daemon::new(config));

    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .context("Failed to create runtime directory")?;
        }
    }

    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("Failed to bind to {}", socket_path))?;

    info!("Listening on {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let daemon = daemon.clone();
                tokio::spawn(async move {
                    handle_connection(stream, daemon).await;
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}
