//! Wayland clipboard handling
use crate::wayland::data_offer::ReadPipe;
use crate::wayland::data_source::{DataSource, DataSourceEvent, WritePipe};
use crate::wayland::data_device::{get_selection, set_selection, WlDataDeviceManager};
use crate::wayland::event_queue::{EventDrain, EventQueue, EventSource};
use crate::wayland::seat::SeatManager;
use wayland_client::Proxy;

/// Clipboard abstraction
pub struct Clipboard {
    data_device_manager: Proxy<WlDataDeviceManager>,
    seat_manager: SeatManager,
    mime_types: Vec<String>,
    data_source: Option<DataSource>,
    event_source: EventSource<ClipboardEvent>,
    event_drain: EventDrain<ClipboardEvent>,
}

impl Clipboard {
    /// Creates a new `Clipboard`
    pub fn new(
        data_device_manager: Proxy<WlDataDeviceManager>,
        seat_manager: SeatManager,
        mime_types: Vec<String>,
    ) -> Self {
        let (source, drain) = EventQueue::new();
        Clipboard {
            seat_manager,
            data_device_manager,
            mime_types,
            data_source: None,
            event_source: source,
            event_drain: drain,
        }
    }

    /// Set clipboard content
    ///
    /// Notifies the compositor that the clipboard has been updated.
    /// When a wayland client requests the clipboard contents a
    /// ClipboardEvent::Set will be emitted.
    pub fn set(&mut self, seat_id: u32, serial: u32) {
        let data_device = self
            .seat_manager
            .get_data_device(seat_id)
            .unwrap();
        let event_source = self.event_source.clone();
        self.data_source = Some(DataSource::new(
            &self.data_device_manager,
            &self.mime_types[..],
            move |event| match event {
                DataSourceEvent::Send { pipe, mime_type } => {
                    let event = ClipboardEvent::Set {
                        seat_id,
                        pipe,
                        mime_type,
                    };
                    event_source.push_event(event);
                }
                DataSourceEvent::Cancelled {..} => {
                    println!("cancelled");
                }
                DataSourceEvent::Target {..} => {
                    println!("target");
                }
                DataSourceEvent::Action {..} => {
                    println!("action");
                }
                DataSourceEvent::Dropped => {
                    println!("dropped");
                }
                DataSourceEvent::Finished => {
                    println!("finished");
                }
            }
        ));
        set_selection(
            &data_device,
            &self.data_source,
            serial,
        );
    }

    /// Get the clipboard contents
    ///
    /// If the clipboard isn't empty it will emit a ClipboardEvent::Get when
    /// the wayland client setting the clipboard is ready to send the contents.
    pub fn get(&self, seat_id: u32) {
        let data_device = self
            .seat_manager
            .get_data_device(seat_id)
            .unwrap();
        let clipboard_types = &self.mime_types;
        if let Some(offer) = get_selection(&data_device) {
            if let Some(mime_type) = offer.with_mime_types(|offer_types| {
                for clipboard_type in clipboard_types {
                    for offer_type in offer_types {
                        if clipboard_type == offer_type {
                            return Some(clipboard_type);
                        }
                    }
                }
                None
            }) {
                if let Some(pipe) = offer.receive(mime_type.clone()).ok() {
                    let mime_type = mime_type.clone();
                    let event = ClipboardEvent::Get {
                        seat_id,
                        pipe,
                        mime_type,
                    };
                    self.event_source.push_event(event);
                }
            }
        }
    }

    /// Polls the clipboard event queue
    pub fn poll_events<F: FnMut(ClipboardEvent)>(&self, mut cb: F) {
        self.event_drain.poll_events(|event| {
            cb(event);
        });
    }
}

/// Events emitted by `Clipboard`
pub enum ClipboardEvent {
    /// The clipboard contents are ready
    Get {
        /// The seat id of the clipboard
        seat_id: u32,
        /// The read pipe
        pipe: ReadPipe,
        /// The negotiated mime type
        mime_type: String,
    },
    /// A client has requested the clipboard contents
    Set {
        /// The seat id of the clipboard
        seat_id: u32,
        /// The write pipe
        pipe: WritePipe,
        /// The negotiated mime type
        mime_type: String,
    },
}
