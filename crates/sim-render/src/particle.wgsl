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
    @location(5) shape: vec4<f32>,
};

struct ParticleOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) alpha: f32,
    @location(3) world: vec3<f32>,
    @location(4) shape: vec4<f32>,
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
    output.shape = input.shape;
    return output;
}

@fragment
fn fs_particle(input: ParticleOut) -> @location(0) vec4<f32> {
    let near = smoothstep(camera.particle_params.x, camera.particle_params.y, distance(camera.camera_pos.xyz, input.world));
    let t = clamp((input.uv.y + 1.0) * 0.5, 0.0, 1.0);
    let shade = camera.particle_shade.x + (camera.particle_shade.y - camera.particle_shade.x) * t;

    if input.shape.x > 0.5 {
        let variant = max(input.shape.y, 0.01);
        let curve = (t * t - 0.35) * 0.12 * variant;
        let x = input.uv.x - curve;
        let half_width = 0.24 * pow(1.0 - t, 1.2) * variant;
        let blade = 1.0 - smoothstep(half_width - 0.018, half_width, abs(x));
        let tip = 1.0 - smoothstep(0.94, 1.0, t);
        let alpha = input.alpha * blade * tip * near;
        let grass_bottom = vec3<f32>(0.032, 0.074, 0.004);
        let grass_top = vec3<f32>(0.076, 0.138, 0.015);
        let blade_color = mix(grass_bottom, grass_top, pow(t, 1.25)) * input.color * shade;
        return vec4<f32>(linear_to_srgb(blade_color), alpha);
    }

    let radial = 1.0 - smoothstep(camera.particle_params.z, camera.particle_params.w, length(input.uv));
    let alpha = input.alpha * radial * near;
    return vec4<f32>(linear_to_srgb(input.color * shade), alpha);
}
