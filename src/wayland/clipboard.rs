//! Wayland clipboard handling
use crate::wayland::data_source::{DataSourceEvent, DataSourceManager};
use crate::wayland::event_queue::{EventDrain, EventQueue, EventSource};
use crate::wayland::pipe::{ReadPipe, WritePipe};
use crate::wayland::seat::SeatManager;

/// Clipboard abstraction
pub struct Clipboard {
    seat_manager: SeatManager,
    data_source_manager: DataSourceManager,
    mime_types: Vec<String>,
    data_sources: Vec<(u32, EventDrain<DataSourceEvent>)>,
    event_source: EventSource<ClipboardEvent>,
    event_drain: EventDrain<ClipboardEvent>,
}

impl Clipboard {
    /// Creates a new `Clipboard`
    pub fn new(
        seat_manager: SeatManager,
        data_source_manager: DataSourceManager,
        mime_types: Vec<String>,
    ) -> Self {
        let (event_source, event_drain) = EventQueue::new();
        Clipboard {
            seat_manager,
            data_source_manager,
            mime_types,
            data_sources: Vec::new(),
            event_source,
            event_drain,
        }
    }

    /// Set clipboard content
    ///
    /// Notifies the compositor that the clipboard has been updated.
    /// When a wayland client requests the clipboard contents a
    /// ClipboardEvent::Set will be emitted.
    pub fn set(&mut self, seat_id: u32, serial: u32) {
        let data_device = self.seat_manager.get_data_device(seat_id).unwrap();
        let (data_source, drain) = self
            .data_source_manager
            .create_data_source(&self.mime_types)
            .split();
        data_device.set_selection(Some(&data_source), serial);
        self.data_sources.push((seat_id, drain));
    }

    /// Get the clipboard contents
    ///
    /// If the clipboard isn't empty it will emit a ClipboardEvent::Get when
    /// the wayland client setting the clipboard is ready to send the contents.
    pub fn get(&self, seat_id: u32) {
        if self
            .data_sources
            .iter()
            .find(|(id, _)| *id == seat_id)
            .is_some()
        {
            let mime_type = self.mime_types[0].clone();
            let event = ClipboardEvent::GetLocal { seat_id, mime_type };
            self.event_source.push_event(event);
            return;
        }
        let data_device = self.seat_manager.get_data_device(seat_id).unwrap();
        let clipboard_types = &self.mime_types;
        if let Some(offer) = data_device.get_selection() {
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
    pub fn poll_events<F: FnMut(ClipboardEvent)>(&mut self, mut cb: F) {
        self.data_sources.retain(|(seat_id, drain)| {
            let mut retain = true;
            drain.poll_events(|event| match event {
                DataSourceEvent::Send { pipe, mime_type } => {
                    let event = ClipboardEvent::Set {
                        seat_id: *seat_id,
                        pipe,
                        mime_type,
                    };
                    cb(event);
                }
                DataSourceEvent::Cancelled {} => {
                    retain = false;
                }
                _ => {}
            });
            retain
        });
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
    /// You requested your own clipboard contents
    GetLocal {
        /// The seat id of the clipboard
        seat_id: u32,
        /// The negotiated mime type
        mime_type: String,
    },
}
