use wayland_client::{Display, EventQueue as WlEventQueue,
                     GlobalEvent, GlobalManager, Proxy};
use crate::wayland::compositor::{initialize_compositor, initialize_subcompositor};
use crate::wayland::cursor::CursorManager;
use crate::wayland::event_queue::EventQueue;
use crate::wayland::output::{OutputManager, OutputManagerEvent};
use crate::wayland::seat::{SeatManager, SeatManagerEvent};
use crate::wayland::shm::{initialize_shm, WlShm};
use crate::wayland::surface::SurfaceManager;

pub struct Environment {
    pub display: Display,
    pub event_queue: WlEventQueue,
    pub globals: GlobalManager,
    pub output_manager: OutputManager,
    pub seat_manager: SeatManager,
    pub surface_manager: SurfaceManager,
    pub cursor_manager: CursorManager,
    pub shm: Proxy<WlShm>
}

impl Environment {
    pub fn initialize(theme_name: Option<String>) -> std::io::Result<Self> {
        let (display, mut event_queue) = Display::connect_to_env().unwrap();

        let (output_manager_source, output_manager_drain) = EventQueue::new();
        let (seat_manager_source, seat_manager_drain) = EventQueue::new();
        let (surface_manager_source, surface_manager_drain) = EventQueue::new();
        let (cursor_manager_source, cursor_manager_drain) = EventQueue::new();

        let globals = {
            GlobalManager::new_with_cb(&display, move |event, registry| {
                match event {
                    GlobalEvent::New { id, ref interface, version } => {
                        match &interface[..] {
                            "wl_output" => {
                                let event = OutputManagerEvent::NewOutput {
                                    id,
                                    version,
                                    registry,
                                };
                                output_manager_source.push_event(event);
                            },
                            "wl_seat" => {
                                let event = SeatManagerEvent::NewSeat {
                                    id,
                                    version,
                                    registry,
                                };
                                seat_manager_source.push_event(event);
                            },
                            _ => {},
                        }
                    }
                    GlobalEvent::Removed { id, ref interface } => {
                        match &interface[..] {
                            "wl_output" => {
                                let event = OutputManagerEvent::RemoveOutput {
                                    id
                                };
                                output_manager_source.push_event(event);
                            },
                            "wl_seat" => {
                                let event = SeatManagerEvent::RemoveSeat {
                                    id
                                };
                                seat_manager_source.push_event(event);
                            },
                            _ => {},
                        }
                    }
                }
            })
        };

        // double sync to retrieve the global list
        // and the globals metadata
        event_queue.sync_roundtrip()?;
        event_queue.sync_roundtrip()?;


        let compositor = initialize_compositor(&globals);
        let subcompositor = initialize_subcompositor(&globals);
        let shm = initialize_shm(&globals);

        let output_manager = OutputManager::new(
            output_manager_drain,
            surface_manager_source.clone(),
            cursor_manager_source.clone(),
        );
        let cursor_manager = CursorManager::new(
            cursor_manager_drain,
            output_manager.clone(),
            compositor.clone(),
            shm.clone(),
            theme_name,
        );
        let seat_manager = SeatManager::new(
            seat_manager_drain,
            cursor_manager.clone(),
        );
        let surface_manager = SurfaceManager::new(
            surface_manager_drain,
            compositor.clone(),
            subcompositor.clone(),
        );

        let mut environment = Environment {
            display,
            event_queue,
            globals,
            output_manager,
            seat_manager,
            surface_manager,
            cursor_manager,
            shm,
        };

        environment.output_manager.handle_events();
        environment.handle_events();

        Ok(environment)
    }

    pub fn handle_events(&mut self) {
        self.display.flush().unwrap();
        self.event_queue.dispatch().unwrap();
        self.output_manager.handle_events();
        self.cursor_manager.handle_events();
        self.seat_manager.handle_events();
        self.surface_manager.handle_events();
    }
}
