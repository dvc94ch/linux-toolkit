use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_touch::WlTouch;
pub use wayland_client::protocol::wl_touch::RequestsTrait as TouchRequests;
use wayland_client::protocol::wl_touch::Event;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

pub fn implement_touch(touch: NewProxy<WlTouch>) -> Proxy<WlTouch> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    touch.implement(move |event, _touch| {
        match event.clone() {
            Event::Down {
                surface,
                x,
                y,
                serial: _,
                time,
                id,
            } => {
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Touch {
                    event: TouchEvent::Down {
                        x,
                        y,
                        time,
                        id,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Up { serial: _, time, id } => {
                let event = SurfaceEvent::Touch {
                    event: TouchEvent::Up {
                        time,
                        id,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Motion { x, y, time, id } => {
                let event = SurfaceEvent::Touch {
                    event: TouchEvent::Motion {
                        x,
                        y,
                        time,
                        id,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Cancel => {
                let event = SurfaceEvent::Touch {
                    event: TouchEvent::Cancel
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Frame => {
                let event = SurfaceEvent::Touch {
                    event: TouchEvent::Frame
                };
                event_source.as_ref().unwrap().push_event(event);
            },
        }
    }, ())
}

#[derive(Clone, Debug)]
pub enum TouchEvent {
    Down { x: f64, y: f64, time: u32, id: i32 },
    Up { time: u32, id: i32 },
    Motion { x: f64, y: f64, time: u32, id: i32 },
    Cancel,
    Frame,
}
