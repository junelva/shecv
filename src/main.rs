#![allow(dead_code)]

use log::info;
use std::{cell::RefCell, error::Error, rc::Rc};

mod geo;
mod listui;
mod text;
mod types;
mod window;

use types::ValueStore;
use window::{process_events, State};

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(init_loop())?;
    Ok(())
}

async fn init_loop() -> Result<(), Box<dyn Error>> {
    let (width, height) = (640, 480);
    env_logger::init();

    let mut store = ValueStore::new();
    let key1 = store.insert("key1", "world".to_string());
    let key2 = store.insert("key2", "interface?".to_string());
    let key3 = store.insert("key3", 0.0_f64);

    let (sdl, mut state) = State::new(width, height, "testing")?;

    let listui_index = {
        state.new_context().await?;
        let listui_index = state.new_listui()?;
        let listui = &mut state.listuis[listui_index];
        listui.add_labeled_value("hell", Rc::clone(&key1));
        listui.add_labeled_value("list", Rc::clone(&key2));
        listui.add_labeled_value("time", Rc::clone(&key3));
        listui.add_labeled_value("time", Rc::clone(&key3));
        state.layout_listui(&store, listui_index)?;
        listui_index
    };

    {
        use std::thread::sleep;
        use std::time::{Duration, Instant};

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
        }
    }

    Ok(())
}
