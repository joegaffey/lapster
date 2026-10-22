struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct MeshOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_mesh(input: MeshIn) -> MeshOut {
    var output: MeshOut;
    output.clip_position = camera.view_proj * model * vec4<f32>(input.position, 1.0);
    output.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    output.color = input.color;
    return output;
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let lower = color * 12.92;
    let higher = 1.055 * pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, color <= cutoff);
}

@fragment
fn fs_mesh(input: MeshOut) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light = normalize(-camera.light_dir.xyz);
    let diffuse = max(dot(normal, light), 0.0);
    let ambient = 0.28;
    let color = input.color * (ambient + 0.72 * diffuse);
    return vec4<f32>(linear_to_srgb(color), 1.0);
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
