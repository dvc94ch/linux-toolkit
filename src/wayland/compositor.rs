use wayland_client::{GlobalManager, Proxy};
pub use wayland_client::protocol::wl_compositor::WlCompositor;
pub use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
pub use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
pub use wayland_client::protocol::wl_subcompositor::RequestsTrait as SubcompositorRequests;

pub fn initialize_compositor(globals: &GlobalManager) -> Proxy<WlCompositor> {
    globals
        .instantiate_auto(|compositor| {
            compositor.implement(|event, _compositor| match event {}, ())
        })
        .expect("Server didn't advertise `wl_compositor`")
}

pub fn initialize_subcompositor(globals: &GlobalManager) -> Proxy<WlSubcompositor> {
    globals
        .instantiate_auto(|subcompositor| {
            subcompositor.implement(|event, _subcompositor| match event {}, ())
        })
        .expect("Server didn't advertise `wl_subcompositor`")
}
