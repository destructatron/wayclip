//! Wayclip daemon - clipboard history manager for Wayland.

mod clipboard;
mod config;
mod database;
mod ipc;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Daemon version from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("wayclip=info".parse()?))
        .init();

    info!("Starting wayclip daemon v{}", VERSION);

    // Ensure directories exist
    let socket_dir = wayclip_common::socket_dir();
    std::fs::create_dir_all(&socket_dir)?;

    let db_dir = wayclip_common::database_dir();
    std::fs::create_dir_all(&db_dir)?;

    // Load configuration
    let config = config::Config::load()?;
    info!("Loaded configuration: {:?}", config);

    // Initialize database
    let db = database::Database::open()?;
    db.migrate()?;
    info!("Database initialized");

    // Create event channels
    let (clipboard_tx, mut clipboard_rx) = tokio::sync::mpsc::channel::<clipboard::ClipboardEvent>(100);
    let (ipc_tx, mut ipc_rx) = tokio::sync::mpsc::channel::<ipc::IpcEvent>(100);

    // Start clipboard monitor in dedicated thread
    let clipboard_handle = {
        let tx = clipboard_tx;
        std::thread::spawn(move || {
            if let Err(e) = clipboard::monitor(tx) {
                tracing::error!("Clipboard monitor error: {}", e);
            }
        })
    };

    // Start IPC server
    let socket_path = wayclip_common::socket_path();
    let ipc_handle = tokio::spawn(ipc::serve(socket_path, ipc_tx));

    info!("Daemon started, waiting for events...");

    // Main event loop
    loop {
        tokio::select! {
            Some(event) = clipboard_rx.recv() => {
                if let Err(e) = handle_clipboard_event(&db, &config, event).await {
                    tracing::error!("Failed to handle clipboard event: {}", e);
                }
            }
            Some(event) = ipc_rx.recv() => {
                handle_ipc_event(&db, event).await;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    // Cleanup
    drop(ipc_handle);
    drop(clipboard_handle);

    info!("Daemon stopped");
    Ok(())
}

async fn handle_clipboard_event(
    db: &database::Database,
    config: &config::Config,
    event: clipboard::ClipboardEvent,
) -> Result<()> {
    use sha2::{Digest, Sha256};

    let clipboard::ClipboardEvent {
        content,
        mime_type,
        ..
    } = event;

    // Check size limits
    if content.len() as u64 > config.daemon.max_entry_size {
        tracing::debug!("Ignoring entry: too large ({} bytes)", content.len());
        return Ok(());
    }

    if (content.len() as u64) < config.daemon.min_entry_size {
        tracing::debug!("Ignoring entry: too small ({} bytes)", content.len());
        return Ok(());
    }

    // Compute hash for deduplication
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = format!("{:x}", hasher.finalize());

    // Check for duplicate
    if db.find_by_hash(&hash)?.is_some() {
        tracing::debug!("Ignoring duplicate entry");
        db.touch_by_hash(&hash)?;
        return Ok(());
    }

    // Generate preview
    let content_type = wayclip_common::ContentType::from_mime(&mime_type);
    let preview = generate_preview(&content, &mime_type, content_type);

    // Store entry
    db.insert_entry(&hash, content_type, &mime_type, &preview, &content)?;
    tracing::info!("Stored new entry: {} ({} bytes)", preview, content.len());

    // Run cleanup
    db.cleanup(config.daemon.max_entries)?;

    Ok(())
}

fn generate_preview(content: &[u8], mime_type: &str, content_type: wayclip_common::ContentType) -> String {
    match content_type {
        wayclip_common::ContentType::Text => {
            let text = String::from_utf8_lossy(content);
            let preview: String = text.chars().take(200).collect();
            // Normalize whitespace for preview
            preview.split_whitespace().collect::<Vec<_>>().join(" ")
        }
        wayclip_common::ContentType::Image => {
            // Try to extract dimensions from PNG
            if mime_type == "image/png" && content.len() >= 24 {
                let width = u32::from_be_bytes([content[16], content[17], content[18], content[19]]);
                let height = u32::from_be_bytes([content[20], content[21], content[22], content[23]]);
                format!("copied image ({}x{})", width, height)
            } else {
                "copied image".to_string()
            }
        }
    }
}

async fn handle_ipc_event(db: &database::Database, event: ipc::IpcEvent) {
    use wayclip_common::{ErrorCode, Request, Response};

    let response = match event.request {
        Request::GetHistory {
            limit,
            offset,
            search,
        } => {
            match db.get_history(limit, offset, search.as_deref()) {
                Ok((entries, total_count)) => Response::History {
                    entries,
                    total_count,
                },
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::GetContent { id } => {
            match db.get_content(id) {
                Ok(Some((mime_type, data))) => {
                    use base64::Engine;
                    Response::Content {
                        id,
                        mime_type,
                        data: base64::engine::general_purpose::STANDARD.encode(&data),
                    }
                }
                Ok(None) => Response::not_found(id),
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::SetClipboard { id } => {
            match db.get_content(id) {
                Ok(Some((mime_type, data))) => {
                    match clipboard::copy_to_clipboard(&data, &mime_type) {
                        Ok(()) => {
                            let _ = db.touch_entry(id);
                            Response::Ok
                        }
                        Err(e) => Response::error(ErrorCode::ClipboardError, e.to_string()),
                    }
                }
                Ok(None) => Response::not_found(id),
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::DeleteEntry { id } => {
            match db.delete_entry(id) {
                Ok(true) => Response::Ok,
                Ok(false) => Response::not_found(id),
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::ClearHistory => {
            match db.clear_unpinned() {
                Ok(()) => Response::Ok,
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::SetPinned { id, pinned } => {
            match db.set_pinned(id, pinned) {
                Ok(true) => Response::Ok,
                Ok(false) => Response::not_found(id),
                Err(e) => Response::error(ErrorCode::DatabaseError, e.to_string()),
            }
        }

        Request::GetStatus => {
            match (db.count_entries(), db.database_size()) {
                (Ok(entry_count), Ok(database_size_bytes)) => Response::Status {
                    version: VERSION.to_string(),
                    entry_count,
                    database_size_bytes,
                },
                _ => Response::error(ErrorCode::DatabaseError, "Failed to get status"),
            }
        }

        Request::Ping => Response::Pong,
    };

    let _ = event.response_tx.send(response);
}
