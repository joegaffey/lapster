use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use sim_types::{
    AssetManifest, CarVisual, MeshBuffers, TextureRef, TrackCollision, TrackMetadata, TrackVisual,
};

use crate::camera::Camera;

const BACKGROUND_TERRAIN_COLOR: [f32; 3] = [0.12, 0.16, 0.22];

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
    pub wind: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OutlineVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct OutlineMesh {
    pub vertices: Vec<OutlineVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct SceneTexture {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub struct MeshData {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub model: Mat4,
    pub texture: Option<usize>,
    pub grass_texture: Option<usize>,
    pub outline: Option<OutlineMesh>,
    pub alpha: f32,
    pub emissive: [f32; 3],
    pub casts_static_shadow: bool,
    pub casts_dynamic_shadow: bool,
    pub is_decal: bool,
    pub foliage: bool,
    pub billboard: bool,
    /// Runtime-only track material blend flag. Never serialized; `None` means
    /// the mesh does not use the tarmac/shoulder/grass blend.
    pub track_blend: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SceneWheel {
    pub mesh_index: usize,
    pub base_model: Mat4,
    pub is_front: bool,
}

#[derive(Debug, Clone)]
pub struct RigWheel {
    pub mesh_index: usize,
    pub local_model: Mat4,
    pub is_front: bool,
    pub radius: f32,
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CockpitNeedleSource {
    Speed,
    Rpm,
    Gear,
}

#[derive(Debug, Clone)]
pub struct CockpitNeedle {
    pub mesh_index: usize,
    pub center: Vec3,
    pub source: CockpitNeedleSource,
}

#[derive(Debug, Clone)]
pub struct CockpitCover {
    pub mesh_index: usize,
    pub source: CockpitNeedleSource,
}

#[derive(Debug, Clone)]
pub struct CockpitGearSegment {
    pub mesh_index: usize,
    pub center: Vec3,
    pub gear: i32,
}

#[derive(Debug, Clone)]
pub struct CarRig {
    pub body_mesh_index: usize,
    pub body_local: Mat4,
    pub extra_body_meshes: Vec<usize>,
    pub glass_mesh_index: Option<usize>,
    pub brake_light_mesh_index: Option<usize>,
    pub reverse_light_mesh_index: Option<usize>,
    pub dashboard_mesh_index: Option<usize>,
    pub dashboard_local: Mat4,
    pub speedometer_face_mesh_index: Option<usize>,
    pub rpm_face_mesh_index: Option<usize>,
    pub cockpit_needles: Vec<CockpitNeedle>,
    pub cockpit_covers: Vec<CockpitCover>,
    pub gear_segments: Vec<CockpitGearSegment>,
    pub wheels: Vec<RigWheel>,
    pub steering_mesh_index: Option<usize>,
    pub steering_local: Mat4,
    pub decal_mesh_index: Option<usize>,
    pub decal_width: f32,
    pub decal_length: f32,
}

#[derive(Debug, Clone)]
pub struct GarageBay {
    pub wheels: Vec<SceneWheel>,
    pub camera: Camera,
    pub name: String,
    pub car_id: String,
    pub car: serde_json::Value,
    pub color: [f32; 3],
    pub color_name: &'static str,
    pub decal_mesh_index: Option<usize>,
    pub decal_center: [f32; 3],
    pub decal_yaw: f32,
    pub decal_width: f32,
    pub decal_length: f32,
    pub decal_ground_y: f32,
}

#[derive(Debug, Clone, Default)]
pub struct VegetationGroups {
    pub near: Vec<usize>,
    pub mid: Vec<usize>,
    pub far: Vec<usize>,
    pub trees: usize,
    pub bushes: usize,
    pub candidates: usize,
    pub collision_candidates: usize,
    pub visual_candidates: usize,
    pub near_candidates: usize,
    pub mid_candidates: usize,
    pub far_candidates: usize,
    pub min_candidate_distance: f32,
    pub sample_candidate: [f32; 3],
}

pub struct Scene {
    pub meshes: Vec<MeshData>,
    pub textures: Vec<SceneTexture>,
    pub lines: Vec<LineVertex>,
    pub grass: crate::grass::GrassField,
    pub camera: Camera,
    pub debug_info: String,
    pub wheels: Vec<SceneWheel>,
    pub garage_bays: Vec<GarageBay>,
    pub car_mesh_range: Option<Range<usize>>,
    pub stats: SceneStats,
    pub car_rig: Option<CarRig>,
    pub car_rigs: Vec<CarRig>,
    pub vegetation: Option<VegetationGroups>,
    pub mass_foliage: Option<crate::vegetation::MassFoliage>,
    pub style_car_transform: Option<Mat4>,
    pub style_car_bounds: Option<sim_types::Aabb>,
}

#[derive(Debug, Clone, Copy)]
pub struct SceneStats {
    pub vertices: u32,
    pub triangles: u32,
    pub draw_calls: u32,
    pub textures: u32,
    pub collision_triangles: u32,
}

impl Scene {
    pub fn build(
        visual: &TrackVisual,
        metadata: &TrackMetadata,
        collision: &TrackCollision,
        manifest: &AssetManifest,
    ) -> Result<Scene, String> {
        Self::build_internal(visual, metadata, collision, manifest, false)
    }

    pub fn build_style_spike(
        visual: &TrackVisual,
        metadata: &TrackMetadata,
        collision: &TrackCollision,
        manifest: &AssetManifest,
    ) -> Result<Scene, String> {
        Self::build_internal(visual, metadata, collision, manifest, true)
    }

    pub fn build_style_spike_with_car(
        visual: &TrackVisual,
        metadata: &TrackMetadata,
        collision: &TrackCollision,
        manifest: &AssetManifest,
        car_visual: &CarVisual,
    ) -> Result<Scene, String> {
        let mut scene = Self::build_internal(visual, metadata, collision, manifest, true)?;
        let texture_base = scene.textures.len();
        for texture in &car_visual.textures {
            scene.textures.push(scene_texture(texture)?);
        }
        let base_transform = crate::garage::car_transform(metadata)?;
        let (_, rotation, _) = base_transform.to_scale_rotation_translation();
        let forward = rotation * Vec3::NEG_Z;
        let transform = Mat4::from_translation(forward * 24.0) * base_transform;
        let built = crate::garage::build_car(
            &mut scene,
            car_visual,
            texture_base,
            transform,
            [0.78, 0.08, 0.06],
            "style_car",
            true,
            false,
        )?;
        scene.style_car_transform = Some(transform);
        scene.style_car_bounds = Some(built.bounds);
        scene.car_rig = Some(built.rig.clone());
        scene.car_rigs.push(built.rig);
        scene.wheels = built.wheels;
        Ok(scene)
    }

    fn build_internal(
        visual: &TrackVisual,
        metadata: &TrackMetadata,
        collision: &TrackCollision,
        manifest: &AssetManifest,
        style_spike: bool,
    ) -> Result<Scene, String> {
        if visual.meshes.len() != visual.mesh_buffers.len() {
            return Err(format!(
                "track visual has {} mesh refs but {} mesh buffers",
                visual.meshes.len(),
                visual.mesh_buffers.len()
            ));
        }

        let mut textures = visual
            .textures
            .iter()
            .map(scene_texture)
            .collect::<Result<Vec<_>, _>>()?;
        let mut meshes = Vec::with_capacity(visual.meshes.len());
        let mut stats = SceneStats {
            vertices: 0,
            triangles: 0,
            draw_calls: 0,
            textures: textures.len() as u32,
            collision_triangles: (collision.indices.len() / 3) as u32,
        };

        for (mesh_ref, buffers) in visual.meshes.iter().zip(visual.mesh_buffers.iter()) {
            if buffers.positions.is_empty() || buffers.indices.is_empty() {
                continue;
            }
            let material = visual.materials.get(mesh_ref.material as usize);
            let color = material_color(visual, mesh_ref.material as usize);
            let is_background_terrain = mesh_ref.name.to_ascii_lowercase().contains("0grassext");
            let is_grass = !is_background_terrain
                && (mesh_ref.name.to_ascii_lowercase().contains("grass")
                    || material.is_some_and(|material| {
                        material.name.to_ascii_lowercase().contains("grass")
                    }));
            let is_tree_card = mesh_ref.name.starts_with("tree_cards");
            let track_blend = material
                .is_some_and(|material| material.name.to_ascii_lowercase().contains("dither"))
                .then_some(1.0);
            let material_texture = material
                .and_then(|material| material.base_color_texture)
                .map(|index| index as usize);
            if let Some(index) = material_texture {
                if index >= textures.len() {
                    return Err(format!(
                        "mesh {} references missing texture {}",
                        mesh_ref.name, index
                    ));
                }
            }
            let texture = if is_grass { None } else { material_texture };
            let grass_texture = if is_grass { material_texture } else { None };
            let outline = material.map(|material| material.outline).unwrap_or(false);
            let mut vertices = build_vertices(buffers, color);
            if is_background_terrain {
                for vertex in &mut vertices {
                    vertex.color = BACKGROUND_TERRAIN_COLOR;
                }
            } else if is_grass {
                for vertex in &mut vertices {
                    vertex.color = crate::grass::GRASS_GROUND_COLOR;
                }
            }
            for index in &buffers.indices {
                if *index as usize >= vertices.len() {
                    return Err(format!(
                        "mesh {} index {} is outside {} vertices",
                        mesh_ref.name,
                        index,
                        vertices.len()
                    ));
                }
            }
            stats.vertices += vertices.len() as u32;
            stats.triangles += (buffers.indices.len() / 3) as u32;
            stats.draw_calls += 1;
            meshes.push(MeshData {
                name: mesh_ref.name.clone(),
                vertices,
                indices: buffers.indices.clone(),
                model: Mat4::IDENTITY,
                texture,
                grass_texture,
                outline: if outline {
                    build_outline(buffers)
                } else {
                    None
                },
                alpha: material
                    .map(|material| material.base_color[3])
                    .unwrap_or(1.0),
                emissive: [0.0; 3],
                casts_static_shadow: true,
                casts_dynamic_shadow: is_tree_card,
                is_decal: false,
                foliage: is_tree_card,
                billboard: is_tree_card,
                track_blend,
            });
        }

        if !visual
            .meshes
            .iter()
            .any(|mesh| mesh.name.eq_ignore_ascii_case("mountains"))
        {
            if let Some(mountains) = background_mountains(manifest) {
                stats.vertices += mountains.vertices.len() as u32;
                stats.triangles += (mountains.indices.len() / 3) as u32;
                stats.draw_calls += 1;
                meshes.push(mountains);
            }
        }

        if !visual
            .materials
            .iter()
            .any(|material| material.name.to_ascii_lowercase().contains("dither"))
        {
            let fringes = edge_dither_fringes(visual);
            for fringe in fringes {
                let texture_index = textures.len();
                if let Some(color) = fringe.texture_color {
                    textures.push(SceneTexture {
                        width: 1,
                        height: 1,
                        rgba8: vec![color[0], color[1], color[2], 255],
                    });
                } else {
                    textures.push(dither_texture(fringe.color));
                }
                stats.vertices += fringe.vertices.len() as u32;
                stats.triangles += (fringe.indices.len() / 3) as u32;
                stats.draw_calls += 1;
                meshes.push(MeshData {
                    name: fringe.name,
                    vertices: fringe.vertices,
                    indices: fringe.indices,
                    model: Mat4::IDENTITY,
                    texture: Some(texture_index),
                    grass_texture: None,
                    outline: None,
                    alpha: 1.0,
                    emissive: [0.0; 3],
                    casts_static_shadow: false,
                    casts_dynamic_shadow: false,
                    is_decal: false,
                    foliage: false,
                    billboard: false,
                    track_blend: fringe.track_blend,
                });
            }
        }

        stats.textures = textures.len() as u32;

        let mut vegetation = None;
        let mut mass_foliage = None;
        if style_spike {
            let seed = manifest.id.bytes().fold(0x9e37_79b9_u64, |hash, byte| {
                hash.wrapping_mul(0x100_0000_01b3).wrapping_add(byte as u64)
            });
            let built = crate::vegetation::build(visual, metadata, collision, seed);
            let texture_index = textures.len();
            textures.push(built.texture);
            let mut mass = built.mass;
            mass.texture = texture_index;
            let mesh_start = meshes.len();
            let mut groups = VegetationGroups {
                trees: built.trees,
                bushes: built.bushes,
                candidates: built.candidates,
                collision_candidates: built.collision_candidates,
                visual_candidates: built.visual_candidates,
                near_candidates: built.near_candidates,
                mid_candidates: built.mid_candidates,
                far_candidates: built.far_candidates,
                min_candidate_distance: built.min_candidate_distance,
                sample_candidate: built.sample_candidate,
                ..VegetationGroups::default()
            };
            for (index, mut mesh) in built.meshes.into_iter().enumerate() {
                if mesh.texture.is_some() {
                    mesh.texture = Some(texture_index);
                }
                stats.vertices += mesh.vertices.len() as u32;
                stats.triangles += (mesh.indices.len() / 3) as u32;
                stats.draw_calls += 1;
                meshes.push(mesh);
                let scene_index = mesh_start + index;
                if built.tiers[0].contains(&index) {
                    groups.near.push(scene_index);
                } else if built.tiers[1].contains(&index) {
                    groups.mid.push(scene_index);
                } else if built.tiers[2].contains(&index) {
                    groups.far.push(scene_index);
                }
            }
            vegetation = Some(groups);
            mass_foliage = Some(mass);
        }

        let mut lines = Vec::new();
        add_axes(&mut lines);
        add_bounds(&mut lines, manifest);
        add_spawn(&mut lines, metadata);
        add_collision(&mut lines, collision);

        let camera = build_camera(metadata);
        let grass = crate::grass::GrassField::new(collision, metadata);
        let debug_info = json!({
            "asset": { "id": manifest.id, "kind": "Track" },
            "stats": {
                "vertices": stats.vertices,
                "triangles": stats.triangles,
                "drawCalls": stats.draw_calls,
                "textures": stats.textures,
                "texturedMeshes": meshes.iter().filter(|mesh| mesh.texture.is_some()).count(),
                "outlinedMeshes": meshes.iter().filter(|mesh| mesh.outline.is_some()).count(),
                "collisionTriangles": stats.collision_triangles,
                "grassBlades": grass.len(),
                "grassChunks": grass.chunks().len(),
            },
            "bounds": { "min": manifest.bounds.min, "max": manifest.bounds.max },
            "surfaces": metadata.surfaces.iter().map(|surface| json!({
                "id": surface.id,
                "name": surface.name,
                "friction": surface.friction,
                "roughness": surface.roughness,
            })).collect::<Vec<_>>(),
            "spawns": metadata.spawns.iter().map(|spawn| json!({
                "name": spawn.name,
                "position": spawn.position,
                "rotation": spawn.rotation_xyzw,
            })).collect::<Vec<_>>(),
            "cameras": metadata.cameras.iter().map(|camera| json!({
                "name": camera.name,
                "position": camera.position,
                "target": camera.target,
                "fovY": camera.fov_y,
            })).collect::<Vec<_>>(),
            "meshes": visual.meshes.iter().map(|mesh| json!({
                "name": mesh.name,
                "vertices": mesh.vertex_count,
                "triangles": mesh.index_count / 3,
                "material": mesh.material,
            })).collect::<Vec<_>>(),
            "vegetation": vegetation.as_ref().map(|groups| json!({
                "trees": groups.trees,
                "bushes": groups.bushes,
                "candidates": groups.candidates,
                "collisionCandidates": groups.collision_candidates,
                "visualCandidates": groups.visual_candidates,
                "nearCandidates": groups.near_candidates,
                "midCandidates": groups.mid_candidates,
                "farCandidates": groups.far_candidates,
                "minCandidateDistance": groups.min_candidate_distance,
                "sampleCandidate": groups.sample_candidate,
                "nearMeshes": groups.near.len(),
                "midMeshes": groups.mid.len(),
                "farMeshes": groups.far.len(),
            })),
            "massFoliage": mass_foliage.as_ref().map(|mass| json!({
                "trees": {
                    "instances": mass.tree_instances.len(),
                    "layers": mass.tree_layer_ends.len(),
                    "layerEnds": mass.tree_layer_ends,
                },
                "bushes": {
                    "instances": mass.bush_instances.len(),
                    "layers": mass.bush_layer_ends.len(),
                    "layerEnds": mass.bush_layer_ends,
                },
            })),
            "warnings": manifest.warnings,
        })
        .to_string();

        let camera = if style_spike {
            style_camera(metadata).unwrap_or(camera)
        } else {
            camera
        };

        Ok(Scene {
            meshes,
            textures,
            lines,
            grass,
            camera,
            debug_info,
            wheels: Vec::new(),
            garage_bays: Vec::new(),
            car_mesh_range: None,
            stats,
            car_rig: None,
            car_rigs: Vec::new(),
            vegetation,
            mass_foliage,
            style_car_transform: None,
            style_car_bounds: None,
        })
    }
}

fn style_camera(metadata: &TrackMetadata) -> Option<Camera> {
    let spawn = metadata.spawns.first()?;
    let position = Vec3::from_array(spawn.position);
    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    );
    let rotation = if rotation.length_squared() > 1e-6 {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    };
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    Some(Camera {
        eye: (position + Vec3::Y * 9.5 - forward * 24.0 + right * 17.0).to_array(),
        target: (position + forward * 20.0 + Vec3::Y * 2.0).to_array(),
        fov_y: 0.9,
    })
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mountain_noise(x: f32, z: f32) -> f32 {
    let ridge = (x * 0.0031 + z * 0.0017).sin() * 0.45
        + (x * 0.0073 - z * 0.0051).sin() * 0.28
        + (x * 0.0157 + z * 0.0113).sin() * 0.17
        + (x * 0.0290 + z * 0.0230).sin() * 0.10;
    (ridge * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn mountain_height_at(
    x: f32,
    z: f32,
    center_x: f32,
    center_z: f32,
    rise_start: f32,
    outer_radius: f32,
) -> f32 {
    let radius = ((x - center_x).powi(2) + (z - center_z).powi(2)).sqrt();
    let t = ((radius - rise_start) / (outer_radius - rise_start)).clamp(0.0, 1.0);
    let envelope = smoothstep(0.0, 0.25, t) * (1.0 - smoothstep(0.88, 1.0, t));
    envelope * (25.0 + 110.0 * mountain_noise(x, z).powf(1.35))
}

fn background_mountains(manifest: &AssetManifest) -> Option<MeshData> {
    let bounds = manifest.bounds;
    if !bounds.min[0].is_finite()
        || !bounds.min[2].is_finite()
        || !bounds.max[0].is_finite()
        || !bounds.max[2].is_finite()
    {
        return None;
    }
    let center_x = (bounds.min[0] + bounds.max[0]) * 0.5;
    let center_z = (bounds.min[2] + bounds.max[2]) * 0.5;
    let half_x = (bounds.max[0] - bounds.min[0]) * 0.5;
    let half_z = (bounds.max[2] - bounds.min[2]) * 0.5;
    let inner_radius = (half_x.min(half_z) * 0.75).max(50.0);
    let rise_start = half_x.max(half_z) + 70.0;
    let outer_radius = rise_start + 3400.0;
    let base_y = bounds.min[1] - 0.5;
    let segments = 128usize;
    let rings = 24usize;
    let row = segments + 1;
    let mut vertices = Vec::with_capacity(row * (rings + 1));
    let mut indices = Vec::with_capacity(segments * rings * 6);
    for ring in 0..=rings {
        let radial = ring as f32 / rings as f32;
        let radius = inner_radius + (outer_radius - inner_radius) * radial.powf(1.6);
        for segment in 0..=segments {
            let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
            let x = center_x + radius * angle.cos();
            let z = center_z + radius * angle.sin();
            let rise = mountain_height_at(x, z, center_x, center_z, rise_start, outer_radius);
            let epsilon = 20.0;
            let dx =
                (mountain_height_at(x + epsilon, z, center_x, center_z, rise_start, outer_radius)
                    - mountain_height_at(
                        x - epsilon,
                        z,
                        center_x,
                        center_z,
                        rise_start,
                        outer_radius,
                    ))
                    / (epsilon * 2.0);
            let dz =
                (mountain_height_at(x, z + epsilon, center_x, center_z, rise_start, outer_radius)
                    - mountain_height_at(
                        x,
                        z - epsilon,
                        center_x,
                        center_z,
                        rise_start,
                        outer_radius,
                    ))
                    / (epsilon * 2.0);
            let normal = Vec3::new(-dx, 1.0, -dz).normalize_or_zero();
            let rock = smoothstep(8.0, 50.0, rise);
            let color = Vec3::from_array([0.12, 0.20, 0.09])
                .lerp(Vec3::from_array([0.07, 0.10, 0.14]), rock);
            vertices.push(MeshVertex {
                position: [x, base_y + rise, z],
                normal: normal.to_array(),
                color: color.to_array(),
                uv: [segment as f32 / segments as f32, radial],
                wind: [0.0, 0.0],
            });
        }
    }
    for ring in 0..rings {
        for segment in 0..segments {
            let base = (ring * row + segment) as u32;
            let next = ((ring + 1) * row + segment) as u32;
            indices.extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
        }
    }
    Some(MeshData {
        name: String::from("mountains"),
        vertices,
        indices,
        model: Mat4::IDENTITY,
        texture: None,
        grass_texture: None,
        outline: None,
        alpha: 1.0,
        emissive: [0.0; 3],
        casts_static_shadow: false,
        casts_dynamic_shadow: false,
        is_decal: false,
        foliage: false,
        billboard: false,
        track_blend: None,
    })
}

fn surface_priority(name: &str) -> Option<u8> {
    match name.strip_prefix("surface_").unwrap_or(name) {
        "asphalt" => Some(3),
        "dirt" => Some(2),
        "grass" => Some(1),
        _ => None,
    }
}

fn position_key(position: [f32; 3]) -> [i64; 3] {
    [
        (position[0] * 1000.0).round() as i64,
        (position[1] * 1000.0).round() as i64,
        (position[2] * 1000.0).round() as i64,
    ]
}

type EdgeKey = (i64, i64, i64, i64, i64, i64);

fn edge_key(a: [f32; 3], b: [f32; 3]) -> EdgeKey {
    let first = position_key(a);
    let second = position_key(b);
    if first <= second {
        (
            first[0], first[1], first[2], second[0], second[1], second[2],
        )
    } else {
        (
            second[0], second[1], second[2], first[0], first[1], first[2],
        )
    }
}

struct EdgeSides {
    a: [f32; 3],
    b: [f32; 3],
    thirds: HashMap<usize, [f32; 3]>,
}

fn linear_to_srgb(value: f32) -> f32 {
    let clamped = value.clamp(0.0, 1.0);
    if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    }
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn material_average_color(visual: &TrackVisual, material_index: usize) -> [u8; 3] {
    let Some(material) = visual.materials.get(material_index) else {
        return [255, 255, 255];
    };
    if let Some(texture) = material
        .base_color_texture
        .and_then(|index| visual.textures.get(index as usize))
    {
        if !texture.rgba8.is_empty() {
            let mut sum = [0u64; 3];
            let pixels = texture.rgba8.len() / 4;
            for pixel in texture.rgba8.chunks_exact(4) {
                sum[0] += pixel[0] as u64;
                sum[1] += pixel[1] as u64;
                sum[2] += pixel[2] as u64;
            }
            let pixels = pixels.max(1) as u64;
            return [
                (sum[0] / pixels) as u8,
                (sum[1] / pixels) as u8,
                (sum[2] / pixels) as u8,
            ];
        }
    }
    let color = material_color(visual, material_index);
    [
        (linear_to_srgb(color[0]) * 255.0).round() as u8,
        (linear_to_srgb(color[1]) * 255.0).round() as u8,
        (linear_to_srgb(color[2]) * 255.0).round() as u8,
    ]
}

struct DitherFringe {
    name: String,
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
    color: [u8; 3],
    texture_color: Option<[u8; 3]>,
    track_blend: Option<f32>,
}

const DITHER_STRIP_HALF_WIDTH: f32 = 0.3;
const DITHER_FALLBACK_WIDTH: f32 = 0.45;
const DITHER_LIFT: f32 = 0.02;
const DITHER_ASPHALT_DIRT_UV: [f32; 2] = [2.96, 3.04];
const DITHER_DIRT_GRASS_UV: [f32; 2] = [3.81, 3.91];

fn srgb_byte_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn edge_dither_fringes(visual: &TrackVisual) -> Vec<DitherFringe> {
    let mut surfaces = Vec::new();
    for (mesh_ref, buffers) in visual.meshes.iter().zip(visual.mesh_buffers.iter()) {
        let Some(priority) = surface_priority(&mesh_ref.name) else {
            continue;
        };
        if buffers.positions.is_empty() || buffers.indices.is_empty() {
            continue;
        }
        surfaces.push((
            mesh_ref.name.clone(),
            buffers,
            priority,
            material_average_color(visual, mesh_ref.material as usize),
        ));
    }
    if surfaces.len() < 2 {
        return Vec::new();
    }
    let mut edges: HashMap<EdgeKey, EdgeSides> = HashMap::new();
    for (surface_index, (_, buffers, _, _)) in surfaces.iter().enumerate() {
        for triangle in buffers.indices.chunks_exact(3) {
            let indices = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            for edge in 0..3 {
                let a = indices[edge];
                let b = indices[(edge + 1) % 3];
                let c = indices[(edge + 2) % 3];
                let a_position = buffers.positions[a];
                let b_position = buffers.positions[b];
                let key = edge_key(a_position, b_position);
                let entry = edges.entry(key).or_insert_with(|| EdgeSides {
                    a: a_position,
                    b: b_position,
                    thirds: HashMap::new(),
                });
                entry
                    .thirds
                    .entry(surface_index)
                    .or_insert(buffers.positions[c]);
            }
        }
    }

    let mut groups: HashMap<(usize, usize), DitherFringe> = HashMap::new();
    for sides in edges.values() {
        let candidates = sides.thirds.iter().collect::<Vec<_>>();
        for first in 0..candidates.len() {
            for second in (first + 1)..candidates.len() {
                let (first_index, first_third) = candidates[first];
                let (second_index, second_third) = candidates[second];
                let first_priority = surfaces[*first_index].2;
                let second_priority = surfaces[*second_index].2;
                let (source, target, target_third) = if first_priority > second_priority {
                    (*first_index, *second_index, *second_third)
                } else if second_priority > first_priority {
                    (*second_index, *first_index, *first_third)
                } else {
                    continue;
                };
                let a = Vec3::from_array(sides.a);
                let b = Vec3::from_array(sides.b);
                let midpoint = (a + b) * 0.5;
                let edge = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
                let toward = Vec3::from_array(target_third) - midpoint;
                let toward = Vec3::new(toward.x, 0.0, toward.z);
                if edge.length_squared() < 1e-8 || toward.length_squared() < 1e-8 {
                    continue;
                }
                let mut direction = Vec3::new(-edge.z, 0.0, edge.x);
                if direction.dot(toward) < 0.0 {
                    direction = -direction;
                }
                let direction = direction.normalize();
                let lift = Vec3::Y * DITHER_LIFT;
                let supported_band = match (surfaces[source].2, surfaces[target].2) {
                    (3, 2) => Some((DITHER_ASPHALT_DIRT_UV, target)),
                    (2, 1) => Some((DITHER_DIRT_GRASS_UV, source)),
                    _ => None,
                };
                let (
                    dirt_surface,
                    uv_inner,
                    uv_outer,
                    texture_color,
                    vertex_color,
                    track_blend,
                    centered,
                ) = if let Some((uv, dirt_surface)) = supported_band {
                    let dirt = surfaces[dirt_surface].3.map(srgb_byte_to_linear);
                    let texel = surfaces[source].3.map(srgb_byte_to_linear);
                    let vertex_color = [
                        (dirt[0] / (0.75 + 0.25 * texel[0])).clamp(0.0, 1.0),
                        (dirt[1] / (0.75 + 0.25 * texel[1])).clamp(0.0, 1.0),
                        (dirt[2] / (0.75 + 0.25 * texel[2])).clamp(0.0, 1.0),
                    ];
                    (
                        dirt_surface,
                        uv[0],
                        uv[1],
                        Some(surfaces[source].3),
                        vertex_color,
                        Some(1.0),
                        true,
                    )
                } else {
                    (source, 0.0, 0.0, None, [1.0, 1.0, 1.0], None, false)
                };
                let points = if centered {
                    let inner_side = -direction * DITHER_STRIP_HALF_WIDTH;
                    let outer_side = direction * DITHER_STRIP_HALF_WIDTH;
                    [
                        a + inner_side + lift,
                        b + inner_side + lift,
                        b + outer_side + lift,
                        a + outer_side + lift,
                    ]
                } else {
                    [
                        a + lift,
                        b + lift,
                        b + direction * DITHER_FALLBACK_WIDTH + lift,
                        a + direction * DITHER_FALLBACK_WIDTH + lift,
                    ]
                };
                let face_normal = (points[1] - points[0]).cross(points[2] - points[0]);
                let order = if face_normal.y >= 0.0 {
                    [0u32, 1, 2, 0, 2, 3]
                } else {
                    [0u32, 2, 1, 0, 3, 2]
                };
                let key = (source, target);
                let group_color = if centered {
                    surfaces[dirt_surface].3
                } else {
                    surfaces[source].3
                };
                let fringe = groups.entry(key).or_insert_with(|| DitherFringe {
                    name: format!("dither_{}_to_{}", surfaces[source].0, surfaces[target].0),
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    color: group_color,
                    texture_color,
                    track_blend,
                });
                let base = fringe.vertices.len() as u32;
                for (index, point) in points.into_iter().enumerate() {
                    let uv = if centered {
                        let uv_y = if index < 2 { uv_inner } else { uv_outer };
                        [point.x / 7.0, uv_y]
                    } else {
                        [point.x / 1.5, point.z / 1.5]
                    };
                    fringe.vertices.push(MeshVertex {
                        position: point.to_array(),
                        normal: [0.0, 1.0, 0.0],
                        color: vertex_color,
                        uv,
                        wind: [0.0, 0.0],
                    });
                }
                fringe
                    .indices
                    .extend(order.into_iter().map(|index| base + index));
            }
        }
    }
    groups.into_values().collect()
}

fn dither_texture(color: [u8; 3]) -> SceneTexture {
    let size = 64u32;
    let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let hash = (x / 2)
                .wrapping_mul(0x9e37_79b1)
                .wrapping_add((y / 2).wrapping_mul(0x85eb_ca77))
                ^ 0x27d4_eb2f;
            let alpha = if hash % 5 < 2 { 0 } else { 255 };
            rgba8.extend_from_slice(&[color[0], color[1], color[2], alpha]);
        }
    }
    SceneTexture {
        width: size,
        height: size,
        rgba8,
    }
}

pub(crate) fn build_vertices(buffers: &MeshBuffers, color: [f32; 3]) -> Vec<MeshVertex> {
    buffers
        .positions
        .iter()
        .enumerate()
        .map(|(index, position)| MeshVertex {
            position: *position,
            normal: buffers
                .normals
                .get(index)
                .copied()
                .unwrap_or([0.0, 1.0, 0.0]),
            color: buffers.colors.get(index).copied().unwrap_or(color),
            uv: buffers.uvs.get(index).copied().unwrap_or([0.0, 0.0]),
            wind: [0.0, 0.0],
        })
        .collect()
}

pub(crate) fn scene_texture(texture: &TextureRef) -> Result<SceneTexture, String> {
    let expected = (texture.width as usize)
        .checked_mul(texture.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("texture {} dimensions overflow", texture.name))?;
    if texture.width == 0 || texture.height == 0 {
        return Err(format!("texture {} has zero dimensions", texture.name));
    }
    if texture.rgba8.len() != expected {
        return Err(format!(
            "texture {} has {} bytes; expected {expected}",
            texture.name,
            texture.rgba8.len()
        ));
    }
    Ok(SceneTexture {
        width: texture.width,
        height: texture.height,
        rgba8: texture.rgba8.clone(),
    })
}

pub(crate) fn build_outline(buffers: &MeshBuffers) -> Option<OutlineMesh> {
    if buffers.positions.is_empty() || buffers.indices.is_empty() {
        return None;
    }
    let mut welded = Vec::with_capacity(buffers.positions.len());
    let mut sums = Vec::<[f32; 3]>::with_capacity(buffers.positions.len());
    let mut map = HashMap::with_capacity(buffers.positions.len());
    let mut remap = Vec::with_capacity(buffers.positions.len());

    for (index, position) in buffers.positions.iter().enumerate() {
        let key = (
            (position[0] * 10_000.0).round() as i64,
            (position[1] * 10_000.0).round() as i64,
            (position[2] * 10_000.0).round() as i64,
        );
        let welded_index = *map.entry(key).or_insert_with(|| {
            welded.push(OutlineVertex {
                position: *position,
                normal: [0.0, 1.0, 0.0],
            });
            sums.push([0.0; 3]);
            welded.len() - 1
        });
        let normal = buffers
            .normals
            .get(index)
            .copied()
            .unwrap_or([0.0, 1.0, 0.0]);
        sums[welded_index][0] += normal[0];
        sums[welded_index][1] += normal[1];
        sums[welded_index][2] += normal[2];
        remap.push(welded_index as u32);
    }

    for (vertex, sum) in welded.iter_mut().zip(sums.iter()) {
        let length = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
        if length > 1e-6 {
            vertex.normal = [sum[0] / length, sum[1] / length, sum[2] / length];
        }
    }

    let indices = buffers
        .indices
        .iter()
        .map(|index| remap.get(*index as usize).copied().unwrap_or(0))
        .collect();
    Some(OutlineMesh {
        vertices: welded,
        indices,
    })
}

pub(crate) fn material_color(visual: &TrackVisual, index: usize) -> [f32; 3] {
    visual
        .materials
        .get(index)
        .map(|material| {
            [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ]
        })
        .unwrap_or([1.0, 1.0, 1.0])
}

fn build_camera(metadata: &TrackMetadata) -> Camera {
    if let Some(camera) = metadata
        .cameras
        .iter()
        .find(|camera| camera.name == "start")
        .or_else(|| metadata.cameras.first())
    {
        return Camera {
            eye: camera.position,
            target: camera.target,
            fov_y: camera.fov_y,
        };
    }

    let spawn = metadata
        .spawns
        .first()
        .map(|spawn| spawn.position)
        .unwrap_or([0.0; 3]);
    Camera {
        eye: [spawn[0], spawn[1] + 6.0, spawn[2] + 14.0],
        target: [spawn[0], spawn[1] + 0.5, spawn[2] - 10.0],
        fov_y: 0.9,
    }
}

fn add_axes(lines: &mut Vec<LineVertex>) {
    let origin = [0.0, 0.05, 0.0];
    push_line(lines, origin, [10.0, 0.05, 0.0], [0.9, 0.15, 0.15]);
    push_line(lines, origin, [0.0, 10.0, 0.0], [0.15, 0.9, 0.15]);
    push_line(lines, origin, [0.0, 0.05, -10.0], [0.15, 0.35, 1.0]);
}

fn add_bounds(lines: &mut Vec<LineVertex>, manifest: &AssetManifest) {
    let min = manifest.bounds.min;
    let max = manifest.bounds.max;
    let color = [0.55, 0.6, 0.7];
    let corners = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], min[1], max[2]],
        [min[0], min[1], max[2]],
        [min[0], max[1], min[2]],
        [max[0], max[1], min[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (from, to) in edges {
        push_line(lines, corners[from], corners[to], color);
    }
}

fn add_spawn(lines: &mut Vec<LineVertex>, metadata: &TrackMetadata) {
    let Some(spawn) = metadata.spawns.first() else {
        return;
    };
    let origin = [
        spawn.position[0],
        spawn.position[1] + 0.05,
        spawn.position[2],
    ];
    let color = [1.0, 0.85, 0.2];
    push_line(
        lines,
        [origin[0] - 2.0, origin[1], origin[2]],
        [origin[0] + 2.0, origin[1], origin[2]],
        color,
    );
    push_line(
        lines,
        [origin[0], origin[1], origin[2] - 2.0],
        [origin[0], origin[1], origin[2] + 2.0],
        color,
    );
    push_line(
        lines,
        [origin[0], origin[1], origin[2]],
        [origin[0], origin[1] + 2.0, origin[2]],
        color,
    );

    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    );
    let rotation = if rotation.length_squared() > 1e-6 {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    };
    let forward = rotation * Vec3::new(0.0, 0.0, -5.0);
    push_line(
        lines,
        origin,
        [
            origin[0] + forward.x,
            origin[1] + forward.y,
            origin[2] + forward.z,
        ],
        color,
    );
}

fn add_collision(lines: &mut Vec<LineVertex>, collision: &TrackCollision) {
    let triangle_count = collision.indices.len() / 3;
    for triangle in 0..triangle_count {
        let surface = collision.surface_ids.get(triangle).copied().unwrap_or(0);
        let color = surface_color(surface);
        let base = triangle * 3;
        let points = [
            collision_point(collision, collision.indices[base]),
            collision_point(collision, collision.indices[base + 1]),
            collision_point(collision, collision.indices[base + 2]),
        ];
        push_line(lines, points[0], points[1], color);
        push_line(lines, points[1], points[2], color);
        push_line(lines, points[2], points[0], color);
    }
}

fn collision_point(collision: &TrackCollision, index: u32) -> [f32; 3] {
    let point = collision
        .vertices
        .get(index as usize)
        .copied()
        .unwrap_or([0.0; 3]);
    [point[0], point[1] + 0.02, point[2]]
}

fn surface_color(surface: u16) -> [f32; 3] {
    match surface {
        0 => [0.2, 0.65, 1.0],
        1 => [1.0, 0.3, 0.2],
        2 => [1.0, 0.9, 0.2],
        3 => [0.8, 0.3, 1.0],
        4 => [0.2, 1.0, 0.7],
        5 => [0.2, 0.55, 0.7],
        6 => [0.7, 0.35, 0.2],
        7 => [0.55, 0.55, 1.0],
        8 => [1.0, 0.55, 0.1],
        9 => [0.2, 1.0, 0.35],
        10 => [1.0, 0.1, 0.9],
        _ => [0.9, 0.9, 0.9],
    }
}

pub(crate) fn push_line(
    lines: &mut Vec<LineVertex>,
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 3],
) {
    lines.push(LineVertex {
        position: from,
        color,
    });
    lines.push(LineVertex {
        position: to,
        color,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_types::MeshRef;

    #[test]
    fn outline_welds_duplicate_positions() {
        let buffers = MeshBuffers {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 6],
            uvs: vec![[0.0, 0.0]; 6],
            colors: Vec::new(),
            indices: vec![0, 1, 2, 3, 4, 5],
        };
        let outline = build_outline(&buffers).expect("outline");
        assert_eq!(outline.vertices.len(), 3);
        assert_eq!(outline.indices, vec![0, 1, 2, 0, 1, 2]);
        assert_eq!(outline.vertices[0].normal, [0.0, 0.0, 1.0]);
    }

    fn test_material(name: &str, base_color: [f32; 4]) -> sim_types::MaterialRef {
        sim_types::MaterialRef {
            name: String::from(name),
            base_color,
            metallic: 0.0,
            roughness: 0.9,
            base_color_texture: None,
            outline: false,
        }
    }

    fn test_visual(surfaces: Vec<(&str, MeshBuffers, sim_types::MaterialRef)>) -> TrackVisual {
        let mut meshes = Vec::new();
        let mut materials = Vec::new();
        let mut mesh_buffers = Vec::new();
        for (name, buffers, material) in surfaces {
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for position in &buffers.positions {
                for axis in 0..3 {
                    min[axis] = min[axis].min(position[axis]);
                    max[axis] = max[axis].max(position[axis]);
                }
            }
            meshes.push(MeshRef {
                name: String::from(name),
                vertex_offset: 0,
                vertex_count: buffers.positions.len() as u32,
                index_offset: 0,
                index_count: buffers.indices.len() as u32,
                material: materials.len() as u16,
                bounds: sim_types::Aabb { min, max },
                lod: 0,
            });
            mesh_buffers.push(buffers);
            materials.push(material);
        }
        TrackVisual {
            meshes,
            materials,
            textures: Vec::new(),
            mesh_buffers,
            instances: Vec::new(),
        }
    }

    fn test_quad(x0: f32, x1: f32) -> MeshBuffers {
        MeshBuffers {
            positions: vec![
                [x0, 0.0, -1.0],
                [x1, 0.0, -1.0],
                [x1, 0.0, 1.0],
                [x0, 0.0, 1.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0, 0.0]; 4],
            colors: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    fn x_span(vertices: &[MeshVertex]) -> f32 {
        let min = vertices
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::INFINITY, f32::min);
        let max = vertices
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        max - min
    }

    #[test]
    fn edge_dither_fringes_use_centered_shader_band() {
        let visual = test_visual(vec![
            (
                "surface_asphalt",
                test_quad(-1.0, 0.0),
                test_material("asphalt", [0.045, 0.045, 0.05, 1.0]),
            ),
            (
                "surface_dirt",
                test_quad(0.0, 1.0),
                test_material("dirt", [0.48, 0.345, 0.208, 1.0]),
            ),
        ]);
        let fringes = edge_dither_fringes(&visual);
        assert_eq!(fringes.len(), 1);
        let fringe = &fringes[0];
        assert_eq!(fringe.name, "dither_surface_asphalt_to_surface_dirt");
        assert_eq!(fringe.track_blend, Some(1.0));
        assert!(fringe.texture_color.is_some());
        assert_eq!(fringe.vertices.len(), 4);
        assert_eq!(fringe.indices.len(), 6);
        assert!((x_span(&fringe.vertices) - DITHER_STRIP_HALF_WIDTH * 2.0).abs() < 1e-5);
        assert!((fringe.vertices[0].uv[1] - DITHER_ASPHALT_DIRT_UV[0]).abs() < 1e-5);
        assert!((fringe.vertices[2].uv[1] - DITHER_ASPHALT_DIRT_UV[1]).abs() < 1e-5);
    }

    #[test]
    fn edge_dither_fringes_ignore_parallel_target_vertex() {
        let dirt = MeshBuffers {
            positions: vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [0.05, 0.0, 1.9]],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            uvs: vec![[0.0, 0.0]; 3],
            colors: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let visual = test_visual(vec![
            (
                "surface_asphalt",
                test_quad(-1.0, 0.0),
                test_material("asphalt", [0.045, 0.045, 0.05, 1.0]),
            ),
            (
                "surface_dirt",
                dirt,
                test_material("dirt", [0.48, 0.345, 0.208, 1.0]),
            ),
        ]);
        let fringes = edge_dither_fringes(&visual);
        assert_eq!(fringes.len(), 1);
        let width = x_span(&fringes[0].vertices);
        assert!(
            (width - DITHER_STRIP_HALF_WIDTH * 2.0).abs() < 1e-4,
            "dither strip must stay perpendicular to the seam, got {width} m"
        );
    }

    #[test]
    fn edge_dither_fringes_use_dirt_to_grass_band() {
        let visual = test_visual(vec![
            (
                "surface_dirt",
                test_quad(-1.0, 0.0),
                test_material("dirt", [0.48, 0.345, 0.208, 1.0]),
            ),
            (
                "surface_grass",
                test_quad(0.0, 1.0),
                test_material("grass", [0.302, 0.431, 0.208, 1.0]),
            ),
        ]);
        let fringes = edge_dither_fringes(&visual);
        assert_eq!(fringes.len(), 1);
        let fringe = &fringes[0];
        assert_eq!(fringe.name, "dither_surface_dirt_to_surface_grass");
        assert_eq!(fringe.track_blend, Some(1.0));
        assert!((fringe.vertices[0].uv[1] - DITHER_DIRT_GRASS_UV[0]).abs() < 1e-5);
        assert!((fringe.vertices[2].uv[1] - DITHER_DIRT_GRASS_UV[1]).abs() < 1e-5);
    }

    #[test]
    fn edge_dither_fringes_fall_back_for_unhandled_pairs() {
        let visual = test_visual(vec![
            (
                "surface_asphalt",
                test_quad(-1.0, 0.0),
                test_material("asphalt", [0.045, 0.045, 0.05, 1.0]),
            ),
            (
                "surface_grass",
                test_quad(0.0, 1.0),
                test_material("grass", [0.302, 0.431, 0.208, 1.0]),
            ),
        ]);
        let fringes = edge_dither_fringes(&visual);
        assert_eq!(fringes.len(), 1);
        let fringe = &fringes[0];
        assert_eq!(fringe.name, "dither_surface_asphalt_to_surface_grass");
        assert_eq!(fringe.track_blend, None);
        assert_eq!(fringe.texture_color, None);
        assert_eq!(fringe.vertices.len(), 4);
        assert_eq!(fringe.indices.len(), 6);
        assert!((fringe.vertices[0].uv[1] - (-1.0 / 1.5)).abs() < 1e-5);
    }

    #[test]
    fn rejects_texture_with_wrong_byte_count() {
        let texture = TextureRef {
            name: String::from("bad"),
            mime: String::from("image/x-rgba8"),
            hash: [0; 32],
            width: 2,
            height: 2,
            rgba8: vec![0; 15],
        };
        assert!(scene_texture(&texture).is_err());
    }
}
