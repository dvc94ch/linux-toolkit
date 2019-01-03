use std::sync::Mutex;
use std::io::{BufWriter, Seek, SeekFrom, Write, Error};
use byteorder::{NativeEndian, WriteBytesExt};
use linux_toolkit::wayland::Proxy;
use linux_toolkit::wayland::environment::Environment;
use linux_toolkit::wayland::mem_pool::{DoubleMemPool, MemPool};
use linux_toolkit::wayland::output::OutputUserData;
use linux_toolkit::wayland::seat::SeatUserData;
use linux_toolkit::wayland::shm::Format;
use linux_toolkit::wayland::surface::{WlSurface, SurfaceRequests};
use linux_toolkit::wayland::xdg_shell::{XdgShell, XdgSurfaceEvent};

fn main() {
    let mut environment = Environment::initialize().unwrap();
    let globals = environment.globals.clone();
    let mut pools = DoubleMemPool::new(&environment.shm, || {}).unwrap();
    let xdg_shell = XdgShell::new(&globals, environment.surface_manager.clone());

    {
        let outputs = environment.output_manager
            .outputs()
            .lock()
            .unwrap();

        for output in outputs.iter() {
            let ud = output.user_data::<Mutex<OutputUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            println!("{:?}", *ud);
        }
    }

    {
        let seats = environment.seat_manager
            .seats()
            .lock()
            .unwrap();

        for seat in seats.iter() {
            let ud = seat.user_data::<Mutex<SeatUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            println!("{}", ud.name());
        }
    }

    let xdg_surface = xdg_shell.create_shell_surface();

    let mut close = false;
    let mut surface_size = (1024, 768);
    let mut surface_scale_factor = 1;

    loop {
        xdg_surface.poll_events(|event, xdg_surface| {
            match event {
                XdgSurfaceEvent::Close => {
                    close = true;
                }
                XdgSurfaceEvent::Configure { size, .. } => {
                    surface_size = size.unwrap_or(surface_size);
                    if let Some(pool) = pools.pool() {
                        redraw(
                            pool,
                            xdg_surface.surface(),
                            surface_size,
                            surface_scale_factor,
                        ).unwrap();
                    }
                }
                XdgSurfaceEvent::Scale { scale_factor } => {
                    surface_scale_factor = scale_factor;
                    xdg_surface.surface().set_buffer_scale(scale_factor as i32);
                }
                XdgSurfaceEvent::Pointer { event } => {
                    println!("{:?}", event);
                }
                XdgSurfaceEvent::Keyboard { event } => {
                    println!("{:?}", event);
                }
                XdgSurfaceEvent::Touch { event: _ } => {
                    println!("touch event");
                }
            }
        });
        if close {
            break;
        }
        environment.handle_events();
    }
}

fn redraw(
    pool: &mut MemPool,
    surface: &Proxy<WlSurface>,
    size: (u32, u32),
    scale_factor: u32,
) -> Result<(), Error> {
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
    surface.commit();
    Ok(())
}

/*let data_device_manager: Proxy<wl_data_device_manager::WlDataDeviceManager> = globals
.instantiate_auto(|data_device_manager| {
data_device_manager.implement(
|event, _data_device_manager| match event {}, ())
        })
        .expect("Server didn't advertise `wl_data_device_manager`");

    let xdg_decoration: Proxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1> = globals
        .instantiate_auto(|manager| {
            manager.implement(|event, _manager| match event {}, ())
        })
        .expect("Server didn't advertise `zxdg_decoration_manager`");*/

//use wayland_client::protocol::wl_data_device_manager;
//use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1;
