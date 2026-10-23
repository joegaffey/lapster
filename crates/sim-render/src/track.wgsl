struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
@group(2) @binding(0) var base_texture: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
}

struct MeshOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_position: vec3<f32>,
}

@vertex
fn vs_mesh(input: MeshIn) -> MeshOut {
    var output: MeshOut;
    let world = model * vec4<f32>(input.position, 1.0);
    output.clip_position = camera.view_proj * world;
    output.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    output.color = input.color;
    output.uv = input.uv;
    output.world_position = world.xyz;
    return output;
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let lower = color * 12.92;
    let higher = 1.055 * pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, color <= cutoff);
}

fn toon_ramp(ndl: f32) -> f32 {
    if ndl > 0.72 {
        return 1.0;
    }
    if ndl > 0.42 {
        return 0.78;
    }
    if ndl > 0.18 {
        return 0.56;
    }
    return 0.40;
}

@fragment
fn fs_mesh(input: MeshOut) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light = normalize(-camera.light_dir.xyz);
    let ndl = max(dot(normal, light), 0.0);
    let ramp = toon_ramp(ndl);
    let texel = textureSample(base_texture, base_sampler, input.uv);
    if texel.a < 0.1 {
        discard;
    }
    let base = input.color * texel.rgb;
    let view_dir = normalize(camera.camera_pos.xyz - input.world_position);
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    let rim_tint = vec3<f32>(0.55, 0.72, 0.95);
    let color = base * (0.30 + 0.70 * ramp) + rim_tint * rim * 0.16;
    return vec4<f32>(linear_to_srgb(color), 1.0);
}

struct OutlineIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct OutlineOut {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_outline(input: OutlineIn) -> OutlineOut {
    let world = model * vec4<f32>(input.position, 1.0);
    let world_normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    let distance = length(camera.camera_pos.xyz - world.xyz);
    let width = 0.022 * clamp(distance / 12.0, 0.65, 2.5);
    var output: OutlineOut;
    output.clip_position =
        camera.view_proj * vec4<f32>(world.xyz + world_normal * width, 1.0);
    return output;
}

@fragment
fn fs_outline() -> @location(0) vec4<f32> {
    return vec4<f32>(linear_to_srgb(vec3<f32>(0.004, 0.005, 0.008)), 1.0);
}

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct LineOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_line(input: LineIn) -> LineOut {
    var output: LineOut;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_line(input: LineOut) -> @location(0) vec4<f32> {
    return vec4<f32>(linear_to_srgb(input.color), 1.0);
}
