//! Wayland protocol handling
pub mod clipboard;
pub mod compositor;
pub mod cursor;
pub mod data_device;
pub mod data_device_manager;
pub mod data_offer;
pub mod data_source;
pub mod environment;
pub mod event_queue;
pub mod keyboard;
pub mod layer_shell;
pub mod mem_pool;
pub mod output;
pub mod pipe;
pub mod pointer;
pub mod seat;
pub mod shm;
pub mod surface;
pub mod toplevel_manager;
pub mod touch;
pub mod xdg_shell;
pub mod xkbcommon;

pub use wayland_client::Proxy;
