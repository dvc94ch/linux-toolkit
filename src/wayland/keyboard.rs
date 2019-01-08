//! Keyboard handling
use crate::wayland::seat::SeatEventSource;
use crate::wayland::xkbcommon::KeyboardState;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};
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
    let mut repeat = Repeat::new(event_queue.clone());

    keyboard.implement(
        move |event, _keyboard| match event {
            Event::Keymap { format, fd, size } => {
                if KeymapFormat::XkbV1 == format {
                    state.load_keymap_from_fd(fd, size as usize);
                }
            }
            Event::RepeatInfo { rate, delay } => {
                repeat.set_info(rate as u32, delay as u32);
            }
            Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                serial,
            } => {
                let modifiers = state.update_modifiers(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                );
                event_queue.queue_event(KeyboardEvent::Modifiers {
                    modifiers,
                    serial,
                });
            }
            Event::Enter {
                surface,
                serial,
                keys,
            } => {
                let rawkeys: Vec<Keycode> = unsafe {
                    ::std::slice::from_raw_parts(
                        keys.as_ptr() as *const u32,
                        keys.len() / 4,
                    )
                    .to_vec()
                };
                let keysyms: Vec<Keysym> = rawkeys
                    .iter()
                    .map(|rawkey| state.get_sym(*rawkey))
                    .collect();

                event_queue.enter_surface(&surface);
                event_queue.queue_event(KeyboardEvent::Enter {
                    rawkeys,
                    keysyms,
                    serial,
                });
            }
            Event::Leave { surface: _, serial } => {
                repeat.abort();
                event_queue.queue_event(KeyboardEvent::Leave { serial });
            }
            Event::Key {
                serial,
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
                match keystate {
                    KeyState::Pressed => {
                        if state.key_repeats(rawkey) {
                            repeat.start(KeyInfo {
                                rawkey,
                                keysym,
                                state: keystate,
                                utf8: utf8.clone(),
                                time,
                                serial,
                            });
                        }
                    }
                    KeyState::Released => {
                        repeat.abort();
                    }
                };
                event_queue.queue_event(KeyboardEvent::Key {
                    rawkey,
                    keysym,
                    state: keystate,
                    utf8,
                    time,
                    serial,
                });
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
        /// serial number of the event
        serial: u32,
        /// raw values of the currently pressed keys
        rawkeys: Vec<Keycode>,
        /// interpreted symbols of the currently pressed keys
        keysyms: Vec<Keysym>,
    },
    /// The keyboard focus has left a surface
    Leave {
        /// serial number of the event
        serial: u32,
    },
    /// A key event occurred
    Key {
        /// serial number of the event
        serial: u32,
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
        /// serial number of the event
        serial: u32,
        /// current state of the modifiers
        modifiers: ModifiersState,
    },
}

/// Keyboard repeat handler
pub struct Repeat {
    rate: u32,
    delay: u32,
    key_held: bool,
    event_queue: SeatEventSource<KeyboardEvent>,
    kill_chan: Arc<Mutex<(Sender<()>, Receiver<()>)>>,
}

impl Repeat {
    /// Creates a new `Repeat`
    pub fn new(event_queue: SeatEventSource<KeyboardEvent>) -> Self {
        Repeat {
            rate: 0,
            delay: 0,
            event_queue,
            key_held: false,
            kill_chan: Arc::new(Mutex::new(channel::<()>())),
        }
    }

    /// Sets the repeat rate and delay
    pub fn set_info(&mut self, rate: u32, delay: u32) {
        self.rate = rate;
        self.delay = delay;
    }

    /// Start the key repeat timer loop
    pub fn start(&mut self, mut key: KeyInfo) {
        // If a key is being held then kill its repeat thread
        self.abort();
        self.key_held = true;

        if self.rate == 0 || self.delay == 0 {
            return;
        }

        // Clone variables for the thread
        let event_queue = self.event_queue.clone();
        let thread_kill_chan = self.kill_chan.clone();
        let rate = self.rate;
        let delay = self.delay;

        // Start new key repeat thread
        thread::spawn(move || {
            let time_tracker = Instant::now();
            // Delay
            thread::sleep(Duration::from_millis(delay as _));
            match thread_kill_chan.lock().unwrap().1.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => return,
                _ => {}
            }
            loop {
                let elapsed_time = time_tracker.elapsed();
                key.time += elapsed_time.as_secs() as u32 * 1000
                    + elapsed_time.subsec_nanos() / 1_000_000;

                let mut release_event = key.clone();
                release_event.state = KeyState::Released;
                release_event.utf8 = None;
                event_queue.queue_event(release_event.into());

                let mut press_event = key.clone();
                press_event.state = KeyState::Pressed;
                event_queue.queue_event(press_event.into());

                // Rate
                thread::sleep(Duration::from_millis(rate as _));
                match thread_kill_chan.lock().unwrap().1.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        break
                    }
                    _ => {}
                }
            }
        });
    }

    /// Abort previous key repeat thread
    pub fn abort(&mut self) {
        if self.key_held {
            self.kill_chan.lock().unwrap().0.send(()).unwrap();
            self.key_held = false;
        }
    }
}

#[derive(Clone, Debug)]
/// A key event occurred
pub struct KeyInfo {
    /// serial number of the event
    serial: u32,
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
}

impl Into<KeyboardEvent> for KeyInfo {
    fn into(self) -> KeyboardEvent {
        KeyboardEvent::Key {
            serial: self.serial,
            time: self.time,
            rawkey: self.rawkey,
            keysym: self.keysym,
            state: self.state,
            utf8: self.utf8,
        }
    }
}
