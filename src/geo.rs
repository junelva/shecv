use crate::types::{
    ColorRGBA, ComponentTransform, GeoUniformMatrix, GeoUniformVec2, InstanceBufferManager,
    RenderPipelineRecord, TextureSheet, TextureSheetDefinition, UNIT_SQUARE_BUFFER_LAYOUT,
    UNIT_SQUARE_INDICES, UNIT_SQUARE_VERTICES,
};
use image::ImageReader;
use image::RgbaImage;
use std::{
    borrow::Cow,
    error::Error,
    fs::read_to_string,
    mem::size_of,
    path::Path,
    sync::{Arc, Mutex},
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BlendState, ColorTargetState, ColorWrites,
};

use glam::{Mat4, UVec2, Vec2};
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Buffer, BufferBindingType, BufferSize, BufferUsages, Device, Extent3d, Face,
    FragmentState, MultisampleState, PrimitiveState, Queue, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureDescriptor, TextureFormat,
    VertexState,
};

pub enum GeoViewType {
    Orthographic,
    Perspective,
}

// various things needed to render geometry.
pub struct GeoInstances {
    pub render_pipeline_record: RenderPipelineRecord,
    pub bind_group: BindGroup,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub sheet: TextureSheet,
    pub view_type: GeoViewType,
    pub view_matrix_uniform: GeoUniformMatrix,
    pub screen_size_uniform: GeoUniformVec2,
    pub instance_buffer_manager: InstanceBufferManager,
}

impl GeoInstances {
    pub fn add_new(
        &mut self,
        queue: Arc<Mutex<Queue>>,
        transform: ComponentTransform,
        cluster_index: usize,
        sub_index: usize,
        color: ColorRGBA,
    ) -> usize {
        let index = self.instance_buffer_manager.data.len();
        self.instance_buffer_manager.add_instance(
            queue,
            transform,
            self.sheet.cluster_sub_transform(cluster_index, sub_index),
            color,
        );
        index
    }

    pub fn mark_all_for_update(&mut self) {
        for instance in self.instance_buffer_manager.data.iter_mut() {
            instance.needs_update = true;
        }
    }

    // pub fn recalc_screen_instances(&mut self, queue: Arc<Mutex<Queue>>, screen: UVec2) {
    //     self.instance_buffer_manager
    //         .recalc_screen_instances(queue, screen);
    // }
}

