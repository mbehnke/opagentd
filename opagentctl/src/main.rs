use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use opagent_core::message::{Command, Request, Response};
use opagent_core::security::Operation;

#[derive(Parser)]
#[command(name = "opagentctl", version, about = "Control client for opagentd")]
struct Cli {
    #[arg(short, long, default_value = "/run/opagentd/opagentd.sock")]
    socket: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit an operation for execution
    Submit {
        /// JSON operation specification
        operation: String,
    },
    /// Approve a pending task
    Approve {
        task_id: String,
    },
    /// Deny a pending task
    Deny {
        task_id: String,
    },
    /// Show daemon status
    Status,
    /// List pending tasks awaiting approval
    Pending,
    /// View audit logs
    Logs {
        #[arg(short, long)]
        count: Option<usize>,
    },
    /// Validate an operation without executing
    Validate {
        /// JSON operation specification
        operation: String,
    },
}

async fn send_request(socket: &PathBuf, request: &Request) -> Result<Response> {
    let mut stream = UnixStream::connect(socket)
        .await
        .context("Failed to connect to opagentd. Is the daemon running?")?;

    let mut json = serde_json::to_string(request)?;
    json.push('\n');
    stream.write_all(json.as_bytes()).await?;

    let (reader, _writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    buf_reader.read_line(&mut line).await?;

    let response: Response =
        serde_json::from_str(line.trim()).context("Malformed response from daemon")?;
    Ok(response)
}

fn print_response(response: &Response) {
    if response.status == "error" {
        let msg = response.message.as_deref().unwrap_or("unknown error");
        eprintln!("Error: {}", msg);
        std::process::exit(1);
    }
    if let Some(data) = &response.data {
        println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cmd = match cli.command {
        Commands::Submit { operation } => {
            let op: Operation = serde_json::from_str(&operation)
                .context("Failed to parse operation JSON. Use format: {\"Shell\":{\"command\":\"whoami\",\"args\":[]}}")?;
            Command::Submit { operation: op }
        }
        Commands::Approve { task_id } => Command::Approve { task_id },
        Commands::Deny { task_id } => Command::Deny { task_id },
        Commands::Status => Command::Status,
        Commands::Pending => Command::Pending,
        Commands::Logs { count } => Command::Logs { count },
        Commands::Validate { operation } => {
            let op: Operation = serde_json::from_str(&operation)
                .context("Failed to parse operation JSON")?;
            Command::Validate { operation: op }
        }
    };

    let request = Request {
        id: String::new(),
        command: cmd,
    };

    let response = send_request(&cli.socket, &request).await?;
    print_response(&response);

    Ok(())
}
