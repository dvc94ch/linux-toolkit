use std::sync::Mutex;
use wayland_client::{GlobalManager, Proxy};
use wayland_protocols::xdg_shell::client::{
    xdg_wm_base::XdgWmBase,
    xdg_wm_base::RequestsTrait as XdgShellRequests,
    xdg_wm_base::Event as XdgShellEvent,
    xdg_surface::XdgSurface,
    xdg_surface::RequestsTrait as XdgSurfaceRequests,
    xdg_surface::Event as XdgSurfaceEvent_,
    xdg_toplevel::XdgToplevel,
    xdg_toplevel::RequestsTrait as XdgToplevelRequests,
    xdg_toplevel::Event as XdgToplevelEvent,
    xdg_toplevel::State,
};
use crate::wayland::event_queue::{EventQueue, EventDrain};
use crate::wayland::keyboard::KeyboardEvent;
use crate::wayland::pointer::PointerEvent;
use crate::wayland::surface::{WlSurface, SurfaceRequests,
                              SurfaceManager, SurfaceEvent, SurfaceUserData};
use crate::wayland::touch::TouchEvent;

pub struct XdgShell {
    surface_manager: SurfaceManager,
    xdg_shell: Proxy<XdgWmBase>,
}

impl XdgShell {
    pub fn new(globals: &GlobalManager, surface_manager: SurfaceManager) -> Self {
        let xdg_shell: Proxy<XdgWmBase> = globals
            .instantiate_auto(|wm_base| {
                wm_base.implement(|event, wmbase| match event {
                    XdgShellEvent::Ping { serial } => {
                        wmbase.pong(serial)
                    }
                }, ())
            })
            .expect("Server didn't advertise `xdg_wm_base`");

        XdgShell {
            xdg_shell,
            surface_manager,
        }
    }

    pub fn create_shell_surface(&self) -> XdgShellSurface {
        let (source, drain) = EventQueue::new();
        let surface = self.surface_manager.create_surface();
        let xdg_surface = self.xdg_shell
            .get_xdg_surface(&surface, |xdg_surface| {
                xdg_surface.implement(|event, xdg_surface| match event {
                    XdgSurfaceEvent_::Configure { serial } => {
                        xdg_surface.ack_configure(serial);
                    }
                }, ())
            }).unwrap();
        let xdg_toplevel = xdg_surface
            .get_toplevel(|xdg_toplevel| {
                xdg_toplevel.implement(move |event, _xdg_toplevel| match event {
                    XdgToplevelEvent::Close => {
                        source.push_event(XdgSurfaceEvent::Close);
                    }
                    XdgToplevelEvent::Configure { width, height, states } => {
                        let width = width as u32;
                        let height = height as u32;
                        let size = if width == 0 || height == 0 {
                            // if either w or h is zero, then we get to choose our size
                            None
                        } else {
                            Some((width, height))
                        };
                        let view: &[u32] = unsafe {
                            ::std::slice::from_raw_parts(
                                states.as_ptr() as *const _,
                                states.len() / 4,
                            )
                        };
                        let states = view
                            .iter()
                            .cloned()
                            .flat_map(State::from_raw)
                            .collect::<Vec<_>>();
                        source.push_event(XdgSurfaceEvent::Configure {
                            size,
                            states,
                        });
                    }
                }, ())
            }).unwrap();
        surface.commit();
        XdgShellSurface {
            surface,
            xdg_surface,
            xdg_toplevel,
            event_drain: drain,
        }
    }
}

pub struct XdgShellSurface {
    surface: Proxy<WlSurface>,
    xdg_surface: Proxy<XdgSurface>,
    xdg_toplevel: Proxy<XdgToplevel>,
    event_drain: EventDrain<XdgSurfaceEvent>,
}

impl XdgShellSurface {
    pub fn surface(&self) -> &Proxy<WlSurface> {
        &self.surface
    }

    pub fn xdg_surface(&self) -> &Proxy<XdgSurface> {
        &self.xdg_surface
    }

    pub fn xdg_toplevel(&self) -> &Proxy<XdgToplevel> {
        &self.xdg_toplevel
    }

    pub fn poll_events<F: FnMut(XdgSurfaceEvent, &XdgShellSurface)>(&self, mut cb: F) {
        {
            let surface_user_data = self.surface
            .user_data::<Mutex<SurfaceUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            surface_user_data.poll_events(|event, _user_data| {
                match event {
                    SurfaceEvent::Scale { scale_factor } => {
                        cb(XdgSurfaceEvent::Scale { scale_factor }, self);
                    }
                    SurfaceEvent::Pointer { event } => {
                        cb(XdgSurfaceEvent::Pointer { event }, self);
                    }
                    SurfaceEvent::Keyboard { event } => {
                        cb(XdgSurfaceEvent::Keyboard { event }, self);
                    }
                    SurfaceEvent::Touch { event } => {
                        cb(XdgSurfaceEvent::Touch { event }, self);
                    }
                }
            });
        }
        self.event_drain.poll_events(|event| {
            cb(event, self);
        });
    }
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        self.xdg_toplevel.destroy();
        self.xdg_surface.destroy();
        self.surface.destroy();
    }
}

#[derive(Clone)]
pub enum XdgSurfaceEvent {
    Scale {
        scale_factor: u32,
    },
    Configure {
        size: Option<(u32, u32)>,
        states: Vec<State>,
    },
    Close,
    Pointer { event: PointerEvent },
    Keyboard { event: KeyboardEvent },
    Touch { event: TouchEvent },
}
