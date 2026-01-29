//! Wayclip GTK 4 client - clipboard history browser.

mod clipboard_item;
mod ipc;
mod item_row;
mod window;

use gtk4::prelude::*;
use gtk4::{gio, glib};
use tracing_subscriber::EnvFilter;

const APP_ID: &str = "com.wayclip.Client";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("wayclip=debug".parse().unwrap()))
        .init();

    // Register custom types
    clipboard_item::ClipboardItem::ensure_type();
    item_row::ItemRow::ensure_type();
    window::WayclipWindow::ensure_type();

    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();

    app.connect_activate(|app| {
        let window = window::WayclipWindow::new(app);
        window.present();
    });

    app.run()
}
