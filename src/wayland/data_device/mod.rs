//! Helpers to handle data device related actions

mod data_device;
mod data_offer;
mod data_source;

use wayland_client::{GlobalManager, Proxy};
pub use wayland_client::protocol::wl_data_device_manager::{
    WlDataDeviceManager, DndAction
};

pub use self::data_device::{DataDevice, DndEvent};
pub use self::data_offer::{DataOffer, ReadPipe};
pub use self::data_source::{DataSource, DataSourceEvent, WritePipe};

pub fn initialize_data_device_manager(
    globals: &GlobalManager
) -> Proxy<WlDataDeviceManager> {
    globals
        .instantiate_auto(|data_device_manager| {
            data_device_manager.implement(
                |event, _data_device_manager| match event {}, ())
        })
        .expect("Server didn't advertise `wl_data_device_manager`")
}
