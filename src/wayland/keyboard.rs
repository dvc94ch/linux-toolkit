use std::sync::Mutex;
use xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1;
pub use xkbcommon::xkb::{Keysym, KeyDirection};

use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_keyboard::WlKeyboard;
pub use wayland_client::protocol::wl_keyboard::RequestsTrait as KeyboardRequests;
pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::protocol::wl_keyboard::Event;
use wayland_client::protocol::wl_keyboard::KeymapFormat;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{WlSurface, SurfaceEvent, SurfaceUserData};
use crate::wayland::xkbcommon::KbState;
pub use crate::wayland::xkbcommon::ModifiersState;

// TODO map keyboard auto with repeat
pub fn implement_keyboard(keyboard: NewProxy<WlKeyboard>) -> Proxy<WlKeyboard> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    let mut kb_state = KbState::new();

    keyboard.implement(move |event, _keyboard| {
        match event.clone() {
            Event::Keymap { format, fd, size } => {
                let format = match format {
                    KeymapFormat::XkbV1 => KEYMAP_FORMAT_TEXT_V1,
                    KeymapFormat::NoKeymap => {
                        panic!("Compositor did not send a keymap.");
                    }
                };
                kb_state.load_keymap_from_fd(format, fd, size as usize);
            },
            Event::RepeatInfo { rate, delay } => {
                kb_state.set_repeat_info(rate as u32, delay as u32);
            },
            Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                serial: _,
            } => {
                // TODO update modifiers
                kb_state.set_modifiers(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                );
            },
            Event::Enter {
                surface,
                serial: _,
                keys: _,
            } => {
                // TODO pass keys to xkb state
                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                //let event = SurfaceEvent::Keyboard { event };
                //event_source.as_ref().unwrap().push_event(event);
            },
            Event::Leave { surface: _, serial: _ } => {
                // TODO abort repeat
                // TODO send release events
                //let event = SurfaceEvent::Keyboard { event };
                //event_source.take().unwrap().push_event(event);
            },
            Event::Key { serial: _, time: _, key, state } => {
                // TODO pass key to xkb state
                // TODO handle compose
                // TODO start repeat thread
                let dir = match state {
                    KeyState::Pressed => KeyDirection::Down,
                    KeyState::Released => KeyDirection::Up,
                };
                kb_state.key(key, dir);
                //let event = SurfaceEvent::Keyboard { event };
                //event_source.as_ref().unwrap().push_event(event);
            }
        }
    }, ())
}

/// Events received from a mapped keyboard
#[derive(Clone)]
pub enum KeyboardEvent {
    /// The keyboard focus has entered a surface
    Enter {
        /// surface that was entered
        surface: Proxy<WlSurface>,
        /// raw values of the currently pressed keys
        rawkeys: Vec<u32>,
        /// interpreted symbols of the currently pressed keys
        keysyms: Vec<Keysym>,
    },
    /// The keyboard focus has left a surface
    Leave {
        /// surface that was left
        surface: Proxy<WlSurface>,
    },
    /// A key event occurred
    Key {
        /// time at which the keypress occurred
        time: u32,
        /// raw value of the key
        rawkey: u32,
        /// interpreted symbol of the key
        keysym: Keysym,
        /// new state of the key
        state: KeyState,
        /// utf8 interpretation of the entered text
        ///
        /// will always be `None` on key release events
        utf8: Option<String>,
        /// physical or emulated keypress due to repeat info
        r#virtual: bool,
    },
    /// Repetition information advertising
    RepeatInfo {
        /// rate (in millisecond) at which the repetition should occur
        rate: i32,
        /// delay (in millisecond) between a key press and the start of repetition
        delay: i32,
    },
    /// The key modifiers have changed state
    Modifiers {
        /// current state of the modifiers
        modifiers: ModifiersState,
    },
}
