use std::{error::Error, sync::Arc};

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer,
};
use wgpu::{MultisampleState, TextureFormat};

use crate::types::ColorRGBA;

pub struct TextLabel {
    pub buffer: Buffer,
    pub left: f64,
    pub top: f64,
    pub scale: f64,
    pub bounds: TextBounds,
    pub color: ColorRGBA,
}

pub struct TextCollection {
    pub texts: Vec<TextLabel>,
    pub font_system: FontSystem,
    pub text_renderer: TextRenderer,
    pub cache: SwashCache,
    pub atlas: TextAtlas,
}

impl TextCollection {
    pub fn new(
        device: Arc<std::sync::Mutex<wgpu::Device>>,
        queue: Arc<std::sync::Mutex<wgpu::Queue>>,
        swapchain_format: TextureFormat,
    ) -> Self {
        let device = device.lock().unwrap();
        let queue = queue.lock().unwrap();

        let font_system = FontSystem::new();
        let cache = SwashCache::new();
        let mut atlas = TextAtlas::new(&device, &queue, swapchain_format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        TextCollection {
            texts: vec![],
            font_system,
            text_renderer,
            cache,
            atlas,
        }
    }

    pub fn clear(&mut self) {
        self.texts.clear();
    }

    pub fn new_text(
        &mut self,
        rect: (f64, f64, f64, f64),
        text: &str,
        text_scale_factor: f64,
        color: ColorRGBA,
    ) {
        let text2 = if text.parse::<f64>().is_ok() {
            let floatval = text.parse::<f64>().unwrap();
            format!("{:.2}", floatval)
        } else {
            text.to_string()
        };

        let display_scale_factor = 1.0f64;
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new((rect.3 * 0.8) as f32, rect.3 as f32),
        );
        let physical_width = (rect.2 * display_scale_factor) as f32;
        let physical_height = (rect.3 * display_scale_factor) as f32;
        buffer.set_size(&mut self.font_system, physical_width, physical_height);
        buffer.set_text(
            &mut self.font_system,
            text2.as_str(),
            Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system);

        self.texts.push(TextLabel {
            buffer,
            left: rect.0,
            top: rect.1,
            scale: text_scale_factor,
            bounds: TextBounds::default(),
            color,
        });
    }

    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }

    pub fn prepare(
        &mut self,
        device: Arc<std::sync::Mutex<wgpu::Device>>,
        queue: Arc<std::sync::Mutex<wgpu::Queue>>,
        screen_width: u32,
        screen_height: u32,
    ) -> Result<(), Box<dyn Error>> {
        let device = device.lock().unwrap();
        let queue = queue.lock().unwrap();

        self.text_renderer.prepare(
            &device,
            &queue,
            &mut self.font_system,
            &mut self.atlas,
            Resolution {
                width: screen_width,
                height: screen_height,
            },
            self.texts.iter().map(|t| TextArea {
                buffer: &t.buffer,
                left: t.left as f32,
                top: t.top as f32,
                scale: t.scale as f32,
                bounds: t.bounds,
                default_color: t.color.to_glyphon_color(),
            }),
            &mut self.cache,
        )?;
        Ok(())
    }
}
