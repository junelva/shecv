#![allow(dead_code)]

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
    let mut key1 = store.insert("world".to_string());
    let mut key2 = store.insert("interface?".to_string());
    let mut key3 = store.insert(0.0_f64);

    let (sdl, mut state) = State::new(width, height, "testing")?;

    let listui_index = {
        state.new_context().await?;
        let listui_index = state.new_listui()?;
        let listui = &mut state.listuis[listui_index];
        listui.add_labeled_value("hello", &mut key1);
        listui.add_labeled_value("list", &mut key2);
        listui.add_labeled_value("time", &mut key3);
        state.layout_listui(&store, listui_index)?;
        listui_index
    };

    {
        use std::thread::sleep;
        use std::time::Duration;

        let sdl = Rc::new(RefCell::new(sdl));
        let state = Rc::new(RefCell::new(state));
        let store = Rc::new(RefCell::new(store));

        loop {
            process_events(Rc::clone(&state), Rc::clone(&sdl), Rc::clone(&store))();

            let mut state = state.borrow_mut();
            match state.flow_command {
                window::FlowCommand::Quit => break,
                window::FlowCommand::None => {}
            }

            let context = state.context.as_mut().unwrap();
            context.texts.clear();
            state.layout_listui(&store.borrow_mut(), listui_index)?;

            sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}
