//! Seat handling
use crate::wayland::cursor::CursorManager;
use crate::wayland::data_device_manager::{DataDeviceManagerRequests, WlDataDeviceManager};
use crate::wayland::event_queue::{EventDrain, EventSource};
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData, WlSurface};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_registry::RequestsTrait as RegistryRequests;
use wayland_client::protocol::wl_registry::WlRegistry;
pub use wayland_client::protocol::wl_seat::RequestsTrait as SeatRequests;
pub use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_seat::{Capability, Event};
use wayland_client::Proxy;

use crate::wayland::data_device::{
    implement_data_device, DataDevice, DataDeviceEvent, DataDeviceRequests, WlDataDevice,
};
use crate::wayland::keyboard::{implement_keyboard, KeyboardEvent, KeyboardRequests, WlKeyboard};
use crate::wayland::pointer::{implement_pointer, PointerEvent, PointerRequests, WlPointer};
use crate::wayland::touch::{implement_touch, TouchEvent, TouchRequests, WlTouch};

/// Handles `wl_seat`s
#[derive(Clone)]
pub struct SeatManager {
    seats: Arc<Mutex<Vec<Proxy<WlSeat>>>>,
    event_drain: EventDrain<SeatManagerEvent>,
    cursor_manager: CursorManager,
    data_device_manager: Proxy<WlDataDeviceManager>,
}

impl SeatManager {
    /// Creates a new `SeatManager`
    pub fn new(
        event_drain: EventDrain<SeatManagerEvent>,
        cursor_manager: CursorManager,
        data_device_manager: Proxy<WlDataDeviceManager>,
    ) -> Self {
        SeatManager {
            seats: Arc::new(Mutex::new(Vec::new())),
            event_drain,
            cursor_manager,
            data_device_manager,
        }
    }

    fn new_seat(&self, seat_id: u32, version: u32, registry: &Proxy<WlRegistry>) {
        let cursor_manager = self.cursor_manager.clone();
        let data_device_manager = self.data_device_manager.clone();
        let seat = registry
            .bind(version, seat_id, |seat| {
                seat.implement(
                    move |event, seat| {
                        let mut user_data = seat
                            .user_data::<Mutex<SeatUserData>>()
                            .unwrap()
                            .lock()
                            .unwrap();

                        user_data.impl_data_device(&seat, &data_device_manager);

                        match event {
                            Event::Name { name } => {
                                user_data.name = name;
                            }
                            Event::Capabilities { capabilities } => {
                                if capabilities.contains(Capability::Pointer) {
                                    user_data.impl_pointer(&seat, &cursor_manager);
                                } else {
                                    user_data.drop_pointer();
                                }
                                if capabilities.contains(Capability::Keyboard) {
                                    user_data.impl_keyboard(&seat);
                                } else {
                                    user_data.drop_keyboard();
                                }
                                if capabilities.contains(Capability::Touch) {
                                    user_data.impl_touch(&seat);
                                } else {
                                    user_data.drop_touch();
                                }
                            }
                        }
                    },
                    Mutex::new(SeatUserData::new()),
                )
            })
            .unwrap();
        self.seats.lock().unwrap().push(seat);
    }

    fn remove_seat(&self, seat_id: u32) {
        self.seats.lock().unwrap().retain(|seat| {
            if seat.id() == seat_id && seat.version() >= 5 {
                seat.release();
            }
            seat.id() != seat_id
        });
    }

    /// A list of all current seats
    pub fn seats(&self) -> &Arc<Mutex<Vec<Proxy<WlSeat>>>> {
        &self.seats
    }

    /// The `wl_seat` with `seat_id`
    pub fn get_seat(&self, seat_id: u32) -> Option<Proxy<WlSeat>> {
        self.seats
            .lock()
            .unwrap()
            .iter()
            .find(|seat| seat.id() == seat_id)
            .map(|seat| seat.clone())
    }

    /// The `wl_data_device` associated with `seat_id`
    pub fn get_data_device(&self, seat_id: u32) -> Option<DataDevice> {
        let seat = self.get_seat(seat_id);
        if seat.is_none() {
            return None;
        }
        seat.unwrap()
            .user_data::<Mutex<SeatUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .data_device()
            .map(|data_device| DataDevice::new(data_device.clone()))
    }

    /// Processes it's event queues
    pub fn handle_events(&self) {
        self.event_drain.poll_events(|event| match event {
            SeatManagerEvent::NewSeat {
                id,
                version,
                registry,
            } => {
                self.new_seat(id, version, &registry);
            }
            SeatManagerEvent::RemoveSeat { id } => {
                self.remove_seat(id);
            }
        })
    }
}

#[derive(Clone)]
/// Compiled information about a seat
pub struct SeatUserData {
    name: String,
    pointer: Option<Proxy<WlPointer>>,
    keyboard: Option<Proxy<WlKeyboard>>,
    touch: Option<Proxy<WlTouch>>,
    data_device: Option<Proxy<WlDataDevice>>,
}

impl SeatUserData {
    /// Creates a new `SeatUserData`
    pub fn new() -> Self {
        SeatUserData {
            name: String::new(),
            pointer: None,
            keyboard: None,
            touch: None,
            data_device: None,
        }
    }

    /// Returns the name of the seat
    pub fn name(&self) -> &str {
        &self.name[..]
    }

    fn impl_pointer(&mut self, seat: &Proxy<WlSeat>, cursor_manager: &CursorManager) {
        if self.pointer.is_none() {
            self.pointer = seat
                .get_pointer(|pointer| {
                    let event_queue = SeatEventSource::new(seat.id());
                    implement_pointer(pointer, event_queue, cursor_manager.clone())
                })
                .ok();
        }
    }

