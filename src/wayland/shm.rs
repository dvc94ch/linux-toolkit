use std::sync::Mutex;
use wayland_client::{Proxy, GlobalManager};
pub use wayland_client::protocol::wl_shm::WlShm;
pub use wayland_client::protocol::wl_shm::RequestsTrait as ShmRequests;
pub use wayland_client::protocol::wl_shm::Format;
use wayland_client::protocol::wl_shm::Event;

pub struct ShmUserData {
    pub formats: Vec<Format>,
}

impl ShmUserData {
    pub fn new() -> Self {
        ShmUserData {
            formats: Vec::new(),
        }
    }
}

pub fn initialize_shm(globals: &GlobalManager) -> Proxy<WlShm> {
    globals
        .instantiate_auto(|shm| {
            shm.implement(move |event, shm| match event {
                Event::Format { format } => {
                    shm.user_data::<Mutex<ShmUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .formats
                        .push(format)
                }
            }, Mutex::new(ShmUserData::new()))
        })
        .expect("Server didn't advertise `wl_shm`")
}
