//! IPC protocol types for communication between daemon and client.

use serde::{Deserialize, Serialize};

use crate::types::HistoryEntry;

/// Request from client to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Get clipboard history entries.
    GetHistory {
        /// Maximum number of entries to return.
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        /// Number of entries to skip.
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<u32>,
        /// Search filter (case-insensitive substring match).
        #[serde(skip_serializing_if = "Option::is_none")]
        search: Option<String>,
    },

    /// Get the raw content of an entry.
    GetContent {
        /// Entry ID.
        id: i64,
    },

    /// Copy an entry back to the clipboard.
    SetClipboard {
        /// Entry ID to copy.
        id: i64,
    },

    /// Delete an entry from history.
    DeleteEntry {
        /// Entry ID to delete.
        id: i64,
    },

    /// Clear all history (except pinned entries).
    ClearHistory,

    /// Pin or unpin an entry.
    SetPinned {
        /// Entry ID.
        id: i64,
        /// Whether to pin (true) or unpin (false).
        pinned: bool,
    },

    /// Get daemon status.
    GetStatus,

    /// Ping to check if daemon is alive.
    Ping,
}

/// Response from daemon to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// List of history entries.
    History {
        /// The entries (metadata only).
        entries: Vec<HistoryEntry>,
        /// Total count of entries (for pagination).
        total_count: u64,
    },

    /// Raw content data.
    Content {
        /// Entry ID.
        id: i64,
        /// MIME type of the content.
        mime_type: String,
        /// Content data (base64 encoded).
        data: String,
    },

    /// Generic success response.
    Ok,

    /// Error response.
    Error {
        /// Error code.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
    },

    /// Daemon status.
    Status {
        /// Daemon version.
        version: String,
        /// Number of entries in history.
        entry_count: u64,
        /// Database size in bytes.
        database_size_bytes: u64,
    },

    /// Pong response to ping.
    Pong,
}

/// Error codes for error responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Entry not found.
    NotFound,
    /// Database error.
    DatabaseError,
    /// Clipboard operation failed.
    ClipboardError,
    /// Invalid request.
    InvalidRequest,
    /// Internal error.
    InternalError,
}

impl Response {
    /// Create an error response.
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Response::Error {
            code,
            message: message.into(),
        }
    }

    /// Create a not found error response.
    pub fn not_found(id: i64) -> Self {
        Self::error(ErrorCode::NotFound, format!("Entry {} not found", id))
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error { .. })
    }
}

/// Encode a request to JSON bytes with newline delimiter.
pub fn encode_request(request: &Request) -> Result<Vec<u8>, serde_json::Error> {
    let mut json = serde_json::to_vec(request)?;
    json.push(b'\n');
    Ok(json)
}

/// Encode a response to JSON bytes with newline delimiter.
pub fn encode_response(response: &Response) -> Result<Vec<u8>, serde_json::Error> {
    let mut json = serde_json::to_vec(response)?;
    json.push(b'\n');
    Ok(json)
}

/// Decode a request from JSON bytes.
pub fn decode_request(data: &[u8]) -> Result<Request, serde_json::Error> {
    serde_json::from_slice(data)
}

/// Decode a response from JSON bytes.
pub fn decode_response(data: &[u8]) -> Result<Response, serde_json::Error> {
    serde_json::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = Request::GetHistory {
            limit: Some(10),
            offset: None,
            search: Some("test".to_string()),
        };

        let encoded = encode_request(&request).unwrap();
        let decoded: Request = decode_request(&encoded).unwrap();

        match decoded {
            Request::GetHistory {
                limit,
                offset,
                search,
            } => {
                assert_eq!(limit, Some(10));
                assert_eq!(offset, None);
                assert_eq!(search, Some("test".to_string()));
            }
            _ => panic!("Wrong request type"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let response = Response::error(ErrorCode::NotFound, "Entry 42 not found");

        let encoded = encode_response(&response).unwrap();
        let decoded: Response = decode_response(&encoded).unwrap();

        match decoded {
            Response::Error { code, message } => {
                assert_eq!(code, ErrorCode::NotFound);
                assert_eq!(message, "Entry 42 not found");
            }
            _ => panic!("Wrong response type"),
        }
    }
}
