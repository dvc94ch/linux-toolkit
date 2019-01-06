//! Handles the `wl_shm` globals.
use std::sync::Mutex;
use wayland_client::protocol::wl_shm::Event;
pub use wayland_client::protocol::wl_shm::Format;
pub use wayland_client::protocol::wl_shm::RequestsTrait as ShmRequests;
pub use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{GlobalManager, Proxy};

struct ShmUserData {
    formats: Vec<Format>,
}

impl ShmUserData {
    fn new() -> Self {
        ShmUserData {
            formats: Vec::new(),
        }
    }

    fn formats(&self) -> &Vec<Format> {
        &self.formats
    }
}

/// Returns the formats supported by `wl_shm`
pub fn formats(shm: &Proxy<WlShm>) -> Vec<Format> {
    shm.user_data::<Mutex<ShmUserData>>()
        .unwrap()
        .lock()
        .unwrap()
        .formats()
        .clone()
}

/// Initializes the `wl_shm`
///
/// Fails if the compositor did not advertise `wl_shm`.
pub fn initialize_shm(globals: &GlobalManager) -> Proxy<WlShm> {
    globals
        .instantiate_auto(|shm| {
            shm.implement(
                move |event, shm| match event {
                    Event::Format { format } => shm
                        .user_data::<Mutex<ShmUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .formats
                        .push(format),
                },
                Mutex::new(ShmUserData::new()),
            )
        })
        .expect("Server didn't advertise `wl_shm`")
}
