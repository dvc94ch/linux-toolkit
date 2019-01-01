use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_pointer::WlPointer;
pub use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
pub use wayland_client::protocol::wl_pointer::Event as PointerEvent;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

pub fn implement_pointer(pointer: NewProxy<WlPointer>) -> Proxy<WlPointer> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    pointer.implement(move |event, _pointer| {
        match event.clone() {
            PointerEvent::Enter {
                surface,
                surface_x: _,
                surface_y: _,
                serial: _,
            } => {
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Pointer { event };
                event_source.as_ref().unwrap().push_event(event);
            },
            //PointerEvent::Leave { surface, serial } => {},
            //PointerEvent::Button { button, state, time, serial } => {},
            //PointerEvent::Motion { surface_x, surface_y, time } => {},
            //PointerEvent::Axis { axis, value, time } => {},
            //PointerEvent::AxisSource { axis_source } => {},
            //PointerEvent::AxisStop { axis, time } => {},
            //PointerEvent::AxisDiscrete { axis, discrete } => {},
            //PointerEvent::Frame {} => {},
            _ => {
                let event = SurfaceEvent::Pointer { event };
                event_source.as_ref().unwrap().push_event(event);
            },
        }
    }, ())
}

// TODO handle cursor
