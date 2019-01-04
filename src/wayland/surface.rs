use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
pub use wayland_client::protocol::wl_surface::WlSurface;
pub use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
pub use wayland_client::protocol::wl_subsurface::WlSubsurface;
pub use wayland_client::protocol::wl_subsurface::RequestsTrait as SubsurfaceRequests;
use wayland_client::protocol::wl_surface::Event;
use crate::wayland::compositor::{WlCompositor, CompositorRequests};
use crate::wayland::compositor::{WlSubcompositor, SubcompositorRequests};
use crate::wayland::event_queue::{EventQueue, EventSource, EventDrain};
use crate::wayland::keyboard::KeyboardEvent;
use crate::wayland::output::{WlOutput, OutputUserData};
use crate::wayland::pointer::PointerEvent;
use crate::wayland::touch::TouchEvent;

#[derive(Clone)]
pub struct SurfaceManager {
    event_drain: EventDrain<SurfaceManagerEvent>,
    compositor: Proxy<WlCompositor>,
    subcompositor: Proxy<WlSubcompositor>,
    surfaces: Arc<Mutex<Vec<Proxy<WlSurface>>>>,
}

impl SurfaceManager {
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

    pub fn create_surface(&self) -> Proxy<WlSurface> {
        let surface = self.compositor
            .create_surface(move |surface| {
                surface.implement(move |event, surface| {
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
                }, Mutex::new(SurfaceUserData::new()))
            }).unwrap();
        self.surfaces.lock().unwrap().push(surface.clone());
        surface
    }

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

    pub fn handle_events(&self) {
        let surfaces = self.surfaces.lock().unwrap();
        self.event_drain.poll_events(|event| {
            match event {
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
            }
        });
    }
}

pub struct SurfaceUserData {
    pub event_source: EventSource<SurfaceEvent>,
    event_drain: EventDrain<SurfaceEvent>,
    scale_factor: u32,
    outputs: Vec<Proxy<WlOutput>>,
}

impl SurfaceUserData {
    pub fn new() -> Self {
        let (source, drain) = EventQueue::new();
        SurfaceUserData {
            event_source: source,
            event_drain: drain,
            scale_factor: 1,
            outputs: Vec::new(),
        }
    }

    pub fn enter(&mut self, output: Proxy<WlOutput>) {
        self.outputs.push(output);
        self.update_scale_factor();
    }

    pub fn leave(&mut self, output: &Proxy<WlOutput>) {
        self.outputs.retain(|output2| !output.equals(output2));
        self.update_scale_factor();
    }

    pub fn update_scale_factor(&mut self) {
        let mut scale_factor = 1;
        for output in &self.outputs {
            let user_data = output
                .user_data::<Mutex<OutputUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            scale_factor = ::std::cmp::max(scale_factor, user_data.scale_factor);
        }
        if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;
            self.event_source.push_event(SurfaceEvent::Scale { scale_factor });
        }
    }

    pub fn poll_events<F: FnMut(SurfaceEvent, &SurfaceUserData)>(&self, mut cb: F) {
        self.event_drain.poll_events(|event| {
            cb(event, self);
        });
    }
}

#[derive(Clone)]
pub enum SurfaceManagerEvent {
    OutputScale { output: Proxy<WlOutput>, factor: u32 },
    OutputLeave { output: Proxy<WlOutput> },
}

#[derive(Clone)]
pub enum SurfaceEvent {
    Scale { scale_factor: u32 },
    Pointer { event: PointerEvent },
    Keyboard { event: KeyboardEvent },
    Touch { event: TouchEvent },
}
