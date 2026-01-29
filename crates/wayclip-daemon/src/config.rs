//! Configuration loading and defaults.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub clipboard: ClipboardConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig::default(),
            clipboard: ClipboardConfig::default(),
        }
    }
}

/// Daemon-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Maximum number of entries to keep.
    #[serde(default = "default_max_entries")]
    pub max_entries: u32,
    /// Maximum size of a single entry in bytes.
    #[serde(default = "default_max_entry_size")]
    pub max_entry_size: u64,
    /// Minimum size of an entry in bytes.
    #[serde(default = "default_min_entry_size")]
    pub min_entry_size: u64,
    /// Maximum age of entries in days (0 = no limit).
    #[serde(default)]
    pub max_age_days: u32,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            max_entry_size: default_max_entry_size(),
            min_entry_size: default_min_entry_size(),
            max_age_days: 0,
        }
    }
}

/// Clipboard-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// MIME type patterns to ignore (regex).
    #[serde(default)]
    pub ignore_mime_patterns: Vec<String>,
    /// Application patterns to ignore (regex).
    #[serde(default)]
    pub ignore_app_patterns: Vec<String>,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            ignore_mime_patterns: vec![
                // Common password manager hints
                "x-kde-passwordManagerHint".to_string(),
            ],
            ignore_app_patterns: vec![],
        }
    }
}

fn default_max_entries() -> u32 {
    1000
}

fn default_max_entry_size() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

fn default_min_entry_size() -> u64 {
    1
}

impl Config {
    /// Load configuration from file, or return defaults if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = wayclip_common::config_path();

        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        Ok(config)
    }
}

// Add toml dependency for parsing
// Note: We're using a simple inline parser for now to avoid adding toml crate
mod toml {
    use super::Config;

    pub fn from_str(_s: &str) -> Result<Config, String> {
        // For now, just return defaults
        // TODO: Implement proper TOML parsing
        Ok(Config::default())
    }
}
