//! Unix socket IPC client using synchronous I/O.

use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use tracing::debug;
use wayclip_common::{decode_response, encode_request, HistoryEntry, Request, Response};

/// IPC client for communicating with the daemon.
pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    /// Connect to the daemon.
    pub fn connect() -> Result<Self> {
        let path = wayclip_common::socket_path();
        debug!("Connecting to daemon at {:?}", path);

        let stream = UnixStream::connect(&path).map_err(|e| {
            anyhow!(
                "Failed to connect to daemon at {:?}: {}. Is wayclip-daemon running?",
                path,
                e
            )
        })?;

        Ok(Self { stream })
    }

    /// Send a request and receive a response.
    fn request(&mut self, request: &Request) -> Result<Response> {
        let encoded = encode_request(request)?;
        self.stream.write_all(&encoded)?;
        self.stream.flush()?;

        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response = decode_response(line.trim().as_bytes())?;
        Ok(response)
    }

    /// Get clipboard history.
    pub fn get_history(
        &mut self,
        limit: Option<u32>,
        offset: Option<u32>,
        search: Option<String>,
    ) -> Result<Vec<HistoryEntry>> {
        let request = Request::GetHistory {
            limit,
            offset,
            search,
        };

        match self.request(&request)? {
            Response::History { entries, .. } => Ok(entries),
            Response::Error { code, message } => {
                Err(anyhow!("Daemon error ({:?}): {}", code, message))
            }
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Copy an item to the clipboard.
    pub fn set_clipboard(&mut self, id: i64) -> Result<()> {
        let request = Request::SetClipboard { id };

        match self.request(&request)? {
            Response::Ok => Ok(()),
            Response::Error { code, message } => {
                Err(anyhow!("Failed to copy item: {} ({:?})", message, code))
            }
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Delete an entry.
    #[allow(dead_code)]
    pub fn delete_entry(&mut self, id: i64) -> Result<()> {
        let request = Request::DeleteEntry { id };

        match self.request(&request)? {
            Response::Ok => Ok(()),
            Response::Error { code, message } => {
                Err(anyhow!("Failed to delete item: {} ({:?})", message, code))
            }
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Clear all history.
    #[allow(dead_code)]
    pub fn clear_history(&mut self) -> Result<()> {
        let request = Request::ClearHistory;

        match self.request(&request)? {
            Response::Ok => Ok(()),
            Response::Error { code, message } => {
                Err(anyhow!("Failed to clear history: {} ({:?})", message, code))
            }
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Ping the daemon.
    #[allow(dead_code)]
    pub fn ping(&mut self) -> Result<()> {
        let request = Request::Ping;

        match self.request(&request)? {
            Response::Pong => Ok(()),
            Response::Error { code, message } => {
                Err(anyhow!("Ping failed: {} ({:?})", message, code))
            }
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }
}
