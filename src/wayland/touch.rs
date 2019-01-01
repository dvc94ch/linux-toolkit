use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_touch::WlTouch;
pub use wayland_client::protocol::wl_touch::RequestsTrait as TouchRequests;
pub use wayland_client::protocol::wl_touch::Event as TouchEvent;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

pub fn implement_touch(touch: NewProxy<WlTouch>) -> Proxy<WlTouch> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    touch.implement(move |event, _touch| {
        match event.clone() {
            TouchEvent::Down {
                surface,
                x: _,
                y: _,
                serial: _,
                time: _,
                id: _,
            } => {
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Touch { event };
                event_source.as_ref().unwrap().push_event(event);
            },
            //TouchEvent::Up { serial, time, id } => {},
            //TouchEvent::Motion { x, y, time, id } => {},
            //TouchEvent::Frame {} => {},
            //TouchEvent::Cancel {} => {},
            _ => {
                let event = SurfaceEvent::Touch { event };
                event_source.as_ref().unwrap().push_event(event);
            }
        }
    }, ())
}
