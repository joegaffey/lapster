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

struct MassParams {
    time: f32,
    max_distance: f32,
    wind_strength: f32,
    pad: f32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> mass_params: MassParams;
@group(2) @binding(0) var leaf_texture: texture_2d<f32>;
@group(2) @binding(1) var leaf_sampler: sampler;

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

struct MassIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) center_kind: vec4<f32>,
    @location(3) shape: vec4<f32>,
    @location(4) tint_phase: vec4<f32>,
    @location(5) cluster_wind: vec4<f32>,
};

struct MassOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) kind: f32,
};

@vertex
fn vs_mass(input: MassIn) -> MassOut {
    var output: MassOut;
    let kind = input.center_kind.w;
    let yaw = input.shape.z;
    let tilt = input.shape.w;
    let right = vec3<f32>(cos(yaw), 0.0, -sin(yaw));
    let up = normalize(vec3<f32>(
        sin(yaw) * sin(tilt),
        cos(tilt),
        cos(yaw) * sin(tilt),
    ));
    var local = right * (input.position.x * input.shape.x)
        + up * (input.position.y * input.shape.y);
    let center = input.center_kind.xyz;
    let distance_xz = distance(center.xz, camera.camera_pos.xz);
    let distance_fade = 1.0 - smoothstep(
        mass_params.max_distance * 0.82,
        mass_params.max_distance,
        distance_xz,
    );
    let near_fade = smoothstep(1.0, 3.5, distance_xz);
    let center_clip = camera.view_proj * vec4<f32>(center, 1.0);
    let behind_fade = smoothstep(0.0, 10.0, center_clip.w);
    let fade = distance_fade * near_fade * behind_fade;
    local *= fade;
    var world = center + local;
    if kind < 0.5 {
        let weight = input.cluster_wind.w * (0.5 + input.uv.y * 0.5);
        let phase = input.tint_phase.w;
        let sway = sin(
            mass_params.time * 1.3 + phase + world.x * 0.07 + world.z * 0.05
        );
        let gust = sin(mass_params.time * 0.7 + phase * 1.7 + world.z * 0.03);
        world += vec3<f32>(sway * 0.11 + gust * 0.05, gust * 0.03, sway * 0.07)
            * weight * mass_params.wind_strength * fade;
    }
    let leaf_normal = normalize(
        world - input.cluster_wind.xyz + vec3<f32>(0.0, 0.28, 0.0)
    );
    let trunk_normal = normalize(vec3<f32>(sin(yaw), 0.0, cos(yaw)));
    output.clip_position = camera.view_proj * vec4<f32>(world, 1.0);
    output.uv = input.uv;
    output.tint = input.tint_phase.rgb;
    output.world = world;
    output.normal = select(leaf_normal, trunk_normal, kind > 0.5);
    output.kind = kind;
    return output;
}

@fragment
fn fs_mass(input: MassOut) -> @location(0) vec4<f32> {
    let texel = textureSample(leaf_texture, leaf_sampler, input.uv);
    if input.kind < 0.5 && texel.a < 0.45 {
        discard;
    }
    var base = input.tint;
    if input.kind < 0.5 {
        base = input.tint * texel.rgb;
    }
    let normal = normalize(input.normal);
    let light = normalize(-camera.light_dir.xyz);
    let ndl = max(dot(normal, light), 0.0);
    let ramp = toon_ramp(ndl);
    let view_dir = normalize(camera.camera_pos.xyz - input.world);
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    var color = base * (0.30 + 0.70 * ramp);
    if input.kind < 0.5 {
        let back = pow(max(dot(view_dir, -light), 0.0), 2.0);
        let edge = 1.0 - abs(dot(normal, view_dir));
        color += vec3<f32>(0.10, 0.20, 0.035) * back * edge * 0.85;
        color += vec3<f32>(0.42, 0.62, 0.18) * rim * 0.30;
    } else {
        color += vec3<f32>(0.55, 0.72, 0.95) * rim * 0.10;
    }
    return vec4<f32>(linear_to_srgb(color), 1.0);
}
