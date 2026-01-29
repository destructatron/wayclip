//! ClipboardItem GObject - data model for list items.

mod imp;

use glib::Object;
use gtk4::glib;
use wayclip_common::HistoryEntry;

glib::wrapper! {
    /// A clipboard history item.
    pub struct ClipboardItem(ObjectSubclass<imp::ClipboardItem>);
}

impl ClipboardItem {
    /// Create a new ClipboardItem from a HistoryEntry.
    pub fn from_entry(entry: HistoryEntry) -> Self {
        Object::builder()
            .property("id", entry.id)
            .property("is-image", entry.content_type.is_image())
            .property("mime-type", &entry.mime_type)
            .property("preview", &entry.preview)
            .property("byte-size", entry.byte_size)
            .property("created-at", entry.created_at)
            .property("pinned", entry.pinned)
            .build()
    }

    /// Generate an accessible description.
    pub fn accessible_description(&self) -> String {
        if self.is_image() {
            format!("Image: {}", self.preview())
        } else {
            format!("Text: {}", self.preview())
        }
    }
}
