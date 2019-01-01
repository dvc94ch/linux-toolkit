use wayland_client::{Display, EventQueue as WlEventQueue,
                     GlobalEvent, GlobalManager, Proxy};
use crate::wayland::event_queue::EventQueue;
use crate::wayland::output::OutputManager;
use crate::wayland::seat::SeatManager;
use crate::wayland::shm::{initialize_shm, WlShm};
use crate::wayland::surface::{SurfaceManager, SurfaceManagerEvent as SmEvent};

pub struct Environment {
    pub display: Display,
    pub event_queue: WlEventQueue,
    pub globals: GlobalManager,
    pub output_manager: OutputManager,
    pub seat_manager: SeatManager,
    pub surface_manager: SurfaceManager,
    pub shm: Proxy<WlShm>
}

impl Environment {
    pub fn initialize() -> std::io::Result<Self> {
        let (display, mut event_queue) = Display::connect_to_env().unwrap();

        let (sm_source, sm_drain) = EventQueue::new();
        let output_manager = OutputManager::new(sm_source.clone());
        let seat_manager = SeatManager::new(sm_source.clone());

        let globals = {
            let output_manager = output_manager.clone();
            let seat_manager = seat_manager.clone();

            GlobalManager::new_with_cb(&display, move |event, registry| {
                match event {
                    GlobalEvent::New { id, ref interface, version } => {
                        match &interface[..] {
                            "wl_output" => {
                                output_manager.new_output(id, version, &registry);
                            },
                            "wl_seat" => {
                                seat_manager.new_seat(id, version, &registry);
                            },
                            _ => {},
                        }
                    }
                    GlobalEvent::Removed { id, ref interface } => {
                        match &interface[..] {
                            "wl_output" => {
                                let output = output_manager.get_output(id)
                                    .unwrap();
                                let event = SmEvent::OutputLeave { output };
                                sm_source.push_event(event);
                                output_manager.remove_output(id);
                            },
                            "wl_seat" => {
                                seat_manager.remove_seat(id);
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

        let surface_manager = SurfaceManager::new(&globals, sm_drain);
        let shm = initialize_shm(&globals);

        // sync to retrieve the global events
        event_queue.sync_roundtrip()?;

        Ok(Environment {
            display,
            event_queue,
            globals,
            output_manager,
            seat_manager,
            surface_manager,
            shm,
        })
    }

    pub fn handle_events(&mut self) {
        self.surface_manager.handle_events();
        self.display.flush().unwrap();
        self.event_queue.dispatch().unwrap();
    }
}
