//! Unix socket IPC server.

use anyhow::Result;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};
use wayclip_common::{decode_request, encode_response, Request, Response};

/// Event from IPC client.
pub struct IpcEvent {
    pub request: Request,
    pub response_tx: oneshot::Sender<Response>,
}

/// Start the IPC server.
pub async fn serve(socket_path: PathBuf, event_tx: mpsc::Sender<IpcEvent>) -> Result<()> {
    // Remove existing socket if present
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    // Create parent directory if needed
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    info!("IPC server listening on {:?}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, tx).await {
                        debug!("Client connection ended: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(stream: UnixStream, event_tx: mpsc::Sender<IpcEvent>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // Client disconnected
            break;
        }

        let request = match decode_request(line.trim().as_bytes()) {
            Ok(req) => req,
            Err(e) => {
                let response = Response::error(
                    wayclip_common::ErrorCode::InvalidRequest,
                    format!("Invalid request: {}", e),
                );
                let encoded = encode_response(&response)?;
                writer.write_all(&encoded).await?;
                continue;
            }
        };

        debug!("Received request: {:?}", request);

        // Send request to main loop and wait for response
        let (response_tx, response_rx) = oneshot::channel();
        let event = IpcEvent {
            request,
            response_tx,
        };

        if event_tx.send(event).await.is_err() {
            // Main loop shut down
            break;
        }

        let response = match response_rx.await {
            Ok(resp) => resp,
            Err(_) => Response::error(
                wayclip_common::ErrorCode::InternalError,
                "Internal error: response channel closed",
            ),
        };

        let encoded = encode_response(&response)?;
        writer.write_all(&encoded).await?;
        writer.flush().await?;
    }

    Ok(())
}
