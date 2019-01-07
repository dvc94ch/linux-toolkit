//! Surface handling
use crate::wayland::compositor::{CompositorRequests, WlCompositor};
use crate::wayland::compositor::{SubcompositorRequests, WlSubcompositor};
use crate::wayland::event_queue::{EventDrain, EventQueue, EventSource};
use crate::wayland::output::{OutputUserData, WlOutput};
use crate::wayland::seat::SeatEvent;
use std::sync::{Arc, Mutex};
pub use wayland_client::protocol::wl_subsurface::RequestsTrait as SubsurfaceRequests;
pub use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::Event;
pub use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
pub use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;

#[derive(Clone)]
/// Handles `wl_surface`s
pub struct SurfaceManager {
    event_drain: EventDrain<SurfaceManagerEvent>,
    compositor: Proxy<WlCompositor>,
    subcompositor: Proxy<WlSubcompositor>,
    surfaces: Arc<Mutex<Vec<Proxy<WlSurface>>>>,
}

impl SurfaceManager {
    /// Creates a new `SurfaceManager`
    pub fn new(
        event_drain: EventDrain<SurfaceManagerEvent>,
        compositor: Proxy<WlCompositor>,
        subcompositor: Proxy<WlSubcompositor>,
    ) -> Self {
        SurfaceManager {
            event_drain,
            compositor,
            subcompositor,
            surfaces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a new `wl_surface`
    pub fn create_surface(&self) -> Proxy<WlSurface> {
        let surface = self
            .compositor
            .create_surface(move |surface| {
                surface.implement(
                    move |event, surface| {
                        let mut user_data = surface
                            .user_data::<Mutex<SurfaceUserData>>()
                            .unwrap()
                            .lock()
                            .unwrap();
                        match event {
                            Event::Enter { output } => {
                                user_data.enter(output);
                            }
                            Event::Leave { output } => {
                                user_data.leave(&output);
                            }
                        }
                    },
                    Mutex::new(SurfaceUserData::new()),
                )
            })
            .unwrap();
        self.surfaces.lock().unwrap().push(surface.clone());
        surface
    }

    /// Creates a new `wl_subsurface`
    pub fn create_subsurface(
        &self,
        surface: &Proxy<WlSurface>,
        parent: &Proxy<WlSurface>,
    ) -> Proxy<WlSubsurface> {
        self.subcompositor
            .get_subsurface(surface, parent, |subsurface| {
                subsurface.implement(|event, _subsurface| match event {}, ())
            })
            .unwrap()
    }

    /// Processes it's event queue
    pub fn handle_events(&self) {
        let surfaces = self.surfaces.lock().unwrap();
        self.event_drain.poll_events(|event| match event {
            SurfaceManagerEvent::OutputLeave { output } => {
                for surface in &*surfaces {
                    surface
                        .user_data::<Mutex<SurfaceUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .leave(&output);
                }
            }
            SurfaceManagerEvent::OutputScale { .. } => {
                for surface in &*surfaces {
                    surface
                        .user_data::<Mutex<SurfaceUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .update_scale_factor();
                }
            }
        });
    }
}

/// The `wl_surface` user data
pub struct SurfaceUserData {
    pub(crate) event_source: EventSource<SurfaceEvent>,
    event_drain: EventDrain<SurfaceEvent>,
    scale_factor: u32,
    outputs: Vec<Proxy<WlOutput>>,
}

impl SurfaceUserData {
    /// Creates a new `SurfaceUserData`
    pub fn new() -> Self {
        let (source, drain) = EventQueue::new();
        SurfaceUserData {
            event_source: source,
            event_drain: drain,
            scale_factor: 1,
            outputs: Vec::new(),
        }
    }

    pub(crate) fn enter(&mut self, output: Proxy<WlOutput>) {
        self.outputs.push(output);
        self.update_scale_factor();
    }

    pub(crate) fn leave(&mut self, output: &Proxy<WlOutput>) {
        self.outputs.retain(|output2| !output.equals(output2));
        self.update_scale_factor();
    }

    pub(crate) fn update_scale_factor(&mut self) {
        let mut scale_factor = 1;
        for output in &self.outputs {
            let user_data = output
                .user_data::<Mutex<OutputUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            scale_factor =
                ::std::cmp::max(scale_factor, user_data.scale_factor);
        }
        if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;
            self.event_source
                .push_event(SurfaceEvent::Scale { scale_factor });
        }
    }

    /// Process it's event queue
    pub fn poll_events<F: FnMut(SurfaceEvent, &SurfaceUserData)>(
        &self,
        mut cb: F,
    ) {
        self.event_drain.poll_events(|event| {
            cb(event, self);
        });
    }
}

#[derive(Clone)]
/// Events the `SurfaceManager` needs to know about
pub enum SurfaceManagerEvent {
    /// Output scale factor changed
    OutputScale {
        /// The `wl_output`
        output: Proxy<WlOutput>,
        /// New scale factor
        factor: u32,
    },
    /// Output was disconnected
    OutputLeave {
        /// The `wl_output`
        output: Proxy<WlOutput>,
    },
}

#[derive(Clone)]
/// Possible events generated by a surface that you need to handle
pub enum SurfaceEvent {
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
}
