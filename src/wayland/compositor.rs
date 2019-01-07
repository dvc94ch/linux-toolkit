//! Handles the `wl_compositor` and `wl_subcompositor` globals.
pub use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
pub use wayland_client::protocol::wl_compositor::WlCompositor;
pub use wayland_client::protocol::wl_subcompositor::RequestsTrait as SubcompositorRequests;
pub use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::{GlobalManager, Proxy};

/// Initializes the `wl_compositor`
///
/// Fails if the compositor did not advertise `wl_compositor`.
pub fn initialize_compositor(globals: &GlobalManager) -> Proxy<WlCompositor> {
    globals
        .instantiate_auto(|compositor| {
            compositor.implement(|event, _compositor| match event {}, ())
        })
        .expect("Server didn't advertise `wl_compositor`")
}

/// Initializes the `wl_subcompositor`
///
/// Fails if the compositor did not advertise `wl_subcompositor`.
pub fn initialize_subcompositor(
    globals: &GlobalManager,
) -> Proxy<WlSubcompositor> {
    globals
        .instantiate_auto(|subcompositor| {
            subcompositor.implement(|event, _subcompositor| match event {}, ())
        })
        .expect("Server didn't advertise `wl_subcompositor`")
}
