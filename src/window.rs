use glam::{IVec2, UVec2};

use glyphon::Resolution;
use sdl2::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    video::Window,
    Sdl,
};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, IndexFormat, Instance,
    InstanceDescriptor, LoadOp, Operations, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use std::{
    cell::RefCell,
    error::Error,
    fs::metadata,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use crate::listui::ListInterface;
use crate::{
    geo::GeoManager,
    listui::ListAnchor,
    types::{TextureSheetDefinition, ValueStore},
};
use crate::{
    text::TextCollection,
    types::{ComponentTransform, PixelRect},
};

enum FileWatcherAction {
    ReloadShader,
}

struct FileWatcherEntry {
    path: String,
    last_modified: SystemTime,
    action: FileWatcherAction,
}

pub struct FileWatcher {
    entries: Vec<FileWatcherEntry>,
}

impl FileWatcher {
    fn new() -> Self {
        FileWatcher { entries: vec![] }
    }

    pub fn add_path(&mut self, path: &str) {
        let metadata = metadata(path).unwrap();
        self.entries.push(FileWatcherEntry {
            path: path.to_string(),
            last_modified: metadata.modified().unwrap(),
            action: FileWatcherAction::ReloadShader,
        })
    }
}

#[derive(Default, Copy, Clone)]
pub enum FlowCommand {
    #[default]
    None,
    Quit,
}

#[derive(Default)]
pub struct State<'a> {
    pub title: Option<String>,
    pub window: Option<Window>,
    pub flow_command: FlowCommand,
    pub context: Option<Context<'a>>,
    pub listuis: Vec<ListInterface>,
    pub ui_wait: Duration,
    pub last_ui_time: Option<SystemTime>,
}

impl State<'_> {
    pub fn new(width: u32, height: u32, title: &str) -> Result<(sdl2::Sdl, State), Box<dyn Error>> {
        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("SDL2/wgpu", width, height)
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .unwrap();

        Ok((
            sdl,
            State {
                title: Some(title.to_string()),
                flow_command: FlowCommand::None,
                window: Some(window),
                listuis: vec![],
                ui_wait: Duration::from_millis(60),
                last_ui_time: None,
                ..Default::default()
            },
        ))
    }

    pub fn layout_listui(
        &mut self,
        store: &ValueStore,
        listui_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        // here i'll make the geometry instance group
        // and populate it according to the listui as specified
        let listui = &self.listuis[listui_index];
        let context = self.context.as_mut().unwrap();
        let config = context.config.lock().unwrap();

        // starting out, we look at the listui and determine where it will go
        let wh = IVec2::new(220, 40);
        let tl = {
            match listui.anchor {
                ListAnchor::Left => IVec2::new(0, 0),
                ListAnchor::Middle => IVec2::new(config.width as i32 / 2 - wh.x / 2, 0),
                ListAnchor::Right => IVec2::new(config.width as i32 - wh.x, 0),
                ListAnchor::Hidden => IVec2::new(0, 0),
            }
        };
        let pad = 4u32;
        let mut y_offset = 0;
        let mut final_x = 0;

        context.geos.instance_groups[listui.render_group_index]
            .instance_buffer_manager
            .clear();

        // for each element in the listui, create a background rect and text label
        context.texts.clear();
        for (i, item) in listui.entries.iter().enumerate() {
            let selected = listui.selected_index == i as i32;

            let mut text_index = context.texts.new_text(
                (
                    tl.x as f64 + 2.5,
                    (tl.y + y_offset) as f64 + 2.5,
                    wh.x as f64,
                    wh.y as f64,
                ),
                format!("{}: ", item.label).as_str(),
                1.0,
                if selected {
                    listui.style.li_selected
                } else {
                    listui.style.li_unselected
                },
            );

            // ------------------------ v -_o
            let label_width = context.texts.texts[text_index].buffer.size().0.unwrap();

            let value_ref = item.value.borrow();
            let value = value_ref.load(store);

            text_index = context.texts.new_text(
                (
                    (tl.x + label_width as i32) as f64 + 2.5,
                    (tl.y + y_offset) as f64 + 2.5,
                    wh.x as f64,
                    wh.y as f64,
                ),
                format!("{}", value).as_str(),
                1.0,
                if selected {
                    listui.style.li_selected
                } else {
                    listui.style.li_unselected
                },
            );

            let value_width = context.texts.texts[text_index].buffer.size().0.unwrap();
            let elem_width = (label_width + value_width) as u32;

            if elem_width as i32 > final_x {
                final_x = elem_width as i32;
            }

            y_offset += wh.y;
        }

        // a background rect is created - will it work!? the answer: yes...
        let _geo_index = context.geos.instance_groups[listui.render_group_index].add_new(
            context.queue.clone(),
            ComponentTransform::unit_square_transform_from_pixel_rect(PixelRect {
                xy: IVec2::new(tl.x, tl.y),
                wh: UVec2::new(final_x as u32, config.height),
                extent: UVec2::new(config.width, config.height),
            }),
            0,
            0,
            listui.style.bg,
        );

        // but now we need to loop again and place the foreground rects
        y_offset = 0;
        for (i, _item) in listui.entries.iter().enumerate() {
            let selected = listui.selected_index == i as i32;
            let _geo_index = context.geos.instance_groups[listui.render_group_index].add_new(
                context.queue.clone(),
                ComponentTransform::unit_square_transform_from_pixel_rect(PixelRect {
                    xy: IVec2::new(tl.x + pad as i32, tl.y + y_offset + pad as i32),
                    wh: UVec2::new(final_x as u32 - pad * 2, wh.y as u32 - pad * 2),
                    extent: UVec2::new(config.width, config.height),
                }),
                0,
                0,
                if selected {
                    listui.style.li_selected_bg
                } else {
                    listui.style.li_unselected_bg
                },
            );
            y_offset += wh.y;
        }

        Ok(())
    }

    pub fn new_listui(&mut self) -> Result<usize, Box<dyn Error>> {
        let context = self.context.as_mut().unwrap();
        context.file_watcher.add_path("src/shader.wgsl");
        let render_group_index = {
            let shader_path = "src/shader.wgsl";
            context.file_watcher.add_path(shader_path);
            let config = context.config.lock().unwrap();
            context.geos.new_unit_square(
                512,
                config.format,
                config.width,
                config.height,
                TextureSheetDefinition::default(),
                shader_path,
            )?
        };

        self.listuis
            .push(ListInterface::default(render_group_index));
        Ok(self.listuis.len() - 1)
    }

    pub async fn new_context(&mut self) -> Result<(), Box<dyn Error>> {
        let window = self.window.as_ref().unwrap();

        let size = window.size();

        // instance, adapter, device, queue
        let instance = Instance::new(InstanceDescriptor {
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .expect("wgpu request_adapter failed");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    ..Default::default()
                },
                None,
            )
            .await?;

        // surface, format, config
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window)?)
        }?;
        let swapchain_format = TextureFormat::Bgra8UnormSrgb;
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.0,
            height: size.1,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let device_arc = Arc::<Mutex<Device>>::new(Mutex::new(device));
        let queue_arc = Arc::<Mutex<Queue>>::new(Mutex::new(queue));
        let texts = TextCollection::new(device_arc.clone(), queue_arc.clone(), swapchain_format);

        self.context = Some(Context {
            device: device_arc.clone(),
            queue: queue_arc.clone(),
            surface: Arc::<Mutex<Surface>>::new(Mutex::new(surface)),
            config: Arc::<Mutex<SurfaceConfiguration>>::new(Mutex::new(config)),
            swapchain_format,
            texts,
            geos: GeoManager::new(device_arc.clone(), queue_arc.clone(), swapchain_format),
            file_watcher: FileWatcher::new(),
        });

        Ok(())
    }
}

