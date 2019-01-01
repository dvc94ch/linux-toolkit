pub mod cursor;
pub mod environment;
pub mod event_queue;
pub mod keyboard;
pub mod mem_pool;
pub mod output;
pub mod pointer;
pub mod seat;
pub mod shm;
pub mod surface;
pub mod touch;
pub mod xdg_shell;

pub use wayland_client::Proxy;
