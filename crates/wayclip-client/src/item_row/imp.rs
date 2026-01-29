//! ItemRow implementation.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label, Orientation};

#[derive(Default)]
pub struct ItemRow {
    pub icon: Image,
    pub content_label: Label,
    pub timestamp_label: Label,
}

#[glib::object_subclass]
impl ObjectSubclass for ItemRow {
    const NAME: &'static str = "WayclipItemRow";
    type Type = super::ItemRow;
    type ParentType = Box;
}

impl ObjectImpl for ItemRow {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.set_orientation(Orientation::Horizontal);
        obj.set_spacing(12);
        obj.set_margin_top(8);
        obj.set_margin_bottom(8);
        obj.set_margin_start(12);
        obj.set_margin_end(12);

        // Icon
        self.icon.set_pixel_size(32);
        self.icon.set_icon_name(Some("text-x-generic-symbolic"));
        obj.append(&self.icon);

        // Content box
        let content_box = Box::new(Orientation::Vertical, 4);
        content_box.set_hexpand(true);

        // Content preview label
        self.content_label.set_xalign(0.0);
        self.content_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        self.content_label.set_max_width_chars(60);
        self.content_label.add_css_class("content-preview");
        content_box.append(&self.content_label);

        // Timestamp label
        self.timestamp_label.set_xalign(0.0);
        self.timestamp_label.add_css_class("dim-label");
        self.timestamp_label.add_css_class("caption");
        content_box.append(&self.timestamp_label);

        obj.append(&content_box);
    }
}

impl WidgetImpl for ItemRow {}
impl BoxImpl for ItemRow {}
