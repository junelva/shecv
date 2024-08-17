use glam::{IVec2, Quat, UVec2};
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    mem::size_of,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
};

use bytemuck::{ByteEq, ByteHash, Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Sampler, Texture, TextureView,
};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, PipelineLayout, Queue, RenderPipeline,
    ShaderModule, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::listui::{ListInterface, OperatorResult};

pub struct ValueStore {
    pub map: HashMap<String, Box<dyn ListItemData>>,
}

impl ValueStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Value<dyn ListItemData> {
        Value {
            p: PhantomData,
            key: key.to_string(),
        }
    }

    pub fn insert<T: 'static + ListItemData>(
        &mut self,
        key: &str,
        v: T,
    ) -> Rc<RefCell<Value<dyn ListItemData>>> {
        Rc::new(RefCell::new(Value::<dyn ListItemData>::new(
            key,
            Box::new(v),
            self,
        )))
    }
}

pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait ListItemData: 'static + ToAny + std::fmt::Display {}

pub struct OpFnMut {
    callback: dyn FnMut(OperatorResult),
}

impl std::fmt::Display for OpFnMut {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<OpFnMut>")
    }
}

impl std::fmt::Display for ListInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<ListInterface>")
    }
}

impl ListItemData for bool {}
impl ListItemData for f32 {}
impl ListItemData for f64 {}
impl ListItemData for i32 {}
impl ListItemData for i64 {}
impl ListItemData for u32 {}
impl ListItemData for u64 {}
impl ListItemData for String {}
// impl ListItemData for OpFnMut {}
// impl ListItemData for ListInterface<'_> {}

#[derive(Debug)]
pub struct Value<T>
where
    T: ListItemData + ?Sized,
{
    p: PhantomData<T>,
    pub key: String,
}

impl<T: ?Sized + 'static> Value<T>
where
    T: 'static + ListItemData,
{
    pub fn load<'a>(&self, store: &'a ValueStore) -> &'a dyn ListItemData {
        store.map.get(&self.key).unwrap().deref()
    }

    pub fn new(
        key: &str,
        boxed_value: Box<dyn ListItemData>,
        store: &mut ValueStore,
    ) -> Value<dyn ListItemData> {
        store.map.insert(key.to_string(), boxed_value);

        Value {
            p: PhantomData,
            key: key.to_string(),
        }
    }

    pub fn replace(&mut self, boxed_value: Box<dyn ListItemData>, store: &mut ValueStore) {
        store.map.remove(&self.key);
        store.map.insert(self.key.as_str().to_string(), boxed_value);
        self.p = PhantomData;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, ByteEq, ByteHash)]
pub struct ColorRGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for ColorRGBA {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 0.0,
            b: 0.5,
            a: 1.0,
        }
    }
}

impl ColorRGBA {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_glyphon_color(self) -> glyphon::Color {
        glyphon::Color::rgba(
            ((self.r * 255.0) as u8).clamp(0, 255),
            ((self.g * 255.0) as u8).clamp(0, 255),
            ((self.b * 255.0) as u8).clamp(0, 255),
            ((self.a * 255.0) as u8).clamp(0, 255),
        )
    }

    pub fn black() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }

    pub fn grey_darkest() -> Self {
        Self {
            r: 0.01,
            g: 0.01,
            b: 0.01,
            a: 1.0,
        }
    }

    pub fn grey_darker() -> Self {
        Self {
            r: 0.02,
            g: 0.02,
            b: 0.02,
            a: 1.0,
        }
    }

    pub fn grey_dark() -> Self {
        Self {
            r: 0.03,
            g: 0.03,
            b: 0.03,
            a: 1.0,
        }
    }

    pub fn grey_medium() -> Self {
        Self {
            r: 0.06,
            g: 0.06,
            b: 0.06,
            a: 1.0,
        }
    }

    pub fn grey_light() -> Self {
        Self {
            r: 0.40,
            g: 0.40,
            b: 0.40,
            a: 1.0,
        }
    }

    pub fn grey_lighter() -> Self {
        Self {
            r: 0.60,
            g: 0.60,
            b: 0.60,
            a: 1.0,
        }
    }

    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub location: Vec3,
    pub tex_coords: Vec2,
}

pub const UNIT_SQUARE_VERTICES: [Vertex; 4] = [
    Vertex {
        location: Vec3::new(0.0, 0.0, 0.0),
        tex_coords: Vec2::new(0.0, 0.0),
    },
    Vertex {
        location: Vec3::new(1.0, 0.0, 0.0),
        tex_coords: Vec2::new(1.0, 0.0),
    },
    Vertex {
        location: Vec3::new(1.0, -1.0, 0.0),
        tex_coords: Vec2::new(1.0, 1.0),
    },
    Vertex {
        location: Vec3::new(0.0, -1.0, 0.0),
        tex_coords: Vec2::new(0.0, 1.0),
    },
];
pub const UNIT_SQUARE_INDICES: [u16; 6] = [0, 2, 1, 2, 0, 3];

