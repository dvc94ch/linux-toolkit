//! Wayland boilerplate handling
use crate::wayland::compositor::{initialize_compositor, initialize_subcompositor};
use crate::wayland::cursor::CursorManager;
use crate::wayland::data_device_manager::initialize_data_device_manager;
use crate::wayland::data_source::DataSourceManager;
use crate::wayland::event_queue::EventQueue;
use crate::wayland::output::{OutputManager, OutputManagerEvent};
use crate::wayland::seat::{SeatManager, SeatManagerEvent};
use crate::wayland::shm::{initialize_shm, WlShm};
use crate::wayland::surface::SurfaceManager;
use wayland_client::{Display, EventQueue as WlEventQueue, GlobalEvent, GlobalManager, Proxy};

/// The `Environment` ties together all the wayland boilerplate
pub struct Environment {
    /// The wayland display
    pub display: Display,
    /// The way
    pub event_queue: WlEventQueue,
    /// The underlying `GlobalManager` wrapping your registry
    pub globals: GlobalManager,
    /// A manager for handling the advertised outputs
    pub output_manager: OutputManager,
    /// A manager for handling the advertised seats
    pub seat_manager: SeatManager,
    /// A manager for handling surfaces
    pub surface_manager: SurfaceManager,
    /// A manager for handling cursors
    pub cursor_manager: CursorManager,
    /// The SHM global, to create shared memory buffers
    pub shm: Proxy<WlShm>,
    /// The data source manager used to handle drag&drop and selection
    pub data_source_manager: DataSourceManager,
}

impl Environment {
    /// Creates an `Environment`.
    ///
    /// Optionally takes the name of the cursor theme to load and otherwise
    /// uses the `libwayland-cursor` default.
    pub fn initialize(theme_name: Option<String>) -> std::io::Result<Self> {
        let (display, mut event_queue) = Display::connect_to_env().unwrap();

        let (output_manager_source, output_manager_drain) = EventQueue::new();
        let (seat_manager_source, seat_manager_drain) = EventQueue::new();
        let (surface_manager_source, surface_manager_drain) = EventQueue::new();
        let (cursor_manager_source, cursor_manager_drain) = EventQueue::new();

        let globals = {
            GlobalManager::new_with_cb(&display, move |event, registry| match event {
                GlobalEvent::New {
                    id,
                    ref interface,
                    version,
                } => match &interface[..] {
                    "wl_output" => {
                        let event = OutputManagerEvent::NewOutput {
                            id,
                            version,
                            registry,
                        };
                        output_manager_source.push_event(event);
                    }
                    "wl_seat" => {
                        let event = SeatManagerEvent::NewSeat {
                            id,
                            version,
                            registry,
                        };
                        seat_manager_source.push_event(event);
                    }
                    _ => {}
                },
                GlobalEvent::Removed { id, ref interface } => match &interface[..] {
                    "wl_output" => {
                        let event = OutputManagerEvent::RemoveOutput { id };
                        output_manager_source.push_event(event);
                    }
                    "wl_seat" => {
                        let event = SeatManagerEvent::RemoveSeat { id };
                        seat_manager_source.push_event(event);
                    }
                    _ => {}
                },
            })
        };

        // double sync to retrieve the global list
        // and the globals metadata
        event_queue.sync_roundtrip()?;
        event_queue.sync_roundtrip()?;

        let compositor = initialize_compositor(&globals);
        let subcompositor = initialize_subcompositor(&globals);
        let shm = initialize_shm(&globals);
        let data_device_manager = initialize_data_device_manager(&globals);

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
            data_device_manager.clone(),
        );
        let surface_manager = SurfaceManager::new(
            surface_manager_drain,
            compositor.clone(),
            subcompositor.clone(),
        );
        let data_source_manager = DataSourceManager::new(data_device_manager);

        let mut environment = Environment {
            display,
            event_queue,
            globals,
            output_manager,
            seat_manager,
            surface_manager,
            cursor_manager,
            shm,
            data_source_manager,
        };

        environment.output_manager.handle_events();
        environment.flush();
        environment.handle_events();
        environment.flush();
        environment.handle_events();

        Ok(environment)
    }

    /// Flush queued messages
    pub fn flush(&self) {
        self.display.flush().unwrap();
    }

    /// Handles sending and receiving queued wayland messages and all internal
    /// event processing. It should be called on every event loop.
    pub fn handle_events(&mut self) {
        self.event_queue.dispatch().unwrap();
        self.output_manager.handle_events();
        self.cursor_manager.handle_events();
        self.seat_manager.handle_events();
        self.surface_manager.handle_events();
    }
}
