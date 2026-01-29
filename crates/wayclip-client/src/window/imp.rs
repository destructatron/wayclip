//! WayclipWindow implementation.

use std::cell::RefCell;

use gtk4::gio::ListStore;
use gtk4::glib;
use gtk4::subclass::prelude::*;
use gtk4::{CustomFilter, FilterListModel, Label, ListView, SearchEntry, SingleSelection};

use crate::clipboard_item::ClipboardItem;

pub struct WayclipWindow {
    pub search_entry: SearchEntry,
    pub list_view: ListView,
    pub status_label: Label,
    pub model: ListStore,
    pub filter: RefCell<Option<CustomFilter>>,
    pub filter_model: RefCell<Option<FilterListModel>>,
    pub selection_model: RefCell<Option<SingleSelection>>,
}

impl Default for WayclipWindow {
    fn default() -> Self {
        Self {
            search_entry: SearchEntry::new(),
            list_view: ListView::new(
                None::<SingleSelection>,
                None::<gtk4::SignalListItemFactory>,
            ),
            status_label: Label::new(None),
            model: ListStore::new::<ClipboardItem>(),
            filter: RefCell::new(None),
            filter_model: RefCell::new(None),
            selection_model: RefCell::new(None),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for WayclipWindow {
    const NAME: &'static str = "WayclipWindow";
    type Type = super::WayclipWindow;
    type ParentType = gtk4::ApplicationWindow;
}

impl ObjectImpl for WayclipWindow {}
impl WidgetImpl for WayclipWindow {}
impl WindowImpl for WayclipWindow {}
impl ApplicationWindowImpl for WayclipWindow {}
