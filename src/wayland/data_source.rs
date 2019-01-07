//! Data source handling
use crate::wayland::data_device_manager::{
    DataDeviceManagerRequests, DndAction, WlDataDeviceManager,
};
use crate::wayland::event_queue::{EventDrain, EventQueue, EventSource};
use crate::wayland::pipe::{FromRawFd, WritePipe};
use wayland_client::protocol::wl_data_source::Event;
pub use wayland_client::protocol::wl_data_source::RequestsTrait as DataSourceRequests;
pub use wayland_client::protocol::wl_data_source::WlDataSource;
use wayland_client::{NewProxy, Proxy};

#[derive(Clone)]
/// A `DataSourceManager` for creating `DataSource`s
pub struct DataSourceManager {
    data_device_manager: Proxy<WlDataDeviceManager>,
}

impl DataSourceManager {
    /// Creates a new `DataSourceManager`
    pub fn new(data_device_manager: Proxy<WlDataDeviceManager>) -> Self {
        DataSourceManager {
            data_device_manager,
        }
    }

    /// Create a new data source
    ///
    /// You'll then need to provide it to a data device to send it
    /// either via selection (aka copy/paste) or via a drag and drop.
    pub fn create_data_source(&self, mime_types: &[String]) -> DataSource {
        let (source, drain) = EventQueue::new();
        let data_source = self
            .data_device_manager
            .create_data_source(|data_source| implement_data_source(data_source, source))
            .unwrap();
        for mime in mime_types {
            data_source.offer(mime.to_owned());
        }
        DataSource::new(data_source, drain)
    }
}

/// Handles `wl_data_source` events and forwards the ones
/// that need user handling to an event queue.
pub fn implement_data_source(
    data_source: NewProxy<WlDataSource>,
    event_queue: EventSource<DataSourceEvent>,
) -> Proxy<WlDataSource> {
    data_source.implement(
        move |event, data_source| {
            let event = match event {
                Event::Target { mime_type } => DataSourceEvent::Target { mime_type },
                Event::Send { mime_type, fd } => DataSourceEvent::Send {
                    mime_type,
                    pipe: unsafe { FromRawFd::from_raw_fd(fd) },
                },
                Event::Action { dnd_action } => DataSourceEvent::Action {
                    action: DndAction::from_bits_truncate(dnd_action),
                },
                Event::Cancelled => {
                    data_source.destroy();
                    DataSourceEvent::Cancelled
                }
                Event::DndDropPerformed => DataSourceEvent::Dropped,
                Event::DndFinished => {
                    data_source.destroy();
                    DataSourceEvent::Finished
                }
            };
            event_queue.push_event(event);
        },
        (),
    )
}

/// Possible events a data source needs to react to
pub enum DataSourceEvent {
    /// Write the offered data for selected mime type
    ///
    /// This can happen several times during a dnd setup,
    /// and does not mean the action is finished.
    Send {
        /// Requested mime type
        mime_type: String,
        /// Pipe to write into
        pipe: WritePipe,
    },
    /// Target mime type
    ///
    /// Notifies that the target accepted a given mime type.
    /// You can use it to provide feedback (changing the icon
    /// of the drag'n'drop for example).
    ///
    /// Can be `None` if the current target does not accept any of the
    /// proposed mime types.
    ///
    /// This event can be emitted several times during the process
    Target {
        /// The type accepted by the target
        mime_type: Option<String>,
    },
    /// Notifies of the current selected action for the drag'n'drop
    ///
    /// Can only happen for data sources used during a drag'n'drop.
    ///
    /// This can change several times, the last received defines which action
    /// should actually be taken.
    Action {
        /// The action chosen by the target
        action: DndAction,
    },
    /// The action using this data source was cancelled.
    ///
    /// Once this event is received, the `DataSource` can not be used any more,
    /// and you should drop it for cleanup.
    ///
    /// Happens if the user cancels the current drag'n'drop, or replaces the
    /// selection buffer.
    Cancelled,
    /// The user performed the "drop" during a drag'n'drop
    ///
    /// This does not mean the operation is finished (the operation can still
    /// be cancelled afterwards).
    ///
    /// You are not guaranteed to receive this event at some point, as the compositor
    /// may cancel the action before the user drops.
    ///
    /// This event can only be generated on sources used for drag'n'drop, not
    /// selection sources.
    Dropped,
    /// The action is finished, this data source will not be used any more
    ///
    /// If the selected drag'n'drop action was "move", you can now delete the
    /// underlying resource.
    ///
    /// This event can only be generated on sources used for drag'n'drop, not
    /// selection sources.
    Finished,
}

/// Wraps a `wl_data_source` and a `EventDrain<DataSourceEvent>`
pub struct DataSource {
    data_source: Proxy<WlDataSource>,
    event_drain: EventDrain<DataSourceEvent>,
}

impl DataSource {
    /// Creates a new `DataSource`
    pub fn new(data_source: Proxy<WlDataSource>, event_drain: EventDrain<DataSourceEvent>) -> Self {
        DataSource {
            data_source,
            event_drain,
        }
    }

    /// Splits a `DataSource` into a `wl_data_source` and an `EventDrain`
    pub fn split(self) -> (Proxy<WlDataSource>, EventDrain<DataSourceEvent>) {
        (self.data_source, self.event_drain)
    }
}
