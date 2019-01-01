use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_keyboard::WlKeyboard;
pub use wayland_client::protocol::wl_keyboard::RequestsTrait as KeyboardRequests;
pub use wayland_client::protocol::wl_keyboard::Event as KeyboardEvent;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};

// TODO map keyboard auto with repeat
pub fn implement_keyboard(keyboard: NewProxy<WlKeyboard>) -> Proxy<WlKeyboard> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    keyboard.implement(move |event, _keyboard| {
        match event.clone() {
            KeyboardEvent::Keymap { format: _, fd: _, size: _ } => {},
            KeyboardEvent::RepeatInfo { rate: _, delay: _ } => {},
            KeyboardEvent::Enter {
                surface,
                serial: _,
                keys: _,
            } => {
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Keyboard { event };
                event_source.as_ref().unwrap().push_event(event);
            },
            KeyboardEvent::Leave { surface: _, serial: _ } => {
                let event = SurfaceEvent::Keyboard { event };
                event_source.take().unwrap().push_event(event);
            },
            //KeyboardEvent::Key { serial, time, key, state } => {},
            //KeyboardEvent::Modifiers {
            //    mods_depressed,
            //    mods_latched,
            //    mods_locked,
            //    group,
            //    ..
            //} => {},
            _ => {
                let event = SurfaceEvent::Keyboard { event };
                event_source.as_ref().unwrap().push_event(event);
            }
        }
    }, ())
}