pub const UNIT_SQUARE_BUFFER_LAYOUT: [VertexBufferLayout<'_>; 2] = [
    VertexBufferLayout {
        array_stride: (size_of::<Vec3>() + size_of::<Vec2>()) as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                // vertex position
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                // vertex tex coord
                format: VertexFormat::Float32x2,
                offset: size_of::<Vec3>() as u64,
                shader_location: 1,
            },
        ],
    },
    VertexBufferLayout {
        array_stride: size_of::<InstanceData>() as BufferAddress,
        step_mode: VertexStepMode::Instance,
        attributes: &[
            // mat4x4 transform
            VertexAttribute {
                offset: 0,
                shader_location: 5,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 4]>() as BufferAddress,
                shader_location: 6,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 8]>() as BufferAddress,
                shader_location: 7,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 12]>() as BufferAddress,
                shader_location: 8,
                format: VertexFormat::Float32x4,
            },
            // mat4x4 texture transform
            VertexAttribute {
                offset: size_of::<[f32; 16]>() as BufferAddress,
                shader_location: 9,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 20]>() as BufferAddress,
                shader_location: 10,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 24]>() as BufferAddress,
                shader_location: 11,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                offset: size_of::<[f32; 28]>() as BufferAddress,
                shader_location: 12,
                format: VertexFormat::Float32x4,
            },
            // vec4 color
            VertexAttribute {
                offset: size_of::<[f32; 32]>() as BufferAddress,
                shader_location: 13,
                format: VertexFormat::Float32x4,
            },
        ],
    },
];

pub struct RenderPipelineRecord {
    pub render_pipeline: RenderPipeline,
    pub pipeline_layout: PipelineLayout,
    pub shader_module: ShaderModule,
    pub shader_path: String,
    pub format: TextureFormat,
}

pub struct GeoUniformVec2 {
    pub vec: Vec2,
    pub buffer: Buffer,
}

pub struct GeoUniformMatrix {
    pub matrix: Mat4,
    pub buffer: Buffer,
}

pub struct Instance {
    pub needs_update: bool,
    pub transform: ComponentTransform,
    pub tex_transform: ComponentTransform,
    pub color: ColorRGBA,
}

impl Instance {
    pub fn translate(&mut self, by: Vec3) {
        self.transform.location += by;
        self.needs_update = true;
    }
}

#[derive(Copy, Clone, Pod, Zeroable, ByteEq, ByteHash)]
#[repr(C)]
pub struct InstanceData {
    pub transform: Mat4,
    pub tex_transform: Mat4,
    pub color: ColorRGBA,
}

