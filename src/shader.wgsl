@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> screen_size: vec2<f32>;
@group(0) @binding(2)
var texture: texture_2d<f32>;
@group(0) @binding(3)
var texture_sampler: sampler;

struct InstanceInput {
    @location(5) transform_0: vec4<f32>,
    @location(6) transform_1: vec4<f32>,
    @location(7) transform_2: vec4<f32>,
    @location(8) transform_3: vec4<f32>,
    @location(9) tex_transform_0: vec4<f32>,
    @location(10) tex_transform_1: vec4<f32>,
    @location(11) tex_transform_2: vec4<f32>,
    @location(12) tex_transform_3: vec4<f32>,
    @location(13) color: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    vin: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let transform = mat4x4<f32>(
        instance.transform_0,
        instance.transform_1,
        instance.transform_2,
        instance.transform_3,
    );
    let tex_transform = mat4x4<f32>(
        instance.tex_transform_0,
        instance.tex_transform_1,
        instance.tex_transform_2,
        instance.tex_transform_3,
    );
    var result: VertexOutput;
    result.position = view * transform * vec4(vin.position, 1.0);
    result.tex_coords = (tex_transform * vec4(vin.tex_coords, 0.0, 1.0)).xy;
    result.color = instance.color;
    return result;
}

@fragment
fn fs_main(vout: VertexOutput) -> @location(0) vec4<f32> {
    var color = vout.color * textureSample(texture, texture_sampler, vout.tex_coords);
    color = vec4(color.rgb * color.a, color.a);
    return color;
}
