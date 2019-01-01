use std::fs::File;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;
use memmap::MmapOptions;
use xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1;

use wayland_client::{Proxy, NewProxy};
pub use wayland_client::protocol::wl_keyboard::WlKeyboard;
pub use wayland_client::protocol::wl_keyboard::RequestsTrait as KeyboardRequests;
pub use wayland_client::protocol::wl_keyboard::Event as KeyboardEvent;
use wayland_client::protocol::wl_keyboard::KeymapFormat;
use crate::wayland::event_queue::EventSource;
use crate::wayland::surface::{SurfaceEvent, SurfaceUserData};
use crate::wayland::xkbcommon::KbState;

// TODO map keyboard auto with repeat
pub fn implement_keyboard(keyboard: NewProxy<WlKeyboard>) -> Proxy<WlKeyboard> {
    let mut event_source: Option<EventSource<SurfaceEvent>> = None;
    let mut kb_state = KbState::new();

    keyboard.implement(move |event, _keyboard| {
        match event.clone() {
            KeyboardEvent::Keymap { format, fd, size } => {
                let map = MmapOptions::new()
                    .len(size as usize)
                    .map(unsafe { &File::from_raw_fd(fd) })
                    .unwrap();
                let format = match format {
                    KeymapFormat::XkbV1 => KEYMAP_FORMAT_TEXT_V1,
                    KeymapFormat::NoKeymap => {
                        panic!("Compositor did not send a keymap.");
                    }
                };
                kb_state.load_keymap_from_file(format, map);
            },
            KeyboardEvent::RepeatInfo { rate, delay } => {
                kb_state.set_repeat_info(rate as u32, delay as u32);
            },
            KeyboardEvent::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                serial: _,
            } => {
                kb_state.set_modifiers(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                );
            },
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
            KeyboardEvent::Key { serial: _, time: _, key: _, state: _ } => {
                let event = SurfaceEvent::Keyboard { event };
                event_source.as_ref().unwrap().push_event(event);
            }
        }
    }, ())
}
