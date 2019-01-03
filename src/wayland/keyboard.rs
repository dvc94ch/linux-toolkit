use std::sync::Mutex;
use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_keyboard::WlKeyboard;
pub use wayland_client::protocol::wl_keyboard::RequestsTrait as KeyboardRequests;
pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::protocol::wl_keyboard::Event;
use wayland_client::protocol::wl_keyboard::KeymapFormat;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};
use crate::wayland::xkbcommon::KbState;
pub use crate::wayland::xkbcommon::{Keycode, Keysym, ModifiersState};

pub fn implement_keyboard(keyboard: NewProxy<WlKeyboard>) -> Proxy<WlKeyboard> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    let mut kb_state = KbState::new();

    keyboard.implement(move |event, _keyboard| {
        match event.clone() {
            Event::Keymap { format, fd, size } => {
                if KeymapFormat::XkbV1 == format {
                    kb_state.load_keymap_from_fd(fd, size as usize);
                }
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
                let modifiers = kb_state.update_modifiers(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                );
                let event = SurfaceEvent::Keyboard {
                    event: KeyboardEvent::Modifiers {
                        modifiers
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Enter {
                surface,
                serial: _,
                keys,
            } => {
                let rawkeys: Vec<Keycode> = unsafe {
                    ::std::slice::from_raw_parts(
                        keys.as_ptr() as *const u32,
                        keys.len() / 4,
                    ).to_vec()
                };
                let keysyms: Vec<Keysym> = rawkeys
                    .iter()
                    .map(|rawkey| kb_state.get_sym(*rawkey))
                    .collect();

                let user_data = surface
                    .user_data::<Mutex<SurfaceUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                event_source = Some(user_data.event_source.clone());
                let event = SurfaceEvent::Keyboard {
                    event: KeyboardEvent::Enter {
                        rawkeys,
                        keysyms,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            },
            Event::Leave { surface: _, serial: _ } => {
                // TODO abort repeat
                let event = SurfaceEvent::Keyboard {
                    event: KeyboardEvent::Leave
                };
                event_source.take().unwrap().push_event(event);
            },
            Event::Key { serial: _, time, key: rawkey, state } => {
                let keysym = kb_state.get_sym(rawkey);
                let utf8 = match state {
                    KeyState::Pressed => {
                        kb_state.compose(keysym).unwrap_or_else(|| {
                            kb_state.get_utf8(rawkey)
                        })
                    }
                    KeyState::Released => None
                };
                // TODO start repeat thread
                let event = SurfaceEvent::Keyboard {
                    event: KeyboardEvent::Key {
                        rawkey,
                        keysym,
                        state,
                        utf8,
                        time,
                    }
                };
                event_source.as_ref().unwrap().push_event(event);
            }
        }
    }, ())
}

/// Events received from a mapped keyboard
#[derive(Clone, Debug)]
pub enum KeyboardEvent {
    /// The keyboard focus has entered a surface
    Enter {
        /// raw values of the currently pressed keys
        rawkeys: Vec<Keycode>,
        /// interpreted symbols of the currently pressed keys
        keysyms: Vec<Keysym>,
    },
    /// The keyboard focus has left a surface
    Leave,
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
        // physical or emulated keypress due to repeat info
        //is_repeat: bool,
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
