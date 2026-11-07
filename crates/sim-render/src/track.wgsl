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

struct Model {
    transform: mat4x4<f32>,
    alpha: vec4<f32>,
    emissive: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: Model;
const GRASS_TEXTURE_GAIN: f32 = 0.75;
const GRASS_TEXTURE_LIFT: f32 = 1.0;
@group(2) @binding(0) var base_texture: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;
@group(3) @binding(0) var shadow_map: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;
@group(3) @binding(2) var baked_shadow_map: texture_depth_2d;

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let lower = color * 12.92;
    let higher = 1.055 * pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, color <= cutoff);
}

struct SkyOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    var output: SkyOut;
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    output.clip_position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    output.uv = corner;
    return output;
}

@fragment
fn fs_sky(input: SkyOut) -> @location(0) vec4<f32> {
    let warmth = clamp(camera.sky_horizon.w, 0.0, 1.0);
    let height = pow(clamp(input.uv.y, 0.0, 1.0), mix(0.8, 2.2, warmth));
    let horizon = camera.sky_horizon.xyz;
    let zenith = camera.sky_zenith.xyz;
    var sky = mix(horizon, zenith, height);
    let dusk_band = smoothstep(0.0, 0.7, height) * (1.0 - smoothstep(0.45, 1.0, height));
    sky = mix(sky, vec3<f32>(0.62, 0.18, 0.34), dusk_band * warmth * 0.45);
    return vec4<f32>(linear_to_srgb(sky), 1.0);
}

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) wind: vec2<f32>,
}

struct MeshOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) alpha: f32,
}

@vertex
fn vs_mesh(input: MeshIn) -> MeshOut {
    var output: MeshOut;
    var world = model.transform * vec4<f32>(input.position, 1.0);
    var out_normal = normalize((model.transform * vec4<f32>(input.normal, 0.0)).xyz);
    var out_color = input.color;
    if model.alpha.z > 1.5 {
        let width = input.normal.x;
        let height = input.normal.y;
        let corner = input.color.xy;
        let tint = input.color.z;
        let to_camera = normalize(camera.camera_pos.xyz - world.xyz);
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_camera));
        let up = normalize(cross(to_camera, right));
        world = vec4<f32>(
            world.xyz + right * (corner.x * width) + up * (corner.y * height),
            1.0,
        );
        out_normal = vec3<f32>(0.0, 1.0, 0.0);
        out_color = vec3<f32>(tint, tint, tint);
    } else if model.alpha.z > 0.5 {
        let phase = input.wind.y;
        let weight = input.wind.x;
        let sway = sin(
            model.emissive.w * 1.35 + phase + world.x * 0.07 + world.z * 0.05
        );
        let gust = sin(model.emissive.w * 0.7 + phase * 1.7 + world.z * 0.03);
        world = vec4<f32>(
            world.xyz + vec3<f32>(sway * 0.11 + gust * 0.05, gust * 0.03, sway * 0.07) * weight,
            1.0,
        );
    }
    output.clip_position = camera.view_proj * world;
    output.normal = out_normal;
    output.color = out_color;
    output.uv = input.uv;
    output.world_position = world.xyz;
    output.alpha = model.alpha.x;
    return output;
}

@vertex
fn vs_shadow(input: MeshIn) -> @builtin(position) vec4<f32> {
    let world = model.transform * vec4<f32>(input.position, 1.0);
    return camera.light_view_proj * world;
}

struct ShadowCardOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_shadow_card(input: MeshIn) -> ShadowCardOut {
    var world = model.transform * vec4<f32>(input.position, 1.0);
    let width = input.normal.x;
    let height = input.normal.y;
    let corner = input.color.xy;
    let to_camera = normalize(camera.camera_pos.xyz - world.xyz);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_camera));
    let up = normalize(cross(to_camera, right));
    world = vec4<f32>(
        world.xyz + right * (corner.x * width) + up * (corner.y * height),
        1.0,
    );
    var output: ShadowCardOut;
    output.clip_position = camera.light_view_proj * world;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_shadow_card(input: ShadowCardOut) {
    let texel = textureSample(base_texture, base_sampler, input.uv);
    if texel.a < 0.45 {
        discard;
    }
}

