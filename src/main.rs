// #![allow(dead_code)]

use glam::{IVec2, UVec2};
// use hecs::World;
use log::info;
use std::ops::DerefMut;
use std::time::{Duration, Instant};
use std::{cell::RefCell, error::Error, rc::Rc};
mod geo;
mod listui;
mod text;
mod types;
mod window;

use types::{ColorRGBA, ComponentTransform, PixelRect, TextureSheetDefinition, ValueStore};
use window::{process_events, State};

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(init_loop())?;
    Ok(())
}

async fn init_loop() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let app_start_time = Instant::now();
    let mut store = ValueStore::new();
    let time = store.insert("time", 0.0_f64);

    // let mut world = World::new();
    // let ent = world.spawn(())

    let (width, height) = (640, 480);
    let (sdl, mut state) = State::new(width, height, "testing")?;

    let listui_index = {
        state.new_context().await?;
        let listui_index = state.new_listui()?;
        let listui = &mut state.listuis[listui_index];
        listui.add_labeled_value("time", Rc::clone(&time));
        state.layout_listui(&store, listui_index)?;
        listui_index
    };

    let (_render_group_index, _geo_index) = {
        let context = state.context.as_mut().unwrap();
        let config = context.config.lock().unwrap();
        context.file_watcher.add_path("src/shader.wgsl");
        let render_group_index = {
            let shader_path = "src/shader.wgsl";
            context.file_watcher.add_path(shader_path);
            context.geos.new_unit_square(
                512,
                config.format,
                config.width,
                config.height,
                TextureSheetDefinition::default(),
                shader_path,
            )?
        };

        let geo_wh = UVec2::new(70, 70);
        let geo_index = context.geos.instance_groups[render_group_index].add_new(
            context.queue.clone(),
            ComponentTransform::unit_square_transform_from_pixel_rect(PixelRect {
                xy: IVec2::new(
                    (config.width as i32 / 2) - geo_wh.x as i32 / 2,
                    (config.height as i32 / 2) - geo_wh.y as i32 / 2,
                ),
                wh: geo_wh,
                extent: UVec2::new(config.width, config.height),
            }),
            0,
            0,
            ColorRGBA::magenta(),
        );

        (render_group_index, geo_index)
    };

    {
        use std::thread::sleep;

        let sdl = Rc::new(RefCell::new(sdl));
        let state = Rc::new(RefCell::new(state));
        let store = Rc::new(RefCell::new(store));

        // nanos per frame at 60 fps: 16_666_667
        // nanos per frame at 30 fps: 33_333_333
        // nanos per frame at 15 fps: 66_666_667
        let desired_frametime = Duration::new(0, 66_666_667);

        loop {
            let loop_start = Instant::now();

            process_events(Rc::clone(&state), Rc::clone(&sdl), Rc::clone(&store))();

            let mut state = state.borrow_mut();
            match state.flow_command {
                window::FlowCommand::Quit => break,
                window::FlowCommand::None => {}
            }
            state.layout_listui(&store.borrow_mut(), listui_index)?;

            let elapsed = loop_start.elapsed();
            info!("ft: {:?}", elapsed);
            if elapsed < desired_frametime {
                sleep(desired_frametime - elapsed);
            }

            let running_time = Box::new(app_start_time.elapsed().as_secs_f64());
            let mut store_borrow = store.borrow_mut();
            let store = store_borrow.deref_mut();
            store.get("time").replace(running_time, store);
        }
    }

    Ok(())
}
