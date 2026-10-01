// Colors are a per-material uniform rather than vertex attributes: with
// default render options, every vertex of a RenderCommand shares them.

#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
}

struct SpineColors {
    light: vec4<f32>,
    dark: vec4<f32>,
}

@group(2) @binding(0) var<uniform> colors: SpineColors;
@group(2) @binding(1) var spine_texture: texture_2d<f32>;
@group(2) @binding(2) var spine_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local, vec4<f32>(v.position, 1.0));
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(spine_texture, spine_sampler, in.uv);
    // Tint-black over a PMA sample. `colors.light` is straight alpha, so
    // `light.a` scales only the output alpha.
    let rgb = ((sample.rgb - vec3<f32>(1.0)) * colors.dark.rgb + sample.rgb) * colors.light.rgb;
    let a = sample.a * colors.light.a;
    return vec4<f32>(rgb, a);
}
