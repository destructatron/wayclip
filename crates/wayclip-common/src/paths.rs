//! XDG path utilities for wayclip.

use std::path::PathBuf;

/// Get the socket path for IPC communication.
///
/// Returns `$XDG_RUNTIME_DIR/wayclip/wayclip.sock` or falls back to
/// `/tmp/wayclip-$UID/wayclip.sock`.
pub fn socket_path() -> PathBuf {
    if let Some(runtime_dir) = dirs::runtime_dir() {
        runtime_dir.join("wayclip").join("wayclip.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/wayclip-{}/wayclip.sock", uid))
    }
}

/// Get the directory containing the socket.
pub fn socket_dir() -> PathBuf {
    socket_path().parent().unwrap().to_path_buf()
}

/// Get the database path.
///
/// Returns `$XDG_DATA_HOME/wayclip/history.db` or falls back to
/// `~/.local/share/wayclip/history.db`.
pub fn database_path() -> PathBuf {
    let data_dir = dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local/share")
    });
    data_dir.join("wayclip").join("history.db")
}

/// Get the directory containing the database.
pub fn database_dir() -> PathBuf {
    database_path().parent().unwrap().to_path_buf()
}

/// Get the configuration file path.
///
/// Returns `$XDG_CONFIG_HOME/wayclip/config.toml` or falls back to
/// `~/.config/wayclip/config.toml`.
pub fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".config")
    });
    config_dir.join("wayclip").join("config.toml")
}

/// Get the directory containing the config file.
pub fn config_dir() -> PathBuf {
    config_path().parent().unwrap().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_are_valid() {
        let socket = socket_path();
        assert!(socket.to_str().unwrap().contains("wayclip"));

        let db = database_path();
        assert!(db.to_str().unwrap().contains("wayclip"));
        assert!(db.to_str().unwrap().ends_with("history.db"));

        let config = config_path();
        assert!(config.to_str().unwrap().contains("wayclip"));
        assert!(config.to_str().unwrap().ends_with("config.toml"));
    }
}
