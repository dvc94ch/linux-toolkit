//!  Handles the global `wl_data_device_manager`
pub use wayland_client::protocol::wl_data_device_manager::{
    DndAction, RequestsTrait as DataDeviceManagerRequests, WlDataDeviceManager,
};
use wayland_client::{GlobalManager, Proxy};

/// Initializes the data device manager
///
/// Fails if the compositor did not advertise `wl_data_device_manager`.
pub fn initialize_data_device_manager(globals: &GlobalManager) -> Proxy<WlDataDeviceManager> {
    globals
        .instantiate_auto(|data_device_manager| {
            data_device_manager.implement(|event, _data_device_manager| match event {}, ())
        })
        .expect("Server didn't advertise `wl_data_device_manager`")
}