    /// Returns the seat pointer device if there is one
    pub fn pointer(&self) -> Option<&Proxy<WlPointer>> {
        self.pointer.as_ref()
    }

    fn drop_pointer(&mut self) {
        if self.pointer.is_some() {
            let pointer = self.pointer.take().unwrap();
            if pointer.version() >= 3 {
                pointer.release();
            }
        }
    }

    fn impl_keyboard(&mut self, seat: &Proxy<WlSeat>) {
        if self.keyboard.is_none() {
            self.keyboard = seat
                .get_keyboard(|keyboard| {
                    let event_queue = SeatEventSource::new(seat.id());
                    implement_keyboard(keyboard, event_queue)
                })
                .ok();
        }
    }

    /// Returns the seat keyboard device if there is one
    pub fn keyboard(&self) -> Option<&Proxy<WlKeyboard>> {
        self.keyboard.as_ref()
    }

    fn drop_keyboard(&mut self) {
        if self.keyboard.is_some() {
            let keyboard = self.keyboard.take().unwrap();
            if keyboard.version() >= 3 {
                keyboard.release();
            }
        }
    }

    fn impl_touch(&mut self, seat: &Proxy<WlSeat>) {
        if self.touch.is_none() {
            self.touch = seat
                .get_touch(|touch| {
                    let event_queue = SeatEventSource::new(seat.id());
                    implement_touch(touch, event_queue)
                })
                .ok();
        }
    }

    /// Reteurns the seat touch device if there is one
    pub fn touch(&self) -> Option<&Proxy<WlTouch>> {
        self.touch.as_ref()
    }

    fn drop_touch(&mut self) {
        if self.touch.is_some() {
            let touch = self.touch.take().unwrap();
            if touch.version() >= 3 {
                touch.release();
            }
        }
    }

    fn impl_data_device(
        &mut self,
        seat: &Proxy<WlSeat>,
        data_device_manager: &Proxy<WlDataDeviceManager>,
    ) {
        if self.data_device.is_none() {
            self.data_device = data_device_manager
                .get_data_device(&seat, |data_device| {
                    let event_queue = SeatEventSource::new(seat.id());
                    implement_data_device(data_device, event_queue)
                })
                .ok();
        }
    }

    /// Returns the seat data device if there is one
    pub fn data_device(&self) -> Option<&Proxy<WlDataDevice>> {
        self.data_device.as_ref()
    }

    fn drop_data_device(&mut self) {
        if self.data_device.is_some() {
            let data_device = self.data_device.take().unwrap();
            data_device.release();
        }
    }
}

impl Drop for SeatUserData {
    fn drop(&mut self) {
        self.drop_pointer();
        self.drop_keyboard();
        self.drop_touch();
        self.drop_data_device();
    }
}

/// The events that a `SeatManager` needs to know about
#[derive(Clone)]
pub enum SeatManagerEvent {
    /// A new seat was announced
    NewSeat {
        /// The id of the seat
        id: u32,
        /// The `wl_seat` protocol version
        version: u32,
        /// The `wl_registry`
        registry: Proxy<WlRegistry>,
    },
    /// A seat was removed
    RemoveSeat {
        /// The id of the seat
        id: u32,
    },
}

#[derive(Clone, Debug)]
/// The events of a seat
pub enum SeatEvent {
    /// A pointer event
    Pointer {
        /// The pointer event
        event: PointerEvent,
    },
    /// A keyboard event
    Keyboard {
        /// The keyboard event
        event: KeyboardEvent,
    },
    /// A touch event
    Touch {
        /// The touch event
        event: TouchEvent,
    },
    /// A data device event
    DataDevice {
        /// The data device event
        event: DataDeviceEvent,
    },
}

/// Seat event source specialized for different seat devices
pub struct SeatEventSource<T> {
    seat_id: u32,
    event_source: Option<EventSource<SurfaceEvent>>,
    _type: PhantomData<T>,
}

impl<T> SeatEventSource<T> {
    /// Creates a new `SeatEventSource`
    pub fn new(seat_id: u32) -> Self {
        SeatEventSource {
            seat_id,
            event_source: None,
            _type: PhantomData,
        }
    }

    /// The seat device entered a surface
    pub fn enter_surface(&mut self, surface: &Proxy<WlSurface>) {
        let surface_user_data = surface
            .user_data::<Mutex<SurfaceUserData>>()
            .unwrap()
            .lock()
            .unwrap();
        self.event_source = Some(surface_user_data.event_source.clone());
    }

    fn _queue_event(&self, event: SeatEvent) {
        let event = SurfaceEvent::Seat {
            seat_id: self.seat_id,
            event,
        };
        self.event_source.as_ref().unwrap().push_event(event);
    }
}

impl SeatEventSource<PointerEvent> {
    /// Queue a pointer event to a seat event source
    pub fn queue_event(&self, event: PointerEvent) {
        self._queue_event(SeatEvent::Pointer { event });
    }
}

impl SeatEventSource<KeyboardEvent> {
    /// Queue a keyboard event to a seat event source
    pub fn queue_event(&self, event: KeyboardEvent) {
        self._queue_event(SeatEvent::Keyboard { event });
    }
}

impl SeatEventSource<TouchEvent> {
    /// Queue a touch event to a seat event source
    pub fn queue_event(&self, event: TouchEvent) {
        self._queue_event(SeatEvent::Touch { event });
    }
}

impl SeatEventSource<DataDeviceEvent> {
    /// Queue a data device event to a seat event source
    pub fn queue_event(&self, event: DataDeviceEvent) {
        self._queue_event(SeatEvent::DataDevice { event });
    }
}