fn bayer4(coord: vec2<f32>) -> f32 {
    let x = u32(coord.x) & 3u;
    let y = u32(coord.y) & 3u;
    var threshold = array<f32, 16>(
        0.0625, 0.5625, 0.1875, 0.6875,
        0.8125, 0.3125, 0.9375, 0.4375,
        0.2500, 0.7500, 0.1250, 0.6250,
        1.0000, 0.5000, 0.8750, 0.3750,
    );
    return threshold[x + y * 4u];
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

fn track_band_color(
    edge: f32,
    inner: vec3<f32>,
    outer: vec3<f32>,
    center: f32,
    half_width: f32,
    mode: f32,
    pixel: vec2<f32>,
) -> vec3<f32> {
    let start = center - half_width;
    let end = center + half_width;
    if edge <= start {
        return inner;
    }
    if edge >= end {
        return outer;
    }
    let t = (edge - start) / (end - start);
    if mode > 2.5 {
        return select(inner, outer, t > 0.5);
    }
    if mode > 1.5 {
        return mix(inner, outer, smoothstep(0.0, 1.0, t));
    }
    return select(inner, outer, t > bayer4(pixel));
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

fn car_shadow_visibility(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
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
    let offset = camera.shadow_params.x * 4.0;
    let taps = array<vec2<f32>, 32>(
        vec2<f32>(0.1250, 0.0000), vec2<f32>(-0.1596, 0.1462),
        vec2<f32>(0.0244, -0.2784), vec2<f32>(0.2012, 0.2625),
        vec2<f32>(-0.3693, -0.0653), vec2<f32>(0.3498, -0.2225),
        vec2<f32>(-0.1170, 0.4352), vec2<f32>(-0.2231, -0.4296),
        vec2<f32>(0.4841, 0.1768), vec2<f32>(-0.5036, 0.2079),
        vec2<f32>(0.2428, -0.5188), vec2<f32>(0.1794, 0.5720),
        vec2<f32>(-0.5408, -0.3134), vec2<f32>(0.6344, -0.1395),
        vec2<f32>(-0.3871, 0.5507), vec2<f32>(-0.0894, -0.6902),
        vec2<f32>(0.5491, 0.4628), vec2<f32>(-0.7389, 0.0306),
        vec2<f32>(0.5390, -0.5363), vec2<f32>(-0.0361, 0.7798),
        vec2<f32>(-0.5128, -0.6145), vec2<f32>(0.8124, 0.1093),
        vec2<f32>(-0.6883, 0.4789), vec2<f32>(0.1881, -0.8361),
        vec2<f32>(0.4350, 0.7592), vec2<f32>(-0.8504, -0.2713),
        vec2<f32>(0.8261, -0.3817), vec2<f32>(-0.3579, 0.8552),
        vec2<f32>(-0.3194, -0.8880), vec2<f32>(0.8499, 0.4467),
        vec2<f32>(-0.9440, 0.2488), vec2<f32>(0.5366, -0.8345),
    );
    var lit = 0.0;
    for (var index = 0; index < 32; index = index + 1) {
        lit += textureSampleCompareLevel(
            shadow_map,
            shadow_sampler,
            uv + taps[index] * offset,
            depth,
        );
    }
    var fade = 1.0;
    if camera.car_shadow_params.w > camera.car_shadow_params.z {
        let distance_from_car = distance(
            world_position.xz,
            camera.car_shadow_params.xy,
        );
        fade = 1.0 - smoothstep(
            camera.car_shadow_params.z,
            camera.car_shadow_params.w,
            distance_from_car,
        );
    }
    return mix(1.0, lit / 32.0, camera.shadow_params.z * fade);
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

fn shadow_visibility(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
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
    let offset = camera.shadow_params.x * 0.75;
    var lit = 0.0;
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(-offset, -offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(offset, -offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(-offset, offset), depth);
    lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(offset, offset), depth);
    return mix(1.0, lit / 4.0, camera.shadow_params.z);
}

@fragment
fn fs_mesh(input: MeshOut) -> @location(0) vec4<f32> {
    let foliage = model.alpha.z > 0.5;
    let normal = normalize(input.normal);
    let light = normalize(-camera.light_dir.xyz);
    let ndl = max(dot(normal, light), 0.0);
    let ramp = toon_ramp(ndl);
    let texel = textureSample(base_texture, base_sampler, input.uv);
    let alpha = input.alpha * texel.a;
    let cutoff = select(0.01, 0.45, foliage);
    if alpha < cutoff {
        discard;
    }
    var base = input.color * texel.rgb;
    if model.alpha.z > 1.5 {
        return vec4<f32>(linear_to_srgb(base), 1.0);
    }
    let blend_mode = model.alpha.w;
    if blend_mode > 0.5 {
        let asphalt = vec3<f32>(0.045, 0.045, 0.05);
        let grass = vec3<f32>(0.048, 0.111, 0.006);
        let edge = abs(input.uv.y * 2.0 - 1.0);
        let pixel = input.clip_position.xy;
        let soft = blend_mode > 1.5;
        let asphalt_half_width = select(0.02, 0.06, soft);
        let grass_half_width = select(0.025, 0.07, soft);
        base = track_band_color(
            edge,
            asphalt,
            input.color,
            0.5,
            asphalt_half_width,
            blend_mode,
            pixel,
        );
        base = track_band_color(
            edge,
            base,
            grass,
            0.93,
            grass_half_width,
            blend_mode,
            pixel,
        );
    }
    if model.alpha.y > 0.5 {
        base = texel.rgb * GRASS_TEXTURE_GAIN;
    }
    var environment_shadow = 1.0;
    if camera.shadow_modes.w > 0.5 {
        let dynamic = shadow_visibility(input.world_position, normal);
        let baked = sample_baked_shadow(input.world_position, normal);
        let weight = near_shadow_weight(input.world_position, normal);
        environment_shadow = mix(baked, dynamic, weight);
    } else if camera.shadow_modes.x > 0.5 {
        environment_shadow = shadow_visibility(input.world_position, normal);
    } else if camera.shadow_modes.y > 0.5 {
        environment_shadow = sample_baked_shadow(input.world_position, normal);
    }
    var car_shadow = 1.0;
    if camera.shadow_modes.z > 0.5 {
        car_shadow = car_shadow_visibility(input.world_position, normal);
    }
    let shadow = environment_shadow * car_shadow;
    let view_dir = normalize(camera.camera_pos.xyz - input.world_position);
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    let rim_tint = select(
        vec3<f32>(0.55, 0.72, 0.95),
        vec3<f32>(0.42, 0.62, 0.18),
        foliage,
    );
    var color =
        base * (0.30 + 0.70 * ramp * shadow)
        + rim_tint * rim * select(0.16, 0.30, foliage)
        + model.emissive.xyz;
    if model.alpha.y > 0.5 {
        color = color * GRASS_TEXTURE_LIFT + vec3<f32>(0.008, 0.008, 0.015);
    }
    if foliage {
        let back = pow(max(dot(view_dir, -light), 0.0), 2.0);
        let edge = 1.0 - abs(dot(normal, view_dir));
        color += vec3<f32>(0.10, 0.20, 0.035) * back * edge * 0.85;
    }
    return vec4<f32>(linear_to_srgb(color), select(alpha, 1.0, foliage));
}

@fragment
fn fs_decal(input: MeshOut) -> @location(0) vec4<f32> {
    let texel = textureSample(base_texture, base_sampler, input.uv);
    let alpha = input.alpha * texel.a;
    if alpha < 0.01 {
        discard;
    }
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
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
    let world = model.transform * vec4<f32>(input.position, 1.0);
    let world_normal = normalize((model.transform * vec4<f32>(input.normal, 0.0)).xyz);
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