fn load_texture(
    device: Arc<Mutex<Device>>,
    queue: Arc<Mutex<Queue>>,
    sheet_info: TextureSheetDefinition,
) -> Result<TextureSheet, Box<dyn Error>> {
    let device = device.lock().unwrap();
    let queue = queue.lock().unwrap();
    let (image, path): (RgbaImage, String) = {
        let texture_exists = Path::new(&sheet_info.path).try_exists()?;
        if texture_exists {
            let result: (RgbaImage, String) = (
                ImageReader::open(sheet_info.path.clone())?
                    .decode()?
                    .to_rgba8(),
                sheet_info.path.clone(),
            );
            result
        } else {
            let result: (RgbaImage, String) = (
                image::load_from_memory(include_bytes!("../res/px.png"))?.to_rgba8(),
                "../res/px.png".to_string(),
            );
            result
        }
    };

    let dimensions = image.dimensions();
    let extent = Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&TextureDescriptor {
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some(&path),
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        extent,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Ok(TextureSheet {
        sheet_info,
        dimensions: UVec2::new(dimensions.0, dimensions.1),
        texture,
        sampler,
        view,
    })
}

pub struct GeoManager {
    pub device: Arc<Mutex<Device>>,
    pub queue: Arc<Mutex<Queue>>,
    #[allow(dead_code)]
    pub format: TextureFormat,
    pub view_ortho: Mat4,
    pub view_persp: Mat4,
    pub instance_groups: Vec<GeoInstances>,
}

impl GeoManager {
    pub fn new(
        device: Arc<Mutex<Device>>,
        queue: Arc<Mutex<Queue>>,
        format: TextureFormat,
    ) -> Self {
        Self {
            device,
            queue,
            format,
            view_ortho: Mat4::orthographic_lh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0),
            view_persp: Mat4::perspective_lh(2.0, 4.0 / 3.0, -1.0, 1000.0),
            instance_groups: vec![],
        }
    }

    pub fn num_instances(&self, group_index: usize) -> u32 {
        self.instance_groups[group_index]
            .instance_buffer_manager
            .data
            .len() as u32
    }

    pub fn update_view(&mut self, queue: Arc<Mutex<Queue>>, width: u32, height: u32) {
        let queue = queue.lock().unwrap();
        // let _view_matrix = Mat4::orthographic_lh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);
        self.view_persp = Mat4::perspective_rh(2.0, width as f32 / height as f32, 0.0, 100.0);
        let screen_size = Vec2::new(width as f32, height as f32);
        for ig in self.instance_groups.iter_mut() {
            ig.view_matrix_uniform.matrix = match ig.view_type {
                GeoViewType::Orthographic => self.view_ortho,
                GeoViewType::Perspective => self.view_persp,
            };
            queue.write_buffer(
                &ig.view_matrix_uniform.buffer,
                0,
                bytemuck::cast_slice(&[ig.view_matrix_uniform.matrix]),
            );
            ig.screen_size_uniform.vec = screen_size;
            queue.write_buffer(
                &ig.screen_size_uniform.buffer,
                0,
                bytemuck::cast_slice(&[screen_size]),
            );
        }
    }

    pub fn reload_shader(
        &mut self,
        device: Arc<Mutex<wgpu::Device>>,
        shader_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        let device = device.lock().unwrap();

        // for every instance group...
        for ig in self.instance_groups.iter_mut() {
            // does the instance group use this shader path?
            if ig.render_pipeline_record.shader_path == *shader_path.to_string() {
                // if so, rebuild the shader.
                ig.render_pipeline_record.shader_module =
                    device.create_shader_module(ShaderModuleDescriptor {
                        label: Some(&*format!("shader {}", shader_path)),
                        source: ShaderSource::Wgsl(Cow::Borrowed(&*read_to_string(shader_path)?)),
                    });

                // and rebuild the render pipeline.
                ig.render_pipeline_record.render_pipeline =
                    device.create_render_pipeline(&RenderPipelineDescriptor {
                        label: Some(&*format!("pipeline {}", shader_path)),
                        layout: Some(&ig.render_pipeline_record.pipeline_layout),
                        vertex: VertexState {
                            module: &ig.render_pipeline_record.shader_module,
                            entry_point: "vs_main",
                            buffers: &UNIT_SQUARE_BUFFER_LAYOUT,
                            compilation_options: Default::default(),
                        },
                        fragment: Some(FragmentState {
                            module: &ig.render_pipeline_record.shader_module,
                            entry_point: "fs_main",
                            targets: &[Some(ColorTargetState {
                                format: ig.render_pipeline_record.format,
                                blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                                write_mask: ColorWrites::ALL,
                            })],
                            compilation_options: Default::default(),
                        }),
                        primitive: PrimitiveState {
                            cull_mode: None,
                            ..Default::default()
                        },
                        depth_stencil: None,
                        multisample: MultisampleState::default(),
                        multiview: None,
                        cache: None,
                    });
            }
        }
        Ok(())
    }

    pub fn new_unit_square(
        &mut self,
        view_type: GeoViewType,
        max_instances: usize,
        format: TextureFormat,
        wh: (u32, u32),
        sheet_info: TextureSheetDefinition,
        shader_path: &str,
    ) -> Result<usize, Box<dyn Error>> {
        let width = wh.0;
        let height = wh.1;
        // prepare texture sheet data
        let sheet = load_texture(self.device.clone(), self.queue.clone(), sheet_info)?;

        let device = self.device.lock().unwrap();

        // compile shader code
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(shader_path),
            source: ShaderSource::Wgsl(Cow::Borrowed(&*read_to_string(shader_path)?)),
        });

        // vertex and index buffers
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("unit square vertices"),
            contents: bytemuck::cast_slice(&UNIT_SQUARE_VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("unit square indices"),
            contents: bytemuck::cast_slice(&UNIT_SQUARE_INDICES),
            usage: BufferUsages::INDEX,
        });

        // view matrix uniform setup
        let (_view_matrix, view_matrix_uniform) = {
            match view_type {
                GeoViewType::Orthographic => {
                    let view_matrix = Mat4::orthographic_lh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);
                    let view_matrix_uniform = GeoUniformMatrix {
                        matrix: view_matrix,
                        buffer: device.create_buffer_init(&BufferInitDescriptor {
                            label: Some("unit square view_matrix"),
                            contents: bytemuck::cast_slice(&[view_matrix]),
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        }),
                    };
                    (view_matrix, view_matrix_uniform)
                }
                GeoViewType::Perspective => {
                    let view_matrix = Mat4::perspective_rh(2.0, 4.0 / 3.0, 0.01, 100.0);
                    let view_matrix_uniform = GeoUniformMatrix {
                        matrix: view_matrix,
                        buffer: device.create_buffer_init(&BufferInitDescriptor {
                            label: Some("unit square view_matrix"),
                            contents: bytemuck::cast_slice(&[view_matrix]),
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        }),
                    };
                    (view_matrix, view_matrix_uniform)
                }
            }
        };

        // screen size uniform setup
        let screen_size = Vec2 {
            x: width as f32,
            y: height as f32,
        };
        let screen_size_uniform = GeoUniformVec2 {
            vec: screen_size,
            buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("unit square screen_res"),
                contents: bytemuck::cast_slice(&[screen_size]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
        };

        // bind group layout and bind group creation; pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new((size_of::<Mat4>()) as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new((size_of::<Vec2>()) as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_matrix_uniform.buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: screen_size_uniform.buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&sheet.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&sheet.sampler),
                },
            ],
            label: None,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // render pipeline itself, with necessary components for reconstruction retained.
        let render_pipeline_record = RenderPipelineRecord {
            render_pipeline: device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("unit square pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &UNIT_SQUARE_BUFFER_LAYOUT,
                    compilation_options: Default::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format,
                        blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            }),
            pipeline_layout,
            shader_module,
            shader_path: shader_path.to_string(),
            format,
        };

        // drop device here because it's used to make the instance buffer below.
        drop(device);

        let index = self.instance_groups.len();
        self.instance_groups.push(GeoInstances {
            render_pipeline_record,
            bind_group,
            vertex_buffer,
            index_buffer,
            sheet,
            view_type,
            view_matrix_uniform,
            screen_size_uniform,
            instance_buffer_manager: InstanceBufferManager::new(max_instances, self.device.clone()),
        });

        Ok(index)
    }
}
