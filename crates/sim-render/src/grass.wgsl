// Stylized grass blade pass adapted from Stylized Components grassField (MIT).
// Source: https://github.com/cortiz2894/stylized-components
// See THIRD_PARTY_NOTICES.md.

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
};

struct Grass {
    time: f32,
    wind_strength: f32,
    wind_speed: f32,
    wind_freq: f32,
    brightness: f32,
    translucency: f32,
    center_x: f32,
    center_z: f32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_sampler: sampler_comparison;
@group(1) @binding(2) var baked_shadow_map: texture_depth_2d;
@group(2) @binding(0) var<uniform> grass: Grass;
@group(3) @binding(0) var<uniform> chunk_fade: vec4<f32>;

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

fn hash(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(127.1, 311.7));
    q += dot(q, q + 19.19);
    return fract(q.x * q.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var total = 0.0;
    var point = p;
    for (var i = 0; i < 4; i = i + 1) {
        v += a * noise(point);
        total += a;
        point = point * 2.03 + vec2<f32>(3.1, 7.7);
        a *= 0.5;
    }
    return v / max(total, 0.001);
}

fn sample_baked_shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if camera.shadow_params.z <= 0.0 {
        return 1.0;
    }
    let world_texel = camera.baked_shadow_params.y;
    let offset_position = world_position + normal * clamp(world_texel * 1.5, 0.03, 0.20);
    let clip = camera.baked_view_proj * vec4<f32>(offset_position, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 {
        return 1.0;
    }
    let depth = ndc.z - camera.baked_shadow_params.z;
    let offset = camera.baked_shadow_params.x * 0.75;
    var lit = 0.0;
    lit += textureSampleCompareLevel(baked_shadow_map, shadow_sampler, uv + vec2<f32>(-offset, -offset), depth);
    lit += textureSampleCompareLevel(baked_shadow_map, shadow_sampler, uv + vec2<f32>(offset, -offset), depth);
    lit += textureSampleCompareLevel(baked_shadow_map, shadow_sampler, uv + vec2<f32>(-offset, offset), depth);
    lit += textureSampleCompareLevel(baked_shadow_map, shadow_sampler, uv + vec2<f32>(offset, offset), depth);
    return mix(1.0, lit / 4.0, camera.shadow_params.z);
}

fn dynamic_shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if camera.shadow_params.z <= 0.0 {
        return 1.0;
    }
    let world_texel = camera.shadow_params.y;
    let offset_position = world_position + normal * clamp(world_texel * 1.5, 0.03, 0.20);
    let clip = camera.light_view_proj * vec4<f32>(offset_position, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 {
        return 1.0;
    }
    let depth = ndc.z - camera.shadow_params.w;
    let offset = camera.shadow_params.x * 1.5;
    var lit = 0.0;
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(-offset, -offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(offset, -offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(-offset, offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(offset, offset), depth);
    return mix(1.0, lit / 4.0, camera.shadow_params.z);
}

fn near_shadow_weight(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let world_texel = camera.shadow_params.y;
    let offset_position = world_position + normal * clamp(world_texel * 1.5, 0.03, 0.20);
    let clip = camera.light_view_proj * vec4<f32>(offset_position, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 {
        return 0.0;
    }
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return smoothstep(0.0, 0.12, edge);
}

fn grass_shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if camera.shadow_modes.w > 0.5 {
        let dynamic = dynamic_shadow(world_position, normal);
        let baked = sample_baked_shadow(world_position, normal);
        let weight = near_shadow_weight(world_position, normal);
        return mix(baked, dynamic, weight);
    }
    if camera.shadow_modes.x > 0.5 {
        return dynamic_shadow(world_position, normal);
    }
    if camera.shadow_modes.y > 0.5 {
        return sample_baked_shadow(world_position, normal);
    }
    return 1.0;
}

struct BladeIn {
    @location(0) position: vec3<f32>,
    @location(1) base: vec4<f32>,
    @location(2) shape: vec4<f32>,
};

struct BladeOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) height: f32,
    @location(2) blade_normal: vec3<f32>,
    @location(3) variation: f32,
    @location(4) fade: f32,
};

@vertex
fn vs_grass(input: BladeIn) -> BladeOut {
    var output: BladeOut;
    let yaw = input.base.w;
    let height = input.shape.y;
    let base = input.base.xyz;
    let camera_distance = distance(base.xz, camera.camera_pos.xz);
    let center_distance = distance(base.xz, vec2<f32>(grass.center_x, grass.center_z));
    let fade = (1.0 - smoothstep(58.0, 86.0, center_distance)) * chunk_fade.x;
    let wide = 1.0 + smoothstep(10.0, 45.0, camera_distance) * 2.0;
    let taller = 1.0 + smoothstep(15.0, 45.0, camera_distance) * 0.6;
    let y = input.position.y * height * fade * taller;
    let x = input.position.x * input.shape.x * wide;
    let c = cos(yaw);
    let s = sin(yaw);
    let rotated = vec3<f32>(x * c, y, -x * s);

    let blade_height = input.position.y;
    let bend_mask = blade_height * blade_height;
    let wind_dir = normalize(vec2<f32>(0.78, 0.38));
    let phase = input.shape.w * 6.2831853;
    let wave = sin(dot(base.xz, wind_dir) * grass.wind_freq + grass.time * grass.wind_speed + phase);
    let wave2 = sin(
        dot(base.xz, wind_dir) * grass.wind_freq * 2.4
            + grass.time * grass.wind_speed * 1.7
            + phase * 2.1
    ) * 0.35;
    let gust = (wave + wave2) * grass.wind_strength * bend_mask;
    let wind = vec3<f32>(wind_dir.x, 0.0, wind_dir.y) * gust;
    let lean = vec3<f32>(cos(yaw), 0.0, -sin(yaw)) * (input.shape.z * bend_mask);
    let world = base + rotated + wind + lean;

    output.clip_position = camera.view_proj * vec4<f32>(world, 1.0);
    output.world_position = world;
    output.height = blade_height;
    output.blade_normal = vec3<f32>(s, 0.0, c);
    output.variation = hash(base.xz * 0.75);
    output.fade = fade;
    return output;
}

@fragment
fn fs_grass(input: BladeOut) -> @location(0) vec4<f32> {
    let normal = vec3<f32>(0.0, 1.0, 0.0);
    let light = normalize(-camera.light_dir.xyz);
    let ndl = max(dot(normal, light), 0.0);
    let ramp = toon_ramp(ndl);
    let shadow = grass_shadow(input.world_position, normal);

    let bottom = vec3<f32>(0.048, 0.111, 0.006);
    let top = vec3<f32>(0.114, 0.207, 0.023);
    let dry = vec3<f32>(0.12, 0.195, 0.023);
    let gradient = pow(clamp(input.height, 0.0, 1.0), 1.25);
    var color = mix(bottom, top, gradient);

    let patch_value = fbm(input.world_position.xz * 0.045);
    color = mix(color, dry, smoothstep(0.58, 0.82, patch_value) * 0.35);
    color *= 0.86 + input.variation * 0.28;
    color *= grass.brightness * (0.34 + 0.66 * ramp * shadow);

    let ground_color = vec3<f32>(0.048, 0.111, 0.006) * (0.30 + 0.70 * ramp * shadow);
    color = mix(ground_color, color, input.fade);

    let view_dir = normalize(camera.camera_pos.xyz - input.world_position);
    let back = pow(max(dot(view_dir, -light), 0.0), 3.0);
    let edge = 1.0 - abs(dot(normalize(input.blade_normal), light));
    let tip = mix(0.25, 1.0, input.height);
    let trans_color = vec3<f32>(0.12, 0.21, 0.03);
    color += trans_color * grass.translucency * back * edge * tip * shadow * input.fade;

    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    color += vec3<f32>(0.21, 0.30, 0.075) * rim * 0.10;

    return vec4<f32>(linear_to_srgb(color), 1.0);
}
