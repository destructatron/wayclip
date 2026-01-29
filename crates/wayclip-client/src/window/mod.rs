//! Main application window.

mod imp;

use glib::Object;
use gtk4::glib::{self, clone};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::Application;
use tracing::{debug, error, info};

use crate::clipboard_item::ClipboardItem;
use crate::ipc::IpcClient;

glib::wrapper! {
    /// The main wayclip window.
    pub struct WayclipWindow(ObjectSubclass<imp::WayclipWindow>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gtk4::gio::ActionGroup, gtk4::gio::ActionMap, gtk4::Accessible,
                    gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native,
                    gtk4::Root, gtk4::ShortcutManager;
}

impl WayclipWindow {
    /// Create a new window.
    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder()
            .property("application", app)
            .property("title", "Wayclip")
            .property("default-width", 450)
            .property("default-height", 500)
            .build();

        window.setup_widgets();
        window.setup_callbacks();
        window.setup_shortcuts();
        window.load_history();

        window
    }

    fn setup_widgets(&self) {
        let imp = self.imp();

        // Main container
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Search entry
        imp.search_entry
            .set_placeholder_text(Some("Search clipboard history..."));
        imp.search_entry.set_hexpand(true);
        imp.search_entry.set_margin_top(12);
        imp.search_entry.set_margin_bottom(12);
        imp.search_entry.set_margin_start(12);
        imp.search_entry.set_margin_end(12);
        imp.search_entry.set_search_delay(150);

        // Accessibility for search
        imp.search_entry.update_property(&[
            gtk4::accessible::Property::Label("Search clipboard history"),
        ]);

        main_box.append(&imp.search_entry);

        // Create filter
        let filter = gtk4::CustomFilter::new(clone!(
            #[weak(rename_to = search_entry)]
            imp.search_entry,
            #[upgrade_or]
            false,
            move |obj| {
                let item = obj.downcast_ref::<ClipboardItem>().unwrap();
                let search_text = search_entry.text().to_lowercase();
                if search_text.is_empty() {
                    return true;
                }
                item.preview().to_lowercase().contains(&search_text)
            }
        ));

        imp.filter.replace(Some(filter.clone()));

        let filter_model = gtk4::FilterListModel::new(Some(imp.model.clone()), Some(filter));
        let selection_model = gtk4::SingleSelection::new(Some(filter_model.clone()));
        selection_model.set_autoselect(true);
        selection_model.set_can_unselect(false);

        imp.filter_model.replace(Some(filter_model));
        imp.selection_model
            .replace(Some(selection_model.clone()));

        // Factory
        let factory = gtk4::SignalListItemFactory::new();

        factory.connect_setup(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let row = crate::item_row::ItemRow::new();
            list_item.set_child(Some(&row));
        });

        factory.connect_bind(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let item = list_item.item().and_downcast::<ClipboardItem>().unwrap();
            let row = list_item
                .child()
                .and_downcast::<crate::item_row::ItemRow>()
                .unwrap();
            row.bind(&item);
        });

        // ListView
        imp.list_view.set_model(Some(&selection_model));
        imp.list_view.set_factory(Some(&factory));
        imp.list_view.set_single_click_activate(false);
        imp.list_view.add_css_class("navigation-sidebar");

        // Scrolled window
        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .child(&imp.list_view)
            .build();

        main_box.append(&scrolled);

        // Status bar with item count
        imp.status_label.set_xalign(0.0);
        imp.status_label.set_margin_top(8);
        imp.status_label.set_margin_bottom(8);
        imp.status_label.set_margin_start(12);
        imp.status_label.add_css_class("dim-label");
        main_box.append(&imp.status_label);

        self.set_child(Some(&main_box));
    }

    fn setup_callbacks(&self) {
        let imp = self.imp();

        // Search changed
        imp.search_entry.connect_search_changed(clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.on_search_changed();
            }
        ));

        // List item activated
        imp.list_view.connect_activate(clone!(
            #[weak(rename_to = window)]
            self,
            move |_, position| {
                window.on_item_activated(position);
            }
        ));
    }

    fn setup_shortcuts(&self) {
        let controller = gtk4::EventControllerKey::new();

        controller.connect_key_pressed(clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, modifier| window.on_key_pressed(key, modifier)
        ));

        self.add_controller(controller);
    }

    fn on_search_changed(&self) {
        let imp = self.imp();
        if let Some(filter) = imp.filter.borrow().as_ref() {
            filter.changed(gtk4::FilterChange::Different);
        }
        self.update_status();
    }

    fn on_item_activated(&self, position: u32) {
        let imp = self.imp();

        let Some(selection_model) = imp.selection_model.borrow().clone() else {
            return;
        };

        let Some(item) = selection_model
            .item(position)
            .and_downcast::<ClipboardItem>()
        else {
            return;
        };

        info!(
            "Activating item: {} (id={})",
            item.preview(),
            item.id()
        );

        // Copy to clipboard via daemon (synchronous, quick operation)
        let item_id = item.id();
        match self.copy_item_to_clipboard(item_id) {
            Ok(()) => {
                info!("Successfully copied item {} to clipboard", item_id);
                self.close();
            }
            Err(e) => {
                error!("Failed to copy item: {}", e);
            }
        }
    }

    fn on_key_pressed(
        &self,
        key: gtk4::gdk::Key,
        modifier: gtk4::gdk::ModifierType,
    ) -> glib::Propagation {
        use gtk4::gdk::Key;

        let imp = self.imp();

        match key {
            // Escape: Clear search or close
            Key::Escape => {
                if !imp.search_entry.text().is_empty() {
                    imp.search_entry.set_text("");
                    glib::Propagation::Stop
                } else {
                    self.close();
                    glib::Propagation::Stop
                }
            }
            // Ctrl+F: Focus search
            Key::f if modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) => {
                imp.search_entry.grab_focus();
                glib::Propagation::Stop
            }
            // Down arrow from search: Move to list
            Key::Down if imp.search_entry.has_focus() => {
                imp.list_view.grab_focus();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    }

    fn load_history(&self) {
        let imp = self.imp();
        imp.status_label.set_label("Loading...");

        match self.fetch_history() {
            Ok(()) => {
                self.update_status();
                imp.search_entry.grab_focus();
            }
            Err(e) => {
                error!("Failed to load history: {}", e);
                imp.status_label.set_label(&format!("Error: {}", e));
            }
        }
    }

    fn fetch_history(&self) -> anyhow::Result<()> {
        let imp = self.imp();

        let mut client = IpcClient::connect()?;
        let entries = client.get_history(Some(100), None, None)?;

        imp.model.remove_all();
        for entry in entries {
            let item = ClipboardItem::from_entry(entry);
            imp.model.append(&item);
        }

        debug!("Loaded {} entries", imp.model.n_items());
        Ok(())
    }

    fn copy_item_to_clipboard(&self, id: i64) -> anyhow::Result<()> {
        let mut client = IpcClient::connect()?;
        client.set_clipboard(id)
    }

    fn update_status(&self) {
        let imp = self.imp();

        let total = imp.model.n_items();
        let visible = imp
            .filter_model
            .borrow()
            .as_ref()
            .map(|m| m.n_items())
            .unwrap_or(total);

        let label = if imp.search_entry.text().is_empty() {
            format!("{} items", total)
        } else {
            format!("{} of {} items", visible, total)
        };

        imp.status_label.set_label(&label);
    }
}
