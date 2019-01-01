use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_registry::RequestsTrait as RegistryRequests;
pub use wayland_client::protocol::wl_output::WlOutput;
pub use wayland_client::protocol::wl_output::RequestsTrait as OutputRequests;
pub use wayland_client::protocol::wl_output::{Subpixel, Transform};
use wayland_client::protocol::wl_output::{Event, Mode as WlMode};
use crate::wayland::event_queue::{EventSource};
use crate::wayland::surface::SurfaceManagerEvent;

#[derive(Clone)]
pub struct OutputManager {
    sm_source: EventSource<SurfaceManagerEvent>,
    outputs: Arc<Mutex<Vec<Proxy<WlOutput>>>>,
}

impl OutputManager {
    pub fn new(sm_source: EventSource<SurfaceManagerEvent>) -> Self {
        OutputManager {
            sm_source,
            outputs: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn new_output(
        &self,
        output_id: u32,
        version: u32,
        registry: &Proxy<WlRegistry>,
    ) {
        let sm_source = self.sm_source.clone();
        let output = registry
            .bind(version, output_id, |output| {
                output.implement(move |event, output| {
                    let mut user_data = output
                        .user_data::<Mutex<OutputUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap();
                    match event {
                        Event::Done => {}
                        Event::Geometry {
                            x,
                            y,
                            physical_width,
                            physical_height,
                            subpixel,
                            model,
                            make,
                            transform,
                        } => {
                            user_data.location = (x, y);
                            user_data.physical_size = (physical_width, physical_height);
                            user_data.subpixel = subpixel;
                            user_data.transform = transform;
                            user_data.model = model;
                            user_data.make = make;
                        }
                        Event::Mode { width, height, refresh, flags } => {
                            let dimensions = (width as u32, height as u32);
                            let refresh_rate = refresh as u32;
                            let is_preferred = flags.contains(WlMode::Preferred);
                            let is_current = flags.contains(WlMode::Current);

                            let existing_mode = user_data.modes
                                .iter_mut()
                                .find(|mode| {
                                    mode.dimensions == dimensions &&
                                        mode.refresh_rate == refresh_rate
                                });
                            match existing_mode {
                                Some(mode) => {
                                    mode.is_preferred = is_preferred;
                                    mode.is_current = is_current;
                                }
                                None => {
                                    let mode = Mode {
                                        dimensions,
                                        refresh_rate,
                                        is_preferred,
                                        is_current,
                                    };
                                    user_data.modes.push(mode);
                                }
                            }
                        }
                        Event::Scale { factor } => {
                            let factor = factor as u32;
                            user_data.scale_factor = factor;
                            let event = SurfaceManagerEvent::OutputScale {
                                output: output.clone(),
                                factor,
                            };
                            sm_source.push_event(event);
                        }
                    }
                }, Mutex::new(OutputUserData::new()))
            }).unwrap();
        self.outputs.lock().unwrap().push(output);
    }

    pub fn remove_output(&self, output_id: u32) {
        self.outputs.lock().unwrap().retain(|output| {
            if output.id() == output_id && output.version() >= 3 {
                output.release();
            }
            output.id() != output_id
        });
    }

    pub fn outputs(&self) -> &Arc<Mutex<Vec<Proxy<WlOutput>>>> {
        &self.outputs
    }

    pub fn get_output(&self, output_id: u32) -> Option<Proxy<WlOutput>> {
        self.outputs.lock().unwrap().iter().find(|output| {
            output.id() == output_id
        }).map(|output| output.clone())
    }
}

#[derive(Clone, Debug)]
/// Compiled information about an output
pub struct OutputUserData {
    /// The model name of this output as advertised by the server
    pub model: String,
    /// The make name of this output as advertised by the server
    pub make: String,
    /// Location of the top-left corner of this output in compositor
    /// space
    ///
    /// Note that the compositor may decide to always report (0,0) if
    /// it decides clients are not allowed to know this information.
    pub location: (i32, i32),
    /// Physical dimensions of this output, in unspecified units
    pub physical_size: (i32, i32),
    /// The subpixel layout for this output
    pub subpixel: Subpixel,
    /// The current transformation applied to this output
    ///
    /// You can pre-render your buffers taking this information
    /// into account and advertising it via `wl_buffer.set_tranform`
    /// for better performances.
    pub transform: Transform,
    /// The scaling factor of this output
    ///
    /// Any buffer whose scaling factor does not match the one
    /// of the output it is displayed on will be rescaled accordingly.
    ///
    /// For example, a buffer of scaling factor 1 will be doubled in
    /// size if the output scaling factor is 2.
    pub scale_factor: u32,
    /// Possible modes for an output
    pub modes: Vec<Mode>,
}

impl OutputUserData {
    pub fn new() -> Self {
        OutputUserData {
            model: String::new(),
            make: String::new(),
            location: (0, 0),
            physical_size: (0, 0),
            subpixel: Subpixel::Unknown,
            transform: Transform::Normal,
            scale_factor: 1,
            modes: Vec::new(),
        }
    }
}

/// A possible mode for an output
#[derive(Clone, Debug)]
pub struct Mode {
    /// Number of pixels of this mode in format `(width, height)`
    ///
    /// for example `(1920, 1080)`
    pub dimensions: (u32, u32),
    /// Refresh rate for this mode, in mHz
    pub refresh_rate: u32,
    /// Whether this is the current mode for this output
    pub is_current: bool,
    /// Whether this is the preferred mode for this output
    pub is_preferred: bool,
}
