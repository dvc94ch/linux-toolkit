//! Handles the `zwlr_layer_shell_v1` protocol.
use crate::wayland::event_queue::{EventDrain, EventQueue};
use crate::wayland::output::{OutputUserData, WlOutput};
use crate::wayland::seat::SeatEvent;
use crate::wayland::surface::{
    SurfaceEvent, SurfaceManager, SurfaceRequests, SurfaceUserData, WlSurface,
};
use std::sync::Mutex;
use wayland_client::{GlobalManager, Proxy};
pub use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_shell_v1::RequestsTrait as LayerShellRequests,
    zwlr_layer_shell_v1::Layer,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    zwlr_layer_surface_v1::RequestsTrait as LayerSurfaceRequests,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1::{Anchor, Event},
};

/// The layer shell
pub struct LayerShell {
    surface_manager: SurfaceManager,
    layer_shell: Proxy<ZwlrLayerShellV1>,
}

impl LayerShell {
    /// Creates a `LayerShell`
    pub fn new(globals: &GlobalManager, surface_manager: SurfaceManager) -> Self {
        let layer_shell = globals
            .instantiate_auto(|layer_shell| {
                layer_shell.implement(
                    |event, _layer_shell| match event {
                    },
                    (),
                )
            })
            .expect("Server didn't advertise `zwlr_layer_shell_v1`");

        LayerShell {
            layer_shell,
            surface_manager,
        }
    }

    /// Creates a `LayerShellSurface`
    pub fn create_shell_surface(
        &self,
        output: Proxy<WlOutput>,
        layer: Layer,
        layout: Layout,
        app_id: String,
    ) -> LayerShellSurface {
        let (source, drain) = EventQueue::new();
        let surface = self.surface_manager.create_surface();
        let layer_surface = self
            .layer_shell
            .get_layer_surface(
                &surface,
                Some(&output),
                layer,
                app_id,
                |layer_surface| {
                    layer_surface.implement(move |event, layer_surface| match event {
                        Event::Closed => {
                            source.push_event(LayerSurfaceEvent::Close);
                        },
                        Event::Configure { serial, width, height } => {
                            layer_surface.ack_configure(serial);
                            let width = width as u32;
                            let height = height as u32;
                            let size = if width == 0 || height == 0 {
                                // if either w or h is zero, then we get to choose our size
                                None
                            } else {
                                Some((width, height))
                            };
                            source.push_event(LayerSurfaceEvent::Configure {
                                size,
                            });
                        },
                    }, ())
                },
            )
            .unwrap();
        layer_surface.set_anchor(layout.anchor());
        layer_surface.set_exclusive_zone(layout.exclusive());
        let size = layout.size(&output);
        layer_surface.set_size(size.0, size.1);
        surface.commit();
        LayerShellSurface {
            surface,
            layer_surface,
            layout,
            output,
            event_drain: drain,
        }
    }
}

/// A layer shell surface
pub struct LayerShellSurface {
    surface: Proxy<WlSurface>,
    layer_surface: Proxy<ZwlrLayerSurfaceV1>,
    layout: Layout,
    output: Proxy<WlOutput>,
    event_drain: EventDrain<LayerSurfaceEvent>,
}

impl LayerShellSurface {
    /// Returns the `wl_surface`
    pub fn surface(&self) -> &Proxy<WlSurface> {
        &self.surface
    }

    /// Returns the `zwlr_layer_surface_v1`
    pub fn xdg_surface(&self) -> &Proxy<ZwlrLayerSurfaceV1> {
        &self.layer_surface
    }

    /// The layout of the surface
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The output the surface is on
    pub fn output(&self) -> &Proxy<WlOutput> {
        &self.output
    }

    /// Polls the events from the event queue
    pub fn poll_events<F: FnMut(LayerSurfaceEvent, &LayerShellSurface)>(&self, mut cb: F) {
        {
            let surface_user_data = self
                .surface
                .user_data::<Mutex<SurfaceUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            surface_user_data.poll_events(|event, _user_data| match event {
                SurfaceEvent::Scale { scale_factor } => {
                    cb(LayerSurfaceEvent::Scale { scale_factor }, self);
                }
                SurfaceEvent::Seat { seat_id, event } => {
                    cb(LayerSurfaceEvent::Seat { seat_id, event }, self);
                }
            });
        }
        self.event_drain.poll_events(|event| {
            cb(event, self);
        });
    }
}

#[derive(Clone, Debug)]
/// Possible events generated by a shell surface that you need to handle
pub enum LayerSurfaceEvent {
    /// The surface scale factor has changed
    Scale {
        /// New scale factor
        scale_factor: u32,
    },
    /// A seat event was received
    Seat {
        /// Seat that sent the event
        seat_id: u32,
        /// The sent event
        event: SeatEvent,
    },
    /// The state of your window has been changed
    Configure {
        /// Optional new size for your shell surface
        ///
        /// This is the new size of the contents of your shell surface
        /// as suggested by the server. You can ignore it and choose
        /// a new size if you want better control on the possible
        /// sizes of your shell surface.
        ///
        /// In all cases, these events can be generated in large batches
        /// during an interactive resize, and you should buffer them before
        /// processing them. You only need to handle the last one of a batch.
        size: Option<(u32, u32)>,
    },
    /// A close request has been received
    ///
    /// Most likely the user has clicked on the close button of the decorations
    /// or something equivalent
    Close,
}

/// The desired layout of the surface
pub enum Layout {
    /// The surface will be anchored to the bottom of the screen
    BarBottom {
        /// The height of the bar
        height: u32
    },
}

impl Layout {
    fn anchor(&self) -> Anchor {
        match *self {
            Layout::BarBottom { .. } => {
                Anchor::Bottom |
                Anchor::Left |
                Anchor::Right
            }
        }
    }

    fn exclusive(&self) -> i32 {
        match *self {
            Layout::BarBottom { height } => height as _,
        }
    }

    fn size(
        &self,
        output: &Proxy<WlOutput>,
    ) -> (u32, u32) {
        let output_user_data = output
            .user_data::<Mutex<OutputUserData>>()
            .unwrap()
            .lock()
            .unwrap();
        let dimensions = output_user_data.modes.iter()
            .find(|mode| mode.is_current)
            .unwrap()
            .dimensions;
        match *self {
            Layout::BarBottom { height } => {
                (dimensions.0 as _, height)
            }
        }
    }
}
