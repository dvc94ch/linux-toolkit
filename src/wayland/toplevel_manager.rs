//! Handles the `zwlr_foreign_toplevel_v1` protocol.
use crate::wayland::event_queue::{EventDrain, EventQueue};
use std::sync::{Arc, Mutex};
use wayland_client::{GlobalManager, Proxy};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::Event,
    zwlr_foreign_toplevel_manager_v1::Event as ManagerEvent,
};
pub use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{State, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};

/// The toplevel manager
#[derive(Clone)]
pub struct ToplevelManager {
    manager: Proxy<ZwlrForeignToplevelManagerV1>,
    toplevels: Arc<Mutex<Vec<Toplevel>>>,
}

impl ToplevelManager {
    /// Creates a new `ToplevelManager`
    pub fn new(globals: &GlobalManager) -> Result<Self, ()> {
        let toplevels = Arc::new(Mutex::new(Vec::<Toplevel>::new()));
        let manager =
            {
                let toplevels = toplevels.clone();
                globals
                    .instantiate_auto(move |manager| {
                        manager.implement(move |event, _manager| match event {
                    ManagerEvent::Toplevel { toplevel } => {
                        let (source, drain) = EventQueue::new();
                        let toplevel = toplevel.implement(move |event, handle| {
                            match event {
                                Event::Title { title } => {
                                    let mut user_data = handle
                                        .user_data::<Mutex<ToplevelUserData>>()
                                        .unwrap()
                                        .lock()
                                        .unwrap();
                                    user_data.title = title;
                                },
                                Event::AppId { app_id } => {
                                    let mut user_data = handle
                                        .user_data::<Mutex<ToplevelUserData>>()
                                        .unwrap()
                                        .lock()
                                        .unwrap();
                                    user_data.app_id = app_id;
                                },
                                Event::State { state: states } => {
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
                                    let mut user_data = handle
                                        .user_data::<Mutex<ToplevelUserData>>()
                                        .unwrap()
                                        .lock()
                                        .unwrap();
                                    user_data.states = states;
                                },
                                Event::Done => {
                                    source.push_event(ToplevelEvent::Configure);
                                },
                                Event::Closed => {
                                    source.push_event(ToplevelEvent::Closed);
                                    let mut user_data = handle
                                        .user_data::<Mutex<ToplevelUserData>>()
                                        .unwrap()
                                        .lock()
                                        .unwrap();
                                    user_data.closed = true;
                                },
                                _ => {},
                            }
                        }, Mutex::new(ToplevelUserData::new()));
                        let mut toplevels = toplevels.lock().unwrap();
                        toplevels.push(Toplevel::new(toplevel, drain));
                    },
                    _ => {},
                }, ())
                    })
                    .map_err(|_| ())?
            };
        Ok(ToplevelManager { manager, toplevels })
    }

    /// A list of all current toplevels
    pub fn toplevels(&self) -> Vec<Toplevel> {
        self.toplevels
            .lock()
            .unwrap()
            .iter()
            .filter(|toplevel| !toplevel.closed())
            .map(|toplevel| toplevel.clone())
            .collect()
    }

    /// The `zwlr_foreign_toplevel_handle_v1` with `toplevel_id`
    pub fn get_toplevel(&self, toplevel_id: u32) -> Option<Toplevel> {
        self.toplevels
            .lock()
            .unwrap()
            .iter()
            .find(|toplevel| toplevel.id() == toplevel_id)
            .map(|toplevel| toplevel.clone())
    }

    /// Process it's event queue
    pub fn poll_events<F: FnMut(ToplevelEvent, Toplevel)>(
        &self,
        mut handler: F,
    ) {
        let mut toplevels = self.toplevels.lock().unwrap();
        toplevels.retain(|toplevel| {
            toplevel.poll_events(|event| handler(event, toplevel.clone()));
            !toplevel.closed()
        });
    }
}

#[derive(Clone, Debug)]
/// Toplevel events
pub enum ToplevelEvent {
    /// Toplevel was configured
    Configure,
    /// Toplevel was closed
    Closed,
}

struct ToplevelUserData {
    title: String,
    app_id: String,
    states: Vec<State>,
    closed: bool,
}

impl ToplevelUserData {
    fn new() -> Self {
        ToplevelUserData {
            title: String::new(),
            app_id: String::new(),
            states: Vec::new(),
            closed: false,
        }
    }
}

#[derive(Clone)]
/// Toplevel wrapps the proxy to a toplevel handle
/// and it's event drain.
pub struct Toplevel {
    proxy: Proxy<ZwlrForeignToplevelHandleV1>,
    drain: EventDrain<ToplevelEvent>,
}

impl Toplevel {
    /// Creates a new `Toplevel`
    pub fn new(
        proxy: Proxy<ZwlrForeignToplevelHandleV1>,
        drain: EventDrain<ToplevelEvent>,
    ) -> Self {
        Toplevel { proxy, drain }
    }

    /// The proxy
    pub fn raw(&self) -> &Proxy<ZwlrForeignToplevelHandleV1> {
        &self.proxy
    }

    /// The id
    pub fn id(&self) -> u32 {
        self.proxy.id()
    }

    /// Poll it's event queue
    pub fn poll_events<F: FnMut(ToplevelEvent)>(&self, handler: F) {
        self.drain.poll_events(handler);
    }

    /// It's app_id
    pub fn app_id(&self) -> String {
        self.proxy
            .user_data::<Mutex<ToplevelUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .app_id
            .to_owned()
    }

    /// It's title
    pub fn title(&self) -> String {
        self.proxy
            .user_data::<Mutex<ToplevelUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .title
            .to_owned()
    }

    /// It's states
    pub fn states(&self) -> Vec<State> {
        self.proxy
            .user_data::<Mutex<ToplevelUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .states
            .to_owned()
    }

    fn closed(&self) -> bool {
        self.proxy
            .user_data::<Mutex<ToplevelUserData>>()
            .unwrap()
            .lock()
            .unwrap()
            .closed
    }
}

impl PartialEq for Toplevel {
    fn eq(&self, other: &Toplevel) -> bool {
        self.proxy.equals(&other.proxy)
    }
}
