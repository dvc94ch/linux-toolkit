//! Keyboard handling
use crate::wayland::seat::SeatEventSource;
use crate::wayland::xkbcommon::KeyboardState;
pub use crate::wayland::xkbcommon::{Keycode, Keysym, ModifiersState};
use wayland_client::protocol::wl_keyboard::Event;
pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::protocol::wl_keyboard::KeymapFormat;
pub use wayland_client::protocol::wl_keyboard::RequestsTrait as KeyboardRequests;
pub use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::{NewProxy, Proxy};

/// Handles `wl_keyboard` events and forwards the ones
/// that need user handling to an event queue.
pub fn implement_keyboard(
    keyboard: NewProxy<WlKeyboard>,
    mut event_queue: SeatEventSource<KeyboardEvent>,
) -> Proxy<WlKeyboard> {
    let mut state = KeyboardState::new();

    keyboard.implement(
        move |event, _keyboard| {
            match event.clone() {
                Event::Keymap { format, fd, size } => {
                    if KeymapFormat::XkbV1 == format {
                        state.load_keymap_from_fd(fd, size as usize);
                    }
                }
                Event::RepeatInfo { rate, delay } => {
                    state.set_repeat_info(rate as u32, delay as u32);
                }
                Event::Modifiers {
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                    serial: _,
                } => {
                    let modifiers =
                        state.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
                    event_queue.queue_event(KeyboardEvent::Modifiers { modifiers });
                }
                Event::Enter {
                    surface,
                    serial: _,
                    keys,
                } => {
                    let rawkeys: Vec<Keycode> = unsafe {
                        ::std::slice::from_raw_parts(keys.as_ptr() as *const u32, keys.len() / 4)
                            .to_vec()
                    };
                    let keysyms: Vec<Keysym> = rawkeys
                        .iter()
                        .map(|rawkey| state.get_sym(*rawkey))
                        .collect();

                    event_queue.enter_surface(&surface);
                    event_queue.queue_event(KeyboardEvent::Enter { rawkeys, keysyms });
                }
                Event::Leave {
                    surface: _,
                    serial: _,
                } => {
                    // TODO abort repeat
                    event_queue.queue_event(KeyboardEvent::Leave);
                }
                Event::Key {
                    serial: _,
                    time,
                    key: rawkey,
                    state: keystate,
                } => {
                    let keysym = state.get_sym(rawkey);
                    let utf8 = match keystate {
                        KeyState::Pressed => state
                            .compose(keysym)
                            .ok()
                            .unwrap_or_else(|| state.get_utf8(rawkey)),
                        KeyState::Released => None,
                    };
                    // TODO start repeat thread
                    event_queue.queue_event(KeyboardEvent::Key {
                        rawkey,
                        keysym,
                        state: keystate,
                        utf8,
                        time,
                    });
                }
            }
        },
        (),
    )
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
