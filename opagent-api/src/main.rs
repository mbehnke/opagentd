use std::path::PathBuf;

use anyhow::Result;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use opagent_core::message::{Command, Request, Response};

#[derive(Serialize, Deserialize)]
struct ApiRequest {
    command: Command,
}

#[derive(Serialize)]
struct ApiResponse {
    #[serde(flatten)]
    response: Response,
}

#[derive(Clone)]
struct AppState {
    socket_path: PathBuf,
}

async fn proxy_request(
    State(state): State<AppState>,
    Json(req): Json<ApiRequest>,
) -> Result<Json<ApiResponse>, String> {
    let request = Request {
        id: String::new(),
        command: req.command,
    };

    let mut stream = UnixStream::connect(&state.socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    let mut json = serde_json::to_string(&request).map_err(|e| format!("Serialize: {}", e))?;
    json.push('\n');
    stream
        .write_all(json.as_bytes())
        .await
        .map_err(|e| format!("Write: {}", e))?;

    let (reader, _writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    buf_reader
        .read_line(&mut line)
        .await
        .map_err(|e| format!("Read: {}", e))?;

    let response: Response =
        serde_json::from_str(line.trim()).map_err(|e| format!("Parse: {}", e))?;

    Ok(Json(ApiResponse { response }))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    let socket_path = PathBuf::from("/run/opagentd/opagentd.sock");

    let state = AppState { socket_path };

    let app = Router::new()
        .route("/api/v1/submit", post(proxy_request))
        .route("/api/v1/approve", post(proxy_request))
        .route("/api/v1/deny", post(proxy_request))
        .route("/api/v1/status", post(proxy_request))
        .route("/api/v1/validate", post(proxy_request))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9090").await?;
    tracing::info!("opagent-api listening on 127.0.0.1:9090");

    axum::serve(listener, app).await?;

    Ok(())
}
