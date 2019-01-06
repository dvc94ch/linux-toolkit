use byteorder::{NativeEndian, WriteBytesExt};
use linux_toolkit::wayland::data_device::DataDeviceEvent;
use linux_toolkit::wayland::environment::Environment;
use linux_toolkit::wayland::mem_pool::{DoubleMemPool, MemPool};
use linux_toolkit::wayland::output::OutputUserData;
use linux_toolkit::wayland::pointer::PointerEvent;
use linux_toolkit::wayland::seat::{SeatEvent, SeatUserData};
use linux_toolkit::wayland::shm::Format;
use linux_toolkit::wayland::surface::{SurfaceRequests, WlSurface};
use linux_toolkit::wayland::xdg_shell::{XdgShell, XdgSurfaceEvent};
use linux_toolkit::wayland::Proxy;
use std::io::{BufWriter, Error, Seek, SeekFrom, Write};
use std::sync::Mutex;

fn main() {
    let mut environment = Environment::initialize(None).unwrap();
    let mut pools = DoubleMemPool::new(&environment.shm, || {}).unwrap();
    let xdg_shell = XdgShell::new(&environment.globals, environment.surface_manager.clone());
    print_outputs(&environment);
    print_seats(&environment);
    let xdg_surface = xdg_shell.create_shell_surface();

    let mut close = false;
    let mut configure = false;
    let mut resize = true;
    let mut surface_size = None;
    let mut surface_scale_factor = 1;

    loop {
        xdg_surface.poll_events(|event, _xdg_surface| match event {
            XdgSurfaceEvent::Close => {
                close = true;
            }
            XdgSurfaceEvent::Configure { size, .. } => {
                configure = true;
                if surface_size != size {
                    surface_size = size;
                    resize = true;
                }
            }
            XdgSurfaceEvent::Scale { scale_factor } => {
                if scale_factor != surface_scale_factor {
                    surface_scale_factor = scale_factor;
                    resize = true;
                }
            }
            XdgSurfaceEvent::Seat { seat_id: _, event } => {
                if let SeatEvent::Pointer {
                    event: PointerEvent::Enter { ref cursor, .. },
                } = event
                {
                    cursor.change_cursor(Some("grabbing".into())).unwrap();
                }
                if let SeatEvent::DataDevice {
                    event:
                        DataDeviceEvent::Enter {
                            offer: Some(ref offer),
                            ..
                        },
                } = event
                {
                    offer.accept(None);
                }
                println!("{:?}", event);
            }
        });
        if close {
            break;
        }
        if configure && resize {
            if let Some(pool) = pools.pool() {
                redraw(
                    pool,
                    xdg_surface.surface(),
                    surface_size,
                    surface_scale_factor,
                )
                .unwrap();
            }
            resize = false;
        }
        environment.handle_events();
    }
}

fn redraw(
    pool: &mut MemPool,
    surface: &Proxy<WlSurface>,
    size: Option<(u32, u32)>,
    scale_factor: u32,
) -> Result<(), Error> {
    let size = size.unwrap_or((1024, 768));
    let (width, height) = (size.0 * scale_factor, size.1 * scale_factor);

    pool.resize((4 * width * height) as usize)?;
    pool.seek(SeekFrom::Start(0))?;
    {
        let mut writer = BufWriter::new(&mut *pool);
        for _i in 0..(width * height) {
            writer.write_u32::<NativeEndian>(0xFF000000)?;
        }
        writer.flush()?;
    }
    let new_buffer = pool.buffer(
        0,
        width as i32,
        height as i32,
        4 * width as i32,
        Format::Argb8888,
    );
    surface.attach(Some(&new_buffer), 0, 0);
    surface.set_buffer_scale(scale_factor as i32);
    surface.commit();
    Ok(())
}

fn print_outputs(environment: &Environment) {
    let outputs = environment.output_manager.outputs().lock().unwrap();

    for output in outputs.iter() {
        let ud = output
            .user_data::<Mutex<OutputUserData>>()
            .unwrap()
            .lock()
            .unwrap();
        println!("{:?}", *ud);
    }
}

fn print_seats(environment: &Environment) {
    let seats = environment.seat_manager.seats().lock().unwrap();

    for seat in seats.iter() {
        let ud = seat
            .user_data::<Mutex<SeatUserData>>()
            .unwrap()
            .lock()
            .unwrap();
        println!("{}", ud.name());
    }
}
