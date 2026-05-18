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
use opagent_core::message::{Command, PendingTask, Request, Response, ResponseData};
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
