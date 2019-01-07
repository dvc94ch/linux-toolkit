//! Handles cursor theme loading and changing the cursor icon.
use crate::wayland::compositor::{CompositorRequests, WlCompositor};
use crate::wayland::event_queue::EventDrain;
use crate::wayland::output::{OutputManager, OutputUserData, WlOutput};
use crate::wayland::pointer::{PointerRequests, WlPointer};
use crate::wayland::shm::WlShm;
use crate::wayland::surface::{SurfaceRequests, WlSurface};
use std::sync::{Arc, Mutex};
use wayland_client::cursor;
use wayland_client::Proxy;

/// A scale factor aware cursor theme
struct CursorTheme {
    /// The `libwayland-cursor` theme
    theme: cursor::CursorTheme,
    /// The scale factor used to load the cursor theme
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

        let theme = cursor::load_theme(name.map(|s| &**s), size as u32, shm);

        Ok(CursorTheme {
            theme,
            scale_factor,
        })
    }

    /// Returns the cursor called `cursor_name` if it exists.
    pub fn get_cursor(&self, cursor_name: &str) -> Option<cursor::Cursor> {
        self.theme.get_cursor(cursor_name)
    }

    /// Returns the scale factor that the theme was loaded with.
    pub fn scale_factor(&self) -> u32 {
        self.scale_factor
    }
}

struct CursorInner {
    pointer: Option<Proxy<WlPointer>>,
    surface: Proxy<WlSurface>,
    theme: Arc<Mutex<Option<CursorTheme>>>,
    cursor_name: String,
    enter_serial: u32,
    hx: i32,
    hy: i32,
}

impl CursorInner {
    fn new(
        compositor: &Proxy<WlCompositor>,
        theme: Arc<Mutex<Option<CursorTheme>>>,
        cursor_name: Option<String>,
    ) -> Result<Self, ()> {
        let surface = compositor
            .create_surface(|surface| surface.implement(|_, _| {}, ()))
            .unwrap();
        let cursor_name = cursor_name.unwrap_or_else(|| "left_ptr".into());
        let mut cursor = CursorInner {
            pointer: None,
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

    fn enter_surface(&mut self, pointer: Proxy<WlPointer>, serial: u32) {
        self.enter_serial = serial;
        self.pointer = Some(pointer);
        self.set_cursor();
    }

    fn change_cursor(&mut self, cursor_name: Option<String>) -> Result<(), ()> {
        let new_cursor_name = cursor_name.unwrap_or_else(|| "left_ptr".into());
        if self.cursor_name != new_cursor_name {
            self.cursor_name = new_cursor_name;
            self.load_cursor()?;
        }
        self.set_cursor();
        Ok(())
    }

    fn load_cursor(&mut self) -> Result<(), ()> {
        let theme = self.theme.lock().unwrap();
        if theme.is_none() {
            return Err(());
        }
        let theme_ref = theme.as_ref().unwrap();
        let cursor = theme_ref.get_cursor(&self.cursor_name).ok_or(())?;
        let buffer = cursor.frame_buffer(0).ok_or(())?;
        let (w, h, hx, hy) = cursor
            .frame_info(0)
            .map(|(w, h, hx, hy, _)| (w as i32, h as i32, hx as i32, hy as i32))
            .unwrap_or((0, 0, 0, 0));
        self.hx = hx;
        self.hy = hy;

        self.surface.attach(Some(&buffer), 0, 0);
        self.surface
            .set_buffer_scale(theme_ref.scale_factor() as i32);
        if self.surface.version() >= 4 {
            self.surface.damage_buffer(0, 0, w, h);
        } else {
            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            self.surface.damage(0, 0, w, h);
        }
        self.surface.commit();
        Ok(())
    }

    fn set_cursor(&self) {
        self.pointer.as_ref().unwrap().set_cursor(
            self.enter_serial,
            Some(&self.surface),
            self.hx,
            self.hy,
        )
    }
}

/// A cloneable cursor
#[derive(Clone)]
pub struct Cursor {
    /// The internal cursor
    inner: Arc<Mutex<CursorInner>>,
}

impl Cursor {
    /// Creates a new `Cursor`
    fn new(
        compositor: &Proxy<WlCompositor>,
        theme: Arc<Mutex<Option<CursorTheme>>>,
        cursor_name: Option<String>,
    ) -> Result<Self, ()> {
        let inner = CursorInner::new(compositor, theme, cursor_name)?;
        Ok(Cursor {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    /// Called when a `wl_pointer` enters a surface. After entering
    /// a surface the cursor's `wl_surface` needs to be sent to the
    /// compositor. You need to call `set_cursor` or `change_cursor`.
    pub fn enter_surface(&self, pointer: Proxy<WlPointer>, serial: u32) {
        let mut cursor = self.inner.lock().unwrap();
        cursor.enter_surface(pointer, serial);
    }

    /// Changes the cursor to `cursor_name` and sets the cursor surface.
    pub fn change_cursor(&self, cursor_name: Option<String>) -> Result<(), ()> {
        let mut cursor = self.inner.lock().unwrap();
        cursor.change_cursor(cursor_name)
    }

    /// Sets the cursor surface.
    pub fn set_cursor(&self) {
        let cursor = self.inner.lock().unwrap();
        cursor.set_cursor();
    }

    fn load_cursor(&self) -> Result<(), ()> {
        let mut cursor = self.inner.lock().unwrap();
        cursor.load_cursor()
    }
}

impl PartialEq for Cursor {
    fn eq(&self, other: &Cursor) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl std::fmt::Debug for Cursor {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter,
    ) -> Result<(), std::fmt::Error> {
        write!(fmt, "Cursor")
    }
}

/// The `CursorManager` reloads the `CursorTheme` when a `wl_output` is removed
/// or a scale factor is changed.
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
    /// Creates a new `CursorManager`
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

    /// Creates a new `Cursor`
    pub fn new_cursor(&self, cursor_name: Option<String>) -> Cursor {
        let cursor =
            Cursor::new(&self.compositor, self.theme.clone(), cursor_name)
                .unwrap();
        let mut cursors = self.cursors.lock().unwrap();
        cursors.push(cursor.clone());
        cursor
    }

    /// Removes a `Cursor` when it is no longer needed
    pub fn remove_cursor(&self, cursor: &Cursor) {
        let mut cursors = self.cursors.lock().unwrap();
        cursors.retain(|cursor2| cursor != cursor2);
    }

    /// Returns all cursors
    pub fn cursors(&self) -> &Arc<Mutex<Vec<Cursor>>> {
        &self.cursors
    }

    /// Processes it's event queues and reloads themes and cursors
    /// when necessary
    pub fn handle_events(&mut self) {
        let mut update_scale_factor = false;
        self.event_drain.poll_events(|event| match event {
            CursorManagerEvent::OutputScale {
                output: _,
                factor: _,
            }
            | CursorManagerEvent::OutputLeave { output: _ } => {
                update_scale_factor = true;
            }
        });
        let new_scale_factor = self
            .output_manager
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
            )
            .ok();
            let mut cursors = self.cursors.lock().unwrap();
            for cursor in cursors.iter_mut() {
                cursor.load_cursor().unwrap();
            }
        }
    }
}

/// The events that a `CursorManager` needs to know about
#[derive(Clone)]
pub enum CursorManagerEvent {
    /// The scale factor of an output was changed
    OutputScale {
        /// The `wl_output`
        output: Proxy<WlOutput>,
        /// The new scale factor
        factor: u32,
    },
    /// An output was disconnected
    OutputLeave {
        /// The `wl_output`
        output: Proxy<WlOutput>,
    },
}
