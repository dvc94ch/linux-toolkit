use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_pointer::WlPointer;
pub use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_pointer::Event;
pub use wayland_client::protocol::wl_pointer::ButtonState;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

pub fn implement_pointer(pointer: NewProxy<WlPointer>) -> Proxy<WlPointer> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    pointer.implement(move |event, _pointer| {
        match event.clone() {
            Event::Enter {
                surface,
                surface_x: x,
                surface_y: y,
                serial: _,
            } => {
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Enter { x, y }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Leave { surface: _, serial: _ } => {
                let event = SurfaceEvent::Pointer {
                    event: PointerEvent::Leave
                };
                event_source.as_ref().unwrap().push_event(event);
                event_source = None;
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
            //PointerEvent::Axis { axis, value, time } => {},
            //PointerEvent::AxisSource { axis_source } => {},
            //PointerEvent::AxisStop { axis, time } => {},
            //PointerEvent::AxisDiscrete { axis, discrete } => {},
            //PointerEvent::Frame {} => {},
            _ => {},
        }
    }, ())
}

#[derive(Clone, Debug)]
pub enum PointerEvent {
    Enter { x: f64, y: f64 },
    Leave,
    Button { button: u32, state: ButtonState, time: u32 },
    Motion { x: f64, y: f64, time: u32 },
}

// TODO handle cursor
