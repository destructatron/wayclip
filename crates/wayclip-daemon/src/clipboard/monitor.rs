//! Clipboard monitoring using wlr-data-control protocol.

use super::ClipboardEvent;
use anyhow::{anyhow, Result};
use std::io::Read;
use std::os::fd::AsFd;
use tokio::sync::mpsc;
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{event_created_child, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
};

/// Monitor the clipboard for changes.
pub fn monitor(tx: mpsc::Sender<ClipboardEvent>) -> Result<()> {
    let conn = Connection::connect_to_env()?;
    let display = conn.display();

    let mut event_queue: EventQueue<ClipboardState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = ClipboardState::new(tx);

    display.get_registry(&qh, ());

    // Initial roundtrip to get globals
    event_queue.roundtrip(&mut state)?;

    if state.data_control_manager.is_none() {
        return Err(anyhow!(
            "Compositor does not support wlr-data-control protocol"
        ));
    }

    // Create data device for the seat
    if let (Some(manager), Some(seat)) = (&state.data_control_manager, &state.seat) {
        let _device = manager.get_data_device(seat, &qh, ());
    }

    // Do another roundtrip to ensure device is ready
    event_queue.roundtrip(&mut state)?;

    // Event loop
    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

struct ClipboardState {
    tx: mpsc::Sender<ClipboardEvent>,
    data_control_manager: Option<ZwlrDataControlManagerV1>,
    seat: Option<WlSeat>,
    current_offer: Option<ZwlrDataControlOfferV1>,
    offered_mime_types: Vec<String>,
}

impl ClipboardState {
    fn new(tx: mpsc::Sender<ClipboardEvent>) -> Self {
        Self {
            tx,
            data_control_manager: None,
            seat: None,
            current_offer: None,
            offered_mime_types: Vec::new(),
        }
    }

    fn receive_clipboard(&mut self) {
        let Some(offer) = self.current_offer.take() else {
            return;
        };

        // Select best MIME type
        let mime_type = wayclip_common::select_best_mime_type(&self.offered_mime_types);
        let Some(mime_type) = mime_type else {
            tracing::debug!("No suitable MIME type offered");
            return;
        };

        // Create pipe
        let (read_fd, write_fd) = match nix::unistd::pipe() {
            Ok(fds) => fds,
            Err(e) => {
                tracing::error!("Failed to create pipe: {}", e);
                return;
            }
        };

        // Request the data
        offer.receive(mime_type.to_string(), write_fd.as_fd());

        // Important: destroy the offer after requesting
        offer.destroy();

        // Drop write fd after sending to compositor
        drop(write_fd);

        // Read data in a separate thread to not block the wayland event loop
        let mime_type = mime_type.to_string();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let mut file = std::fs::File::from(read_fd);
            let mut content = Vec::new();

            if let Err(e) = file.read_to_end(&mut content) {
                tracing::error!("Failed to read clipboard data: {}", e);
                return;
            }

            if content.is_empty() {
                tracing::debug!("Clipboard content is empty, ignoring");
                return;
            }

            let event = ClipboardEvent {
                content,
                mime_type,
                source_app: None,
            };

            let _ = tx.blocking_send(event);
        });
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "zwlr_data_control_manager_v1" => {
                    let manager =
                        registry.bind::<ZwlrDataControlManagerV1, _, _>(name, version, qh, ());
                    state.data_control_manager = Some(manager);
                }
                "wl_seat" => {
                    let seat = registry.bind::<WlSeat, _, _>(name, version, qh, ());
                    state.seat = Some(seat);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We don't need to handle seat events
    }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                // New offer, start collecting MIME types
                state.current_offer = Some(id);
                state.offered_mime_types.clear();
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                if id.is_some() {
                    // Selection changed, receive the data
                    state.receive_clipboard();
                }
            }
            zwlr_data_control_device_v1::Event::Finished => {
                // Device is no longer valid
                tracing::warn!("Data control device finished");
            }
            zwlr_data_control_device_v1::Event::PrimarySelection { .. } => {
                // We're not monitoring primary selection
            }
            _ => {}
        }
    }

    // Tell wayland-client how to create child objects for DataOffer events
    event_created_child!(ClipboardState, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.offered_mime_types.push(mime_type);
        }
    }
}
