struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    light_view_proj: mat4x4<f32>,
    shadow_params: vec4<f32>,
    sky_horizon: vec4<f32>,
    sky_zenith: vec4<f32>,
    baked_view_proj: mat4x4<f32>,
    baked_shadow_params: vec4<f32>,
    shadow_modes: vec4<f32>,
    car_shadow_params: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    particle_params: vec4<f32>,
    particle_shade: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

struct ParticleIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) center_scale: vec4<f32>,
    @location(3) rotation: vec4<f32>,
    @location(4) color_alpha: vec4<f32>,
};

struct ParticleOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) alpha: f32,
    @location(3) world: vec3<f32>,
};

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let lower = color * 12.92;
    let higher = 1.055 * pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, color <= cutoff);
}

fn rotate2d(rotation: vec4<f32>, value: vec2<f32>) -> vec2<f32> {
    let z = rotation.z;
    let w = rotation.w;
    let cos_angle = 1.0 - 2.0 * z * z;
    let sin_angle = 2.0 * w * z;
    return vec2<f32>(
        value.x * cos_angle - value.y * sin_angle,
        value.x * sin_angle + value.y * cos_angle,
    );
}

@vertex
fn vs_particle(input: ParticleIn) -> ParticleOut {
    var output: ParticleOut;
    let rotation = normalize(input.rotation);
    let local = rotate2d(rotation, input.position.xy) * input.center_scale.w;
    let world = input.center_scale.xyz
        + camera.camera_right.xyz * local.x
        + camera.camera_up.xyz * local.y;
    output.position = camera.view_proj * vec4<f32>(world, 1.0);
    output.uv = input.uv;
    output.color = input.color_alpha.rgb;
    output.alpha = input.color_alpha.a;
    output.world = world;
    return output;
}

@fragment
fn fs_particle(input: ParticleOut) -> @location(0) vec4<f32> {
    let radial = 1.0 - smoothstep(camera.particle_params.z, camera.particle_params.w, length(input.uv));
    let near = smoothstep(camera.particle_params.x, camera.particle_params.y, distance(camera.camera_pos.xyz, input.world));
    let alpha = input.alpha * radial * near;
    let shade = camera.particle_shade.x + (camera.particle_shade.y - camera.particle_shade.x) * clamp(input.uv.y, 0.0, 1.0);
    return vec4<f32>(linear_to_srgb(input.color * shade), alpha);
}
