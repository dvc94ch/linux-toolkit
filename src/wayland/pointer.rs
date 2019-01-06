//! Pointer handling
use crate::wayland::cursor::{Cursor, CursorManager};
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
    cursor_manager: CursorManager,
) -> Proxy<WlPointer> {
    pointer.implement(
        move |event, pointer| match event.clone() {
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
                event_queue.queue_event(PointerEvent::Enter { cursor, x, y });
            }
            Event::Leave {
                surface: _,
                serial: _,
            } => {
                event_queue.queue_event(PointerEvent::Leave);
            }
            Event::Button {
                button,
                state,
                time,
                serial: _,
            } => {
                event_queue.queue_event(PointerEvent::Button {
                    button,
                    state,
                    time,
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
                event_queue.queue_event(PointerEvent::Axis { axis, value, time });
            }
            Event::AxisSource { axis_source } => {
                event_queue.queue_event(PointerEvent::AxisSource { axis_source });
            }
            Event::AxisStop { axis, time } => {
                event_queue.queue_event(PointerEvent::AxisStop { axis, time });
            }
            Event::AxisDiscrete { axis, discrete } => {
                event_queue.queue_event(PointerEvent::AxisDiscrete { axis, discrete });
            }
            Event::Frame => {
                event_queue.queue_event(PointerEvent::Frame);
            }
        },
        Mutex::new(PointerUserData::new(cursor_manager)),
    )
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
    },
    /// A `wl_pointer` left your surface
    Leave,
    /// A mouse button was pressed or released
    Button {
        /// The button that was pressed
        button: u32,
        /// The state of the button
        state: ButtonState,
        /// The time of this event
        time: u32,
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

struct PointerUserData {
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
