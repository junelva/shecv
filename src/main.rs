#![allow(dead_code)]

pub use glam::{Mat4, Vec2, Vec3, Vec4};
use types::ListItemData;

mod geo;
mod listui;
mod text;
mod types;
mod window;

use crate::types::Value;
use crate::window::{process_events, State};

use std::{cell::RefCell, error::Error, rc::Rc};

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(init_loop())?;
    Ok(())
}

async fn init_loop() -> Result<(), Box<dyn Error>> {
    let (width, height) = (640, 480);
    env_logger::init();

    let mut store = vec![];
    let mut key1 = Value::<dyn ListItemData>::new(Box::new("world".to_string()), &mut store);
    let mut key2 = Value::<dyn ListItemData>::new(Box::new("interface?".to_string()), &mut store);
    let mut key3 = Value::<dyn ListItemData>::new(Box::new(0.0_f64), &mut store);

    let (sdl, mut state) = State::new(width, height, "testing")?;

    let listui_index = {
        state.new_context().await?;
        let listui_index = state.new_listui()?;
        let listui = &mut state.listuis[listui_index];
        listui.add_entry(listui::ListItemType::Text, "hello", true, false, &mut key1);
        listui.add_entry(listui::ListItemType::Text, "list", true, false, &mut key2);
        listui.add_entry(listui::ListItemType::Text, "time", true, false, &mut key3);
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

            {
                let context = state.context.as_mut().unwrap();
                context.texts.clear();
            }
            state.layout_listui(&store.borrow_mut(), listui_index)?;
            sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}
