//! Data device handling
use crate::wayland::data_device_manager::DndAction;
use crate::wayland::data_offer::{DataOffer, WlDataOffer};
use crate::wayland::data_source::{DataSourceRequests, WlDataSource};
use crate::wayland::seat::SeatEventSource;
use crate::wayland::surface::WlSurface;
use std::sync::Mutex;
use wayland_client::protocol::wl_data_device::Event;
pub use wayland_client::protocol::wl_data_device::{
    RequestsTrait as DataDeviceRequests, WlDataDevice,
};
use wayland_client::{NewProxy, Proxy};

/// Handles `wl_data_device` events and forwards the ones
/// that need user handling to an event queue.
pub fn implement_data_device(
    data_device: NewProxy<WlDataDevice>,
    mut event_queue: SeatEventSource<DataDeviceEvent>,
) -> Proxy<WlDataDevice> {
    data_device.implement(
        move |event, data_device| match event {
            Event::Enter {
                serial,
                surface,
                x,
                y,
                id,
            } => {
                let mut user_data = data_device
                    .user_data::<Mutex<DataDeviceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                user_data.set_dnd(id);
                event_queue.enter_surface(&surface);
                event_queue.queue_event(DataDeviceEvent::Enter {
                    x,
                    y,
                    offer: user_data.current_dnd.clone(),
                    serial,
                });
            }
            Event::Motion { x, y, time } => {
                event_queue.queue_event(DataDeviceEvent::Motion { x, y, time });
            }
            Event::Leave => {
                event_queue.queue_event(DataDeviceEvent::Leave);
            }
            Event::Drop => {
                event_queue.queue_event(DataDeviceEvent::Drop);
            }
            Event::Selection { id } => {
                let mut user_data = data_device
                    .user_data::<Mutex<DataDeviceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                user_data.set_selection(id);
            }
            Event::DataOffer { id } => {
                let mut user_data = data_device
                    .user_data::<Mutex<DataDeviceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                user_data.offers.push(DataOffer::new(id));
            }
        },
        Mutex::new(DataDeviceUserData::new()),
    )
}

/// `wl_data_device` user data
pub struct DataDeviceUserData {
    /// The current selection
    selection: Option<DataOffer>,
    /// The current drag'n'drop
    current_dnd: Option<DataOffer>,
    /// All data offers anounced by the compositor
    offers: Vec<DataOffer>,
}

impl DataDeviceUserData {
    /// Creates `DataDeviceUserData`
    pub fn new() -> Self {
        DataDeviceUserData {
            selection: None,
            current_dnd: None,
            offers: Vec::new(),
        }
    }

    fn set_selection(&mut self, offer: Option<Proxy<WlDataOffer>>) {
        if let Some(offer) = offer {
            if let Some(id) = self.offers.iter().position(|o| o.offer.equals(&offer)) {
                self.selection = Some(self.offers.swap_remove(id));
            } else {
                panic!("Compositor set an unknown data_offer for selection.");
            }
        } else {
            // drop the current offer if any
            self.selection = None;
        }
    }

    fn set_dnd(&mut self, offer: Option<Proxy<WlDataOffer>>) {
        if let Some(offer) = offer {
            if let Some(id) = self.offers.iter().position(|o| o.offer.equals(&offer)) {
                self.current_dnd = Some(self.offers.swap_remove(id));
            } else {
                panic!("Compositor set an unknown data_offer for selection.");
            }
        } else {
            // drop the current offer if any
            self.current_dnd = None;
        }
    }
}

/// Possible events generated during a drag'n'drop session
#[derive(Clone, Debug)]
pub enum DataDeviceEvent {
    /// A new drag'n'drop entered your surface
    Enter {
        /// The associated data offer
        ///
        /// Is None if it is an internal drag'n'drop you started with
        /// no source. See `DataDevice::start_drag` for details.
        offer: Option<DataOffer>,
        /// horizontal location on the surface
        x: f64,
        /// vertical location on the surface
        y: f64,
        /// serial number of the event
        serial: u32,
    },
    /// The drag'n'drop offer moved on the surface
    Motion {
        /// new horizontal location
        x: f64,
        /// new vertical location
        y: f64,
        /// The time of this motion
        time: u32,
    },
    /// The drag'n'drop offer left your surface
    Leave,
    /// The drag'n'drop was dropped on your surface
    Drop,
}

#[derive(Clone)]
/// Wraps a `DataDevice`
pub struct DataDevice {
    data_device: Proxy<WlDataDevice>,
}

impl DataDevice {
    /// Creates a new `DataDevice` from a `wl_data_device`
    pub fn new(data_device: Proxy<WlDataDevice>) -> Self {
        DataDevice { data_device }
    }

    /// Provide a data source as the new content for the selection
    ///
    /// Correspond to traditional copy/paste behavior. Setting the
    /// source to `None` will clear the selection.
    pub fn set_selection(&self, source: Option<&Proxy<WlDataSource>>, serial: u32) {
        self.data_device.set_selection(source, serial);
    }

    /// Get the current selection
    ///
    /// Correspond to traditional copy/paste behavior.
    pub fn get_selection(&self) -> Option<DataOffer> {
        self.data_device
            .user_data::<Mutex<DataDeviceUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .selection
            .clone()
    }

    /// Start a drag'n'drop offer
    ///
    /// You need to specify the origin surface, as well a serial associated
    /// to an implicit grab on this surface (for example received by a pointer click).
    ///
    /// An optional `DataSource` can be provided. If it is `None`, this drag'n'drop will
    /// be considered as internal to your application, and other applications will not be
    /// notified of it. You are then responsible for acting accordingly on drop.
    ///
    /// You also need to specify which possible drag'n'drop actions are associated to this
    /// drag (copy, move, or ask), the final action will be chosen by the target and/or
    /// compositor.
    ///
    /// You can finally provide a surface that will be used as an icon associated with
    /// this drag'n'drop for user visibility.
    pub fn start_drag(
        &self,
        origin: &Proxy<WlSurface>,
        data_source: Option<&Proxy<WlDataSource>>,
        actions: DndAction,
        icon: Option<&Proxy<WlSurface>>,
        serial: u32,
    ) {
        if data_source.is_some() {
            data_source.map(|data_source| {
                data_source.set_actions(actions.to_raw());
            });
        }
        self.data_device
            .start_drag(data_source, origin, icon, serial);
    }
}
