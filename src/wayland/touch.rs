//! Touch screen handling
use crate::wayland::seat::SeatEventSource;
use wayland_client::protocol::wl_touch::Event;
pub use wayland_client::protocol::wl_touch::RequestsTrait as TouchRequests;
pub use wayland_client::protocol::wl_touch::WlTouch;
use wayland_client::{NewProxy, Proxy};

/// Handles `wl_touch` events and forwards the ones
/// that need user handling to an event queue.
pub fn implement_touch(
    touch: NewProxy<WlTouch>,
    mut event_queue: SeatEventSource<TouchEvent>,
) -> Proxy<WlTouch> {
    touch.implement(
        move |event, _touch| match event {
            Event::Down {
                surface,
                x,
                y,
                serial,
                time,
                id,
            } => {
                event_queue.enter_surface(&surface);
                event_queue.queue_event(TouchEvent::Down {
                    x,
                    y,
                    time,
                    id,
                    serial,
                });
            }
            Event::Up { serial, time, id } => {
                event_queue.queue_event(TouchEvent::Up { time, id, serial });
            }
            Event::Motion { x, y, time, id } => {
                event_queue.queue_event(TouchEvent::Motion { x, y, time, id });
            }
            Event::Cancel => {
                event_queue.queue_event(TouchEvent::Cancel);
            }
            Event::Frame => {
                event_queue.queue_event(TouchEvent::Frame);
            }
        },
        (),
    )
}

/// Possible events generated from a `wl_touch` device
#[derive(Clone, Debug)]
pub enum TouchEvent {
    /// A finger touched your surface
    Down {
        /// horizontal location on the surface
        x: f64,
        /// vertical location on the surface
        y: f64,
        /// The time of this event
        time: u32,
        /// The finger id of this event for multitouch handling
        id: i32,
        /// serial number of the event
        serial: u32,
    },
    /// A finger stopped touching your surface
    Up {
        /// The time of this event
        time: u32,
        /// The finger id of this event for multitouch handling
        id: i32,
        /// serial number of the event
        serial: u32,
    },
    /// A finger was moved on your surface
    Motion {
        /// new horizontal location
        x: f64,
        /// new vertical location
        y: f64,
        /// The time of this event
        time: u32,
        /// The finger id of this event for multitouch handling
        id: i32,
    },
    /// A finger left your surface
    Cancel,
    /// End of event batch
    Frame,
}