pub struct Context<'a> {
    pub device: Arc<Mutex<Device>>,
    pub queue: Arc<Mutex<Queue>>,
    pub surface: Arc<Mutex<Surface<'a>>>,
    pub config: Arc<Mutex<SurfaceConfiguration>>,
    pub swapchain_format: TextureFormat,
    pub texts: TextCollection,
    pub geos: GeoManager,
    pub file_watcher: FileWatcher,
}

impl Context<'_> {
    pub fn check_watched_files(&mut self) -> Result<(), Box<dyn Error>> {
        for fwe in self.file_watcher.entries.iter_mut() {
            let metadata = metadata(&*fwe.path)?;
            if metadata.modified().unwrap() > fwe.last_modified {
                match fwe.action {
                    FileWatcherAction::ReloadShader => {
                        self.geos.reload_shader(self.device.clone(), &fwe.path)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        self.check_watched_files()?;
        let config = self.config.lock().unwrap();
        for group in self.geos.instance_groups.iter_mut() {
            group.instance_buffer_manager.recalc_screen_instances(
                self.queue.clone(),
                UVec2::new(config.width, config.height),
            );
        }
        Ok(())
    }

    pub fn resize(&mut self, surface: Arc<Mutex<Surface>>, size: (u32, u32)) {
        let device = self.device.lock().unwrap();
        let mut config = self.config.lock().unwrap();
        config.width = size.0;
        config.height = size.1;
        let surface = surface.lock().unwrap();
        surface.configure(&device, &config);

        // below functions were to resize on-screen geometry instances...
        // this is not necessary atm bc we recreate geo instances every frame

        // self.geos
        //     .update_view(self.queue.clone(), config.width, config.height);
        // for group in self.geos.instance_groups.iter_mut() {
        //     group.mark_all_for_update();
        // }
    }

    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        let surface = self.surface.clone();
        let config = self.config.lock().unwrap();

        self.texts.prepare(
            self.device.clone(),
            self.queue.clone(),
            config.width,
            config.height,
        )?;

        let device = self.device.lock().unwrap();
        let queue = self.queue.lock().unwrap();
        let surface = surface.lock().unwrap();

        let frame = surface.get_current_texture()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.01,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // include geos in pass
            if !self.geos.instance_groups.is_empty() {
                for (i, ig) in self.geos.instance_groups.iter().enumerate() {
                    pass.set_pipeline(&ig.render_pipeline_record.render_pipeline);
                    pass.set_bind_group(0, &ig.bind_group, &[]);
                    pass.set_index_buffer(ig.index_buffer.slice(..), IndexFormat::Uint16);
                    pass.set_vertex_buffer(0, ig.vertex_buffer.slice(..));
                    pass.set_vertex_buffer(1, ig.instance_buffer_manager.buffer.slice(..));
                    pass.draw_indexed(0..6_u32, 0, 0..self.geos.num_instances(i));
                }
            }

            // include text labels in pass
            self.texts
                .text_renderer
                .render(&self.texts.atlas, &self.texts.viewport, &mut pass)?;
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
        self.texts.trim_atlas();

        Ok(())
    }
}

pub fn process_events(
    state: Rc<RefCell<State>>,
    sdl: Rc<RefCell<Sdl>>,
    store: Rc<RefCell<ValueStore>>,
) -> impl FnMut() + '_ {
    let mut events = sdl.borrow_mut().event_pump().unwrap();

    move || {
        for event in events.poll_iter() {
            match event {
                Event::Window {
                    timestamp: _,
                    window_id: _,
                    win_event,
                } => match win_event {
                    WindowEvent::Resized(w, h) => {
                        let mut state = state.borrow_mut();
                        let context = state.context.as_mut().unwrap();
                        context.resize(context.surface.clone(), (w as u32, h as u32));
                        let sdl = sdl.borrow_mut();
                        sdl.event().unwrap().flush_events(0, 0xFFFF);
                        let mut listuis_to_update = vec![];
                        for (index, listui) in state.listuis.iter().enumerate() {
                            if listui.anchor != ListAnchor::Hidden {
                                listuis_to_update.push(index);
                            }
                        }
                        for index in listuis_to_update {
                            let _ = state.layout_listui(&store.borrow_mut(), index);
                        }
                    }
                    WindowEvent::Enter => {}
                    _ => {}
                },
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    state.borrow_mut().flow_command = FlowCommand::Quit;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {}
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {}
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    let mut state = state.borrow_mut();
                    let input_ok = {
                        let mut input_ok = true;
                        if state.last_ui_time.is_some()
                            && state.last_ui_time.unwrap() + state.ui_wait > SystemTime::now()
                        {
                            input_ok = false;
                        }
                        input_ok
                    };
                    if input_ok {
                        for listui in &mut state.listuis {
                            if listui.selected_index == 0 {
                                listui.selected_index = (listui.entries.len() - 1) as i32;
                            } else if listui.selected_index >= 0 {
                                listui.selected_index -= 1;
                            }
                        }
                        state.last_ui_time = Some(SystemTime::now());
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    let mut state = state.borrow_mut();
                    let input_ok = {
                        let mut input_ok = true;
                        if state.last_ui_time.is_some()
                            && state.last_ui_time.unwrap() + state.ui_wait > SystemTime::now()
                        {
                            input_ok = false;
                        }
                        input_ok
                    };
                    if input_ok {
                        for listui in &mut state.listuis {
                            if listui.selected_index == (listui.entries.len() - 1) as i32 {
                                listui.selected_index = 0;
                            } else if listui.selected_index >= 0 {
                                listui.selected_index =
                                    (listui.selected_index + 1) % listui.entries.len() as i32;
                            }
                        }
                        state.last_ui_time = Some(SystemTime::now());
                    }
                }
                _ => {}
            }
        }

        {
            let mut store = store.borrow_mut();
            let mut value_ref = store.get("key1");
            value_ref.replace(Box::new("banana".to_string()), &mut store);
            let mut k3 = store.get("key3");
            let val = k3.load(&store).as_any().downcast_ref().unwrap();
            k3.replace(Box::new(val + 0.01), &mut store);
        }

        let mut state = state.borrow_mut();
        let context = state.context.as_mut().unwrap();
        {
            let config = context.config.lock().unwrap();
            context.texts.viewport.update(
                &context.queue.clone().lock().unwrap(),
                Resolution {
                    width: config.width,
                    height: config.height,
                },
            );

            context
                .texts
                .prepare(
                    context.device.clone(),
                    context.queue.clone(),
                    config.width,
                    config.height,
                )
                .unwrap();
        }

        context.render().unwrap();
    }
}
