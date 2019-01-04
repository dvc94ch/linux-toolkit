use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_pointer::WlPointer;
pub use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_pointer::Event;
pub use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};
use crate::wayland::cursor::{Cursor, CursorManager};
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

pub fn implement_pointer(
    pointer: NewProxy<WlPointer>,
    cursor_manager: CursorManager,
) -> Proxy<WlPointer> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    pointer.implement(move |event, pointer| {
        match event.clone() {
            Event::Enter {
                surface,
                surface_x: x,
                surface_y: y,
                serial,
            } => {
                let pointer_user_data = pointer
                    .user_data::<Mutex<PointerUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                let cursor = pointer_user_data.cursor.clone();
                cursor.enter_surface(pointer.clone(), serial);

                let surface_user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(surface_user_data.event_source.clone());
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Enter { cursor, x, y }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Leave { surface: _, serial: _ } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Leave
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Button { button, state, time, serial: _ } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Button {
                        button,
                        state,
                        time,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Motion { surface_x: x, surface_y: y, time } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Motion {
                        x,
                        y,
                        time,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Axis { axis, value, time } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Axis {
                        axis,
                        value,
                        time,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::AxisSource { axis_source } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::AxisSource {
                        axis_source,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::AxisStop { axis, time } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::AxisStop {
                        axis,
                        time,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::AxisDiscrete { axis, discrete } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::AxisDiscrete {
                        axis,
                        discrete,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Frame => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Frame
                };
                event_source.as_ref().unwrap().push_event(event);
            },
        }
    }, Mutex::new(PointerUserData::new(cursor_manager)))
}

#[derive(Clone, Debug)]
pub enum PointerEvent {
    Enter { cursor: Cursor, x: f64, y: f64 },
    Leave,
    Button { button: u32, state: ButtonState, time: u32 },
    Motion { x: f64, y: f64, time: u32 },
    Axis { axis: Axis, value: f64, time: u32 },
    AxisSource { axis_source: AxisSource },
    AxisStop { axis: Axis, time: u32 },
    AxisDiscrete { axis: Axis, discrete: i32 },
    Frame,
}

pub struct PointerUserData {
    cursor_manager: CursorManager,
    cursor: Cursor,
}

impl PointerUserData {
    pub fn new(cursor_manager: CursorManager) -> Self {
        let cursor = cursor_manager.new_cursor(None);
        PointerUserData {
            cursor_manager,
            cursor,
        }
    }
}

impl Drop for PointerUserData {
    fn drop(&mut self) {
        self.cursor_manager.remove_cursor(&self.cursor)
    }
}