impl Default for InstanceData {
    fn default() -> Self {
        InstanceData {
            transform: Mat4::IDENTITY,
            tex_transform: Mat4::IDENTITY,
            color: ColorRGBA::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

pub struct InstanceBufferManager {
    pub data: Vec<Instance>,
    pub buffer: Buffer,
}

impl InstanceBufferManager {
    pub fn new(max_instances: usize, device: Arc<Mutex<Device>>) -> Self {
        let device = device.lock().unwrap();
        let init_buffer_data = vec![InstanceData::default(); max_instances];
        InstanceBufferManager {
            data: vec![],
            buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("instance buffer"),
                contents: bytemuck::cast_slice(&init_buffer_data),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
        }
    }

    pub fn add_instance(
        &mut self,
        queue: Arc<Mutex<Queue>>,
        transform: ComponentTransform,
        tex_transform: ComponentTransform,
        color: ColorRGBA,
    ) {
        let queue = queue.lock().unwrap();
        let new_data = InstanceData {
            transform: transform.to_mat4(),
            tex_transform: tex_transform.to_mat4(),
            color,
        };
        queue.write_buffer(
            &self.buffer,
            (self.data.len() * size_of::<InstanceData>()) as u64,
            bytemuck::cast_slice(&[new_data]),
        );
        self.data.push(Instance {
            needs_update: false,
            transform,
            tex_transform,
            color,
        });
    }

    pub fn clear(&mut self) {
        // instance.needs_update = false;
        // let queue = queue.lock().unwrap();
        self.data.clear();
    }

    pub fn recalc_screen_instances(&mut self, queue: Arc<Mutex<Queue>>, screen: UVec2) {
        for (instance_index, instance) in self.data.iter_mut().enumerate() {
            if instance.needs_update && instance.transform.pixel_rect.is_some() {
                instance.needs_update = false;
                let queue = queue.lock().unwrap();
                let pr = instance.transform.pixel_rect.unwrap();
                let new_data = InstanceData {
                    transform: ComponentTransform::unit_square_transform_from_pixel_rect(
                        PixelRect {
                            xy: IVec2::new(
                                ((instance.transform.location.x * 0.5 + 0.5) * screen.x as f32)
                                    as i32,
                                ((instance.transform.location.y * 0.5 + 0.5) * screen.y as f32)
                                    as i32,
                            ),
                            wh: pr.wh,
                            extent: screen,
                        },
                    )
                    .to_mat4(),
                    tex_transform: instance.tex_transform.to_mat4(),
                    color: instance.color,
                };
                queue.write_buffer(
                    &self.buffer,
                    (instance_index * size_of::<InstanceData>()) as BufferAddress,
                    bytemuck::cast_slice(&[new_data]),
                );
            }
        }
    }
}

pub struct TextureSheetClusterDefinition {
    pub label: String,
    pub offset: UVec2,
    pub cluster_size: UVec2,
    pub sub_size: UVec2,
    pub spacing: UVec2,
}

impl Default for TextureSheetClusterDefinition {
    fn default() -> Self {
        Self {
            label: "".to_string(),
            offset: UVec2::new(0, 0),
            cluster_size: UVec2::new(1, 1),
            sub_size: UVec2::new(1, 1),
            spacing: UVec2::new(0, 0),
        }
    }
}

pub struct TextureSheetDefinition {
    pub path: String,
    pub clusters: Vec<TextureSheetClusterDefinition>,
}

impl TextureSheetDefinition {
    pub fn none() -> Self {
        Self {
            path: "".to_string(),
            clusters: vec![TextureSheetClusterDefinition::default()],
        }
    }
}

impl Default for TextureSheetDefinition {
    fn default() -> Self {
        Self::none()
    }
}

pub struct TextureSheet {
    pub sheet_info: TextureSheetDefinition,
    pub dimensions: UVec2,
    pub texture: Texture,
    pub sampler: Sampler,
    pub view: TextureView,
}

impl TextureSheet {
    pub fn cluster_sub_transform(
        &self,
        cluster_index: usize,
        sub_index: usize,
    ) -> ComponentTransform {
        let c /*cluster*/ = &self.sheet_info.clusters[cluster_index];
        let rc /*row count*/ = {
            let mut rc = 0;
            for _ in (0..c.cluster_size.x).step_by((c.sub_size.x + c.spacing.x) as usize) {
                rc += 1;
            }
            rc
        };

        let row_index = sub_index as u32 / rc;
        let col_index = sub_index as u32 % rc;

        let x_offset = c.offset.x + col_index * (c.sub_size.x + c.spacing.x);
        let y_offset = c.offset.y + row_index * (c.sub_size.y + c.spacing.y);

        ComponentTransform::tex_transform_from_pixel_rect(PixelRect {
            xy: IVec2::new(x_offset as i32, y_offset as i32),
            wh: c.sub_size,
            extent: self.dimensions,
        })
    }
}

#[derive(Copy, Clone)]
pub struct PixelRect {
    pub xy: IVec2,
    pub wh: UVec2,
    pub extent: UVec2,
}

pub struct ComponentTransform {
    pub pixel_rect: Option<PixelRect>,
    pub location: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for ComponentTransform {
    fn default() -> Self {
        Self {
            pixel_rect: None,
            location: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl ComponentTransform {
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.location)
    }

    pub fn tex_transform_from_pixel_rect(pixel_rect: PixelRect) -> ComponentTransform {
        let xy = Vec2::new(pixel_rect.xy.x as f32, pixel_rect.xy.y as f32);
        let wh = Vec2::new(pixel_rect.wh.x as f32, pixel_rect.wh.y as f32);
        let extent = Vec2::new(pixel_rect.extent.x as f32, pixel_rect.extent.y as f32);

        let location = Vec3::new(xy.x / extent.x, xy.y / extent.y, 0.0);
        let rotation = Quat::IDENTITY;
        let scale = Vec3::new(wh.x / extent.x, wh.y / extent.y, 1.0);

        ComponentTransform {
            pixel_rect: Some(pixel_rect),
            location,
            rotation,
            scale,
        }
    }

    pub fn unit_square_transform_from_pixel_rect(pixel_rect: PixelRect) -> ComponentTransform {
        // given window pixels x, y (top left) of w, h (width, height) produce a transform
        // that positions the UNIT_SQUARE geometry as desired in render space...

        let xy = Vec2::new(pixel_rect.xy.x as f32, pixel_rect.xy.y as f32);
        let wh = Vec2::new(pixel_rect.wh.x as f32, pixel_rect.wh.y as f32);
        let extent = Vec2::new(pixel_rect.extent.x as f32, pixel_rect.extent.y as f32);

        let location = Vec3::new(
            (xy.x / extent.x) * 2.0 - 1.0,
            1.0 - (xy.y / extent.y) * 2.0,
            0.0,
        );
        let rotation = Quat::IDENTITY;
        let scale = Vec3::new((wh.x / extent.x) * 2.0, (wh.y / extent.y) * 2.0, 1.0);

        ComponentTransform {
            pixel_rect: Some(pixel_rect),
            location,
            rotation,
            scale,
        }
    }
}
