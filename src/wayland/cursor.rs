use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
use wayland_client::cursor;
use crate::wayland::compositor::{WlCompositor, CompositorRequests};
use crate::wayland::event_queue::EventDrain;
use crate::wayland::output::{WlOutput, OutputUserData, OutputManager};
use crate::wayland::pointer::{WlPointer, PointerRequests};
use crate::wayland::shm::WlShm;
use crate::wayland::surface::{WlSurface, SurfaceRequests};

pub struct CursorTheme {
    theme: cursor::CursorTheme,
    scale_factor: u32,
}

impl CursorTheme {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if theme_name is `None`.
    ///
    /// Returns `Err(())` if `libwayland-cursor` is not available.
    pub fn new(
        shm: &Proxy<WlShm>,
        name: Option<&String>,
        scale_factor: u32,
    ) -> Result<Self, ()> {
        if !cursor::is_available() {
            return Err(());
        }

        // No way to find the cursor size
        // Good cursor size for scale factors 1, 2 where determined
        // to be 16 and 48. A linear function is fitted to those points.
        // 32 * 1 - 16 = 16
        // 32 * 2 - 16 = 48
        let size = 32 * scale_factor - 16;

        let theme = cursor::load_theme(
            name.map(|s| &**s),
            size as u32,
            shm,
        );

        Ok(CursorTheme {
            theme,
            scale_factor,
        })
    }

    pub fn get_cursor(&self, cursor_name: &str) -> Option<cursor::Cursor> {
        self.theme.get_cursor(cursor_name)
    }

    pub fn scale_factor(&self) -> u32 {
        self.scale_factor
    }
}

#[derive(Clone)]
pub struct Cursor {
    surface: Proxy<WlSurface>,
    theme: Arc<Mutex<Option<CursorTheme>>>,
    cursor_name: String,
    enter_serial: u32,
    hx: i32,
    hy: i32,
}

impl Cursor {
    pub fn new(
        compositor: &Proxy<WlCompositor>,
        theme: Arc<Mutex<Option<CursorTheme>>>,
        cursor_name: Option<String>,
    ) -> Result<Self, ()> {
        let surface = compositor.create_surface(|surface| {
            surface.implement(|_, _| {}, ())
        }).unwrap();
        let cursor_name = cursor_name.unwrap_or_else(|| "left_ptr".into());
        let mut cursor = Cursor {
            surface,
            theme,
            cursor_name,
            enter_serial: 0,
            hx: 0,
            hy: 0,
        };
        cursor.load_cursor()?;
        Ok(cursor)
    }

    pub fn enter_surface(&mut self, pointer: &Proxy<WlPointer>, serial: u32) {
        self.enter_serial = serial;
        self.set_cursor(pointer);
    }

    pub fn change_cursor(
        &mut self,
        pointer: &Proxy<WlPointer>,
        cursor_name: Option<String>,
    ) -> Result<(), ()> {
        self.cursor_name = cursor_name.unwrap_or_else(|| "left_ptr".into());
        self.load_cursor()?;
        self.set_cursor(pointer);
        Ok(())
    }

    fn load_cursor(&mut self) -> Result<(), ()> {
        let theme = self.theme.lock().unwrap();
        if theme.is_none() {
            return Err(())
        }
        let theme_ref = theme.as_ref().unwrap();
        let cursor = theme_ref
            .get_cursor(&self.cursor_name)
            .ok_or(())?;
        let buffer = cursor.frame_buffer(0).ok_or(())?;
        let (hx, hy) = cursor
            .frame_info(0)
            .map(|(_w, _h, hx, hy, _)| (hx as i32, hy as i32))
            .unwrap_or((0, 0));
        self.hx = hx;
        self.hy = hy;

        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.set_buffer_scale(theme_ref.scale_factor() as i32);
        self.surface.commit();
        Ok(())
    }

    fn set_cursor(&self, pointer: &Proxy<WlPointer>) {
        pointer.set_cursor(
            self.enter_serial,
            Some(&self.surface),
            self.hx,
            self.hy,
        )
    }
}

#[derive(Clone)]
pub struct CursorManager {
    cursors: Arc<Mutex<Vec<Cursor>>>,
    event_drain: EventDrain<CursorManagerEvent>,
    theme: Arc<Mutex<Option<CursorTheme>>>,
    theme_name: Option<String>,
    scale_factor: u32,
    output_manager: OutputManager,
    compositor: Proxy<WlCompositor>,
    shm: Proxy<WlShm>,
}

impl CursorManager {
    pub fn new(
        event_drain: EventDrain<CursorManagerEvent>,
        output_manager: OutputManager,
        compositor: Proxy<WlCompositor>,
        shm: Proxy<WlShm>,
        theme_name: Option<String>,
    ) -> Self {
        CursorManager {
            cursors: Arc::new(Mutex::new(Vec::new())),
            event_drain,
            theme: Arc::new(Mutex::new(None)),
            theme_name,
            scale_factor: 1,
            output_manager,
            compositor,
            shm,
        }
    }

    pub fn get_cursor(
        &self,
        cursor_name: Option<String>,
    ) -> Cursor {
        let cursor = Cursor::new(
            &self.compositor,
            self.theme.clone(),
            cursor_name,
        ).unwrap();
        let mut cursors = self.cursors.lock().unwrap();
        cursors.push(cursor.clone());
        cursor
    }

    pub fn handle_events(&mut self) {
        let mut update_scale_factor = false;
        self.event_drain.poll_events(|event| match event {
            CursorManagerEvent::OutputScale { output: _, factor: _ } |
            CursorManagerEvent::OutputLeave { output: _ } => {
                update_scale_factor = true;
            }
        });
        let new_scale_factor = self.output_manager
            .outputs()
            .lock()
            .unwrap()
            .iter()
            .map(|output| {
                output
                    .user_data::<Mutex<OutputUserData>>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .scale_factor
            })
            .max()
            .unwrap_or(1);
        if new_scale_factor != self.scale_factor {
            self.scale_factor = new_scale_factor;
            let mut theme = self.theme.lock().unwrap();
            *theme = CursorTheme::new(
                &self.shm,
                self.theme_name.as_ref(),
                self.scale_factor,
            ).ok();
        }
    }
}

#[derive(Clone)]
pub enum CursorManagerEvent {
    OutputScale { output: Proxy<WlOutput>, factor: u32 },
    OutputLeave { output: Proxy<WlOutput> },
}
