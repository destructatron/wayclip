//! Core types shared between daemon and client.

use serde::{Deserialize, Serialize};

/// The type of clipboard content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Image,
}

impl ContentType {
    /// Determine content type from MIME type string.
    pub fn from_mime(mime: &str) -> Self {
        if mime.starts_with("image/") {
            ContentType::Image
        } else {
            ContentType::Text
        }
    }

    /// Check if this is an image type.
    pub fn is_image(&self) -> bool {
        matches!(self, ContentType::Image)
    }

    /// Check if this is a text type.
    pub fn is_text(&self) -> bool {
        matches!(self, ContentType::Text)
    }
}

/// A clipboard history entry (metadata only, no content data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique identifier for this entry.
    pub id: i64,
    /// Type of content (text or image).
    pub content_type: ContentType,
    /// MIME type of the content.
    pub mime_type: String,
    /// Preview text: first 200 chars for text, filename or "copied image" for images.
    pub preview: String,
    /// Size of the content in bytes.
    pub byte_size: u64,
    /// Unix timestamp when this was copied.
    pub created_at: i64,
    /// Whether this entry is pinned (won't be auto-deleted).
    pub pinned: bool,
    /// Optional thumbnail for images (small PNG, base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

impl HistoryEntry {
    /// Generate an accessible description for screen readers.
    pub fn accessible_description(&self) -> String {
        match self.content_type {
            ContentType::Text => format!("Text: {}", self.preview),
            ContentType::Image => format!("Image: {}", self.preview),
        }
    }
}

/// MIME type priority for text content.
pub const TEXT_MIME_PRIORITY: &[&str] = &[
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];

/// MIME type priority for image content.
pub const IMAGE_MIME_PRIORITY: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/gif",
    "image/bmp",
    "image/tiff",
];

/// Select the best MIME type from a list of offered types.
pub fn select_best_mime_type(offered: &[String]) -> Option<&str> {
    // First try image types
    for priority in IMAGE_MIME_PRIORITY {
        if offered.iter().any(|m| m == *priority) {
            return Some(priority);
        }
    }

    // Then try text types
    for priority in TEXT_MIME_PRIORITY {
        if offered.iter().any(|m| m == *priority) {
            return Some(priority);
        }
    }

    // Fall back to first offered type
    offered.first().map(|s| s.as_str())
}
