//! Pointer handling
use crate::wayland::cursor::Cursor;
use crate::wayland::seat::SeatEventSource;
use std::sync::Mutex;
use wayland_client::protocol::wl_pointer::Event;
pub use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
pub use wayland_client::protocol::wl_pointer::WlPointer;
pub use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};
use wayland_client::{NewProxy, Proxy};

/// Handles `wl_pointer` events and forwards the ones
/// that need user handling to an event queue.
pub fn implement_pointer(
    pointer: NewProxy<WlPointer>,
    mut event_queue: SeatEventSource<PointerEvent>,
    cursor: Cursor,
) -> Proxy<WlPointer> {
    pointer.implement(
        move |event, pointer| match event {
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

                event_queue.enter_surface(&surface);
                event_queue.queue_event(PointerEvent::Enter {
                    cursor,
                    x,
                    y,
                    serial,
                });
            }
            Event::Leave { surface: _, serial } => {
                event_queue.queue_event(PointerEvent::Leave { serial });
            }
            Event::Button {
                button,
                state,
                time,
                serial,
            } => {
                let button = MouseButton::from(button);
                event_queue.queue_event(PointerEvent::Button {
                    button,
                    state,
                    time,
                    serial,
                });
            }
            Event::Motion {
                surface_x: x,
                surface_y: y,
                time,
            } => {
                event_queue.queue_event(PointerEvent::Motion { x, y, time });
            }
            Event::Axis { axis, value, time } => {
                event_queue.queue_event(PointerEvent::Axis {
                    axis,
                    value,
                    time,
                });
            }
            Event::AxisSource { axis_source } => {
                event_queue
                    .queue_event(PointerEvent::AxisSource { axis_source });
            }
            Event::AxisStop { axis, time } => {
                event_queue.queue_event(PointerEvent::AxisStop { axis, time });
            }
            Event::AxisDiscrete { axis, discrete } => {
                event_queue
                    .queue_event(PointerEvent::AxisDiscrete { axis, discrete });
            }
            Event::Frame => {
                event_queue.queue_event(PointerEvent::Frame);
            }
        },
        Mutex::new(PointerUserData::new(cursor)),
    )
}

#[derive(Clone, Copy, Debug)]
/// Mouse button
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
    /// Other mouse button
    Other(u8),
}

impl From<u32> for MouseButton {
    fn from(button: u32) -> MouseButton {
        match button - 0x110 {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            i => MouseButton::Other(i as u8)
        }
    }
}

/// Possible events generated from a `wl_pointer` device
#[derive(Clone, Debug)]
pub enum PointerEvent {
    /// A `wl_pointer` entered your surface
    Enter {
        /// The cursor
        cursor: Cursor,
        /// horizontal location on the surface
        x: f64,
        /// vertical location on the surface
        y: f64,
        /// serial number of the event
        serial: u32,
    },
    /// A `wl_pointer` left your surface
    Leave {
        /// serial number of the event
        serial: u32,
    },
    /// A mouse button was pressed or released
    Button {
        /// The button that was pressed
        button: MouseButton,
        /// The state of the button
        state: ButtonState,
        /// The time of this event
        time: u32,
        /// serial number of the event
        serial: u32,
    },
    /// The mouse moved
    Motion {
        /// new horizontal location
        x: f64,
        /// new vertical location
        y: f64,
        /// The time of this event
        time: u32,
    },
    /// The pointing device is scrolling
    Axis {
        /// The direction that was scrolled
        axis: Axis,
        /// The amount that was scrolled
        value: f64,
        /// The time of this event
        time: u32,
    },
    /// The source of the scroll motion
    AxisSource {
        /// The source of the scroll motion
        axis_source: AxisSource,
    },
    /// The pointing device stopped scrolling
    AxisStop {
        /// The direction that was scrolled
        axis: Axis,
        /// The time of this event
        time: u32,
    },
    /// The pointing device is scrolling
    AxisDiscrete {
        /// The direction that was scrolled
        axis: Axis,
        /// The amount that was scrolled
        discrete: i32,
    },
    /// End of event batch
    Frame,
}

/// The `wl_pointer` user data
pub struct PointerUserData {
    cursor: Cursor,
}

impl PointerUserData {
    /// Creates a new `PointerUserData`
    pub fn new(cursor: Cursor) -> Self {
        PointerUserData { cursor }
    }
}
