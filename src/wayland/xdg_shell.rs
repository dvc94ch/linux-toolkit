//! Handles the `xdg_wm_base` protocol.
use crate::wayland::event_queue::{EventDrain, EventQueue};
use crate::wayland::seat::SeatEvent;
use crate::wayland::surface::{
    SurfaceEvent, SurfaceManager, SurfaceRequests, SurfaceUserData, WlSurface,
};
use std::sync::Mutex;
use wayland_client::{GlobalManager, Proxy};
use wayland_protocols::xdg_shell::client::{
    xdg_surface::Event as XdgSurfaceEvent_,
    xdg_surface::RequestsTrait as XdgSurfaceRequests, xdg_surface::XdgSurface,
    xdg_toplevel::Event as XdgToplevelEvent,
    xdg_toplevel::RequestsTrait as XdgToplevelRequests, xdg_toplevel::State,
    xdg_toplevel::XdgToplevel, xdg_wm_base::Event as XdgShellEvent,
    xdg_wm_base::RequestsTrait as XdgShellRequests, xdg_wm_base::XdgWmBase,
};

/// The xdg shell
pub struct XdgShell {
    surface_manager: SurfaceManager,
    xdg_shell: Proxy<XdgWmBase>,
}

impl XdgShell {
    /// Creates a `XdgShell`
    pub fn new(
        globals: &GlobalManager,
        surface_manager: SurfaceManager,
    ) -> Self {
        let xdg_shell = globals
            .instantiate_auto(|wm_base| {
                wm_base.implement(
                    |event, wmbase| match event {
                        XdgShellEvent::Ping { serial } => wmbase.pong(serial),
                    },
                    (),
                )
            })
            .expect("Server didn't advertise `xdg_wm_base`");

        XdgShell {
            xdg_shell,
            surface_manager,
        }
    }

    /// Creates a `XdgShellSurface`
    pub fn create_shell_surface(&self) -> XdgShellSurface {
        let (source, drain) = EventQueue::new();
        let surface = self.surface_manager.create_surface();
        let xdg_surface = self
            .xdg_shell
            .get_xdg_surface(&surface, |xdg_surface| {
                xdg_surface.implement(
                    |event, xdg_surface| match event {
                        XdgSurfaceEvent_::Configure { serial } => {
                            xdg_surface.ack_configure(serial);
                        }
                    },
                    (),
                )
            })
            .unwrap();
        let xdg_toplevel = xdg_surface
            .get_toplevel(|xdg_toplevel| {
                xdg_toplevel.implement(
                    move |event, _xdg_toplevel| match event {
                        XdgToplevelEvent::Close => {
                            source.push_event(XdgSurfaceEvent::Close);
                        }
                        XdgToplevelEvent::Configure {
                            width,
                            height,
                            states,
                        } => {
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
                    },
                    (),
                )
            })
            .unwrap();
        surface.commit();
        XdgShellSurface {
            surface,
            xdg_surface,
            xdg_toplevel,
            event_drain: drain,
        }
    }
}

/// A xdg shell surface
pub struct XdgShellSurface {
    surface: Proxy<WlSurface>,
    xdg_surface: Proxy<XdgSurface>,
    xdg_toplevel: Proxy<XdgToplevel>,
    event_drain: EventDrain<XdgSurfaceEvent>,
}

impl XdgShellSurface {
    /// Returns the `wl_surface`
    pub fn surface(&self) -> &Proxy<WlSurface> {
        &self.surface
    }

    /// Returns the `xdg_surface`
    pub fn xdg_surface(&self) -> &Proxy<XdgSurface> {
        &self.xdg_surface
    }

    /// Returns the `xdg_toplevel`
    pub fn xdg_toplevel(&self) -> &Proxy<XdgToplevel> {
        &self.xdg_toplevel
    }

    /// Polls the events from the event queue
    pub fn poll_events<F: FnMut(XdgSurfaceEvent, &XdgShellSurface)>(
        &self,
        mut cb: F,
    ) {
        {
            let surface_user_data = self
                .surface
                .user_data::<Mutex<SurfaceUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            surface_user_data.poll_events(|event, _user_data| match event {
                SurfaceEvent::Scale { scale_factor } => {
                    cb(XdgSurfaceEvent::Scale { scale_factor }, self);
                }
                SurfaceEvent::Seat { seat_id, event } => {
                    cb(XdgSurfaceEvent::Seat { seat_id, event }, self);
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

#[derive(Clone, Debug)]
/// Possible events generated by a shell surface that you need to handle
pub enum XdgSurfaceEvent {
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
        /// New combination of states of your window
        ///
        /// Typically tells you if your surface is active/inactive, maximized,
        /// etc...
        states: Vec<State>,
    },
    /// A close request has been received
    ///
    /// Most likely the user has clicked on the close button of the decorations
    /// or something equivalent
    Close,
}
