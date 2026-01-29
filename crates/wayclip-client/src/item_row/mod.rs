//! ItemRow widget - custom widget for displaying clipboard items.

mod imp;

use glib::Object;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use crate::clipboard_item::ClipboardItem;

glib::wrapper! {
    /// A row widget for displaying a clipboard item.
    pub struct ItemRow(ObjectSubclass<imp::ItemRow>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for ItemRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemRow {
    /// Create a new ItemRow.
    pub fn new() -> Self {
        Object::builder().build()
    }

    /// Bind this row to a clipboard item.
    pub fn bind(&self, item: &ClipboardItem) {
        let imp = self.imp();

        // Update icon
        let icon_name = if item.is_image() {
            "image-x-generic-symbolic"
        } else {
            "text-x-generic-symbolic"
        };
        imp.icon.set_icon_name(Some(icon_name));

        // Update content label
        imp.content_label.set_label(&item.preview());

        // Update timestamp label
        let timestamp = format_relative_time(item.created_at());
        imp.timestamp_label.set_label(&timestamp);

        // Update accessibility
        self.update_property(&[gtk4::accessible::Property::Label(
            &item.accessible_description(),
        )]);
    }
}

/// Format a Unix timestamp as relative time (e.g., "2 minutes ago").
fn format_relative_time(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let diff = now - timestamp;

    if diff < 60 {
        "Just now".to_string()
    } else if diff < 3600 {
        let mins = diff / 60;
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if diff < 86400 {
        let hours = diff / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        let days = diff / 86400;
        if days == 1 {
            "Yesterday".to_string()
        } else {
            format!("{} days ago", days)
        }
    }
}
