//! Handles the `zwlr_foreign_toplevel_v1` protocol.
#![allow(missing_docs)]
use crate::wayland::event_queue::{EventDrain, EventQueue};
use std::sync::{Arc, Mutex};
use wayland_client::{GlobalManager, Proxy};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::Event,
    zwlr_foreign_toplevel_manager_v1::Event as ManagerEvent,
};
pub use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};

/// The toplevel manager
#[derive(Clone)]
pub struct ToplevelManager {
    manager: Proxy<ZwlrForeignToplevelManagerV1>,
    toplevels: Arc<Mutex<Vec<Proxy<ZwlrForeignToplevelHandleV1>>>>,
    event_drain: EventDrain<ToplevelEvent>,
}

impl ToplevelManager {
    /// Creates a new `ToplevelManager`
    pub fn new(globals: &GlobalManager) -> Result<Self, ()> {
        let (source, drain) = EventQueue::new();
        let toplevels = Arc::new(Mutex::new(Vec::new()));
        let toplevels2 = toplevels.clone();
        let manager = globals
            .instantiate_auto(move |manager| {
                let toplevels = toplevels2.clone();
                manager.implement(
                    move |event, _manager| match event {
                        ManagerEvent::Toplevel { toplevel } => {
                            let handle = {
                                let source = source.clone();
                                let toplevels = toplevels.clone();
                                toplevel.implement(
                                    move |event, handle| match event {
                                        Event::Title { title } => {
                                            let mut user_data = handle
                                                .user_data::<Mutex<ToplevelUserData>>()
                                                .unwrap()
                                                .lock()
                                                .unwrap();
                                            user_data.title = title;
                                        }
                                        Event::AppId { app_id } => {
                                            let mut user_data = handle
                                                .user_data::<Mutex<ToplevelUserData>>()
                                                .unwrap()
                                                .lock()
                                                .unwrap();
                                            user_data.app_id = app_id;
                                        }
                                        Event::State { state } => {
                                            let mut user_data = handle
                                                .user_data::<Mutex<ToplevelUserData>>()
                                                .unwrap()
                                                .lock()
                                                .unwrap();
                                            user_data.state = state;
                                        }
                                        Event::Done => {
                                            source.push_event(ToplevelEvent::Done);
                                        }
                                        Event::Closed => {
                                            let mut toplevels = toplevels.lock().unwrap();
                                            toplevels.retain(|toplevel| !handle.equals(toplevel));
                                        }
                                        _ => {}
                                    },
                                    Mutex::new(ToplevelUserData::new()),
                                )
                            };
                            let mut toplevels = toplevels.lock().unwrap();
                            toplevels.push(handle);
                        }
                        _ => {}
                    },
                    (),
                )
            })
            .map_err(|_| ())?;
        Ok(ToplevelManager {
            manager,
            toplevels,
            event_drain: drain,
        })
    }
}

#[derive(Clone, Debug)]
pub enum ToplevelEvent {
    Done,
}

pub struct ToplevelUserData {
    title: String,
    app_id: String,
    state: Vec<u8>,
}

impl ToplevelUserData {
    pub fn new() -> Self {
        ToplevelUserData {
            title: String::new(),
            app_id: String::new(),
            state: Vec::new(),
        }
    }

    pub fn app_id(&self) -> &String {
        &self.app_id
    }

    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn state(&self) -> &Vec<u8> {
        &self.state
    }
}
