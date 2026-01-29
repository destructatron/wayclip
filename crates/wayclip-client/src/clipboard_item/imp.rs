//! ClipboardItem implementation.

use std::cell::{Cell, RefCell};

use glib::Properties;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::ClipboardItem)]
pub struct ClipboardItem {
    /// Entry ID.
    #[property(get, set)]
    pub id: Cell<i64>,

    /// Whether this is an image (true) or text (false).
    #[property(name = "is-image", get, set)]
    pub is_image: Cell<bool>,

    /// MIME type string.
    #[property(name = "mime-type", get, set)]
    pub mime_type: RefCell<String>,

    /// Preview text.
    #[property(get, set)]
    pub preview: RefCell<String>,

    /// Size in bytes.
    #[property(name = "byte-size", get, set)]
    pub byte_size: Cell<u64>,

    /// Creation timestamp.
    #[property(name = "created-at", get, set)]
    pub created_at: Cell<i64>,

    /// Whether pinned.
    #[property(get, set)]
    pub pinned: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for ClipboardItem {
    const NAME: &'static str = "WayclipClipboardItem";
    type Type = super::ClipboardItem;
}

#[glib::derived_properties]
impl ObjectImpl for ClipboardItem {}
