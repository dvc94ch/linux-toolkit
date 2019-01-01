use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_registry::RequestsTrait as RegistryRequests;
pub use wayland_client::protocol::wl_seat::WlSeat;
pub use wayland_client::protocol::wl_seat::RequestsTrait as SeatRequests;
use wayland_client::protocol::wl_seat::{Capability, Event};
use crate::wayland::event_queue::EventSource;
use crate::wayland::keyboard::{WlKeyboard, KeyboardRequests, implement_keyboard};
use crate::wayland::pointer::{WlPointer, PointerRequests, implement_pointer};
use crate::wayland::surface::SurfaceManagerEvent;
use crate::wayland::touch::{WlTouch, TouchRequests, implement_touch};

#[derive(Clone)]
pub struct SeatManager {
    sm_source: EventSource<SurfaceManagerEvent>,
    seats: Arc<Mutex<Vec<Proxy<WlSeat>>>>,
}

impl SeatManager {
    pub fn new(sm_source: EventSource<SurfaceManagerEvent>) -> Self {
        SeatManager {
            sm_source,
            seats: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn new_seat(
        &self,
        seat_id: u32,
        version: u32,
        registry: &Proxy<WlRegistry>,
    ) {
        let seat = registry
            .bind(version, seat_id, |seat| {
                seat.implement(move |event, seat| {
                    let mut user_data = seat
                        .user_data::<Mutex<SeatUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap();

                    match event {
                        Event::Name { name } => {
                            user_data.name = name;
                        }
                        Event::Capabilities { capabilities } => {
                            if capabilities.contains(Capability::Pointer) {
                                user_data.pointer = seat.get_pointer(|pointer| {
                                    implement_pointer(pointer)
                                }).ok();
                            } else {
                                if user_data.pointer.is_some() {
                                    let pointer = user_data.pointer.take().unwrap();
                                    if pointer.version() >= 3 {
                                        pointer.release();
                                    }
                                }
                            }
                            if capabilities.contains(Capability::Keyboard) {
                                user_data.keyboard = seat.get_keyboard(|keyboard| {
                                    implement_keyboard(keyboard)
                                }).ok();
                            } else {
                                let keyboard = user_data.keyboard.take().unwrap();
                                if keyboard.version() >= 3 {
                                    keyboard.release();
                                }
                            }
                            if capabilities.contains(Capability::Touch) {
                                user_data.touch = seat.get_touch(|touch| {
                                    implement_touch(touch)
                                }).ok();
                            } else {
                                let touch = user_data.touch.take().unwrap();
                                if touch.version() >= 3 {
                                    touch.release();
                                }
                            }
                        }
                    }
                }, Mutex::new(SeatUserData::new()))
            }).unwrap();
        self.seats.lock().unwrap().push(seat);
    }

    pub fn remove_seat(&self, seat_id: u32) {
        self.seats.lock().unwrap().retain(|seat| {
            if seat.id() == seat_id && seat.version() >= 5 {
                seat.release();
            }
            seat.id() != seat_id
        });
    }

    pub fn seats(&self) -> &Arc<Mutex<Vec<Proxy<WlSeat>>>> {
        &self.seats
    }

    pub fn get_seat(&self, seat_id: u32) -> Option<Proxy<WlSeat>> {
        self.seats.lock().unwrap().iter().find(|seat| {
            seat.id() == seat_id
        }).map(|seat| seat.clone())
    }
}

#[derive(Clone)]
pub struct SeatUserData {
    name: String,
    pointer: Option<Proxy<WlPointer>>,
    keyboard: Option<Proxy<WlKeyboard>>,
    touch: Option<Proxy<WlTouch>>,
}

impl SeatUserData {
    pub fn new() -> Self {
        SeatUserData {
            name: String::new(),
            pointer: None,
            keyboard: None,
            touch: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name[..]
    }

    pub fn pointer(&self) -> &Option<Proxy<WlPointer>> {
        &self.pointer
    }

    pub fn keyboard(&self) -> &Option<Proxy<WlKeyboard>> {
        &self.keyboard
    }

    pub fn touch(&self) -> &Option<Proxy<WlTouch>> {
        &self.touch
    }
}