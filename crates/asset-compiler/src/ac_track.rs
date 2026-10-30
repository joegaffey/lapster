use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use glam::{Mat3, Mat4, Quat, Vec3};
use image::imageops::FilterType;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sim_types::{
    Aabb, CameraSpec, MaterialRef, MeshBuffers, MeshRef, SpawnPoint, SurfaceSpec, TextureRef,
    TrackCollision, TrackMetadata, TrackVisual,
};

use crate::kn5::Kn5;
use crate::mesh::{bounds, merge_bounds};

pub struct ImportedTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub bounds: Aabb,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum UvMode {
    #[default]
    Mesh,
    WorldXz,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextureOverride {
    texture: String,
    #[serde(default)]
    uv_scale: Option<f32>,
    #[serde(default)]
    uv_mode: UvMode,
}

struct LoadedTextureOverride {
    name: String,
    bytes: Vec<u8>,
    uv_scale: Option<f32>,
    uv_mode: UvMode,
    cache_key: String,
}

#[derive(Debug, Deserialize)]
struct UiTrack {
    name: Option<String>,
    length: Option<String>,
}

pub fn import_track(dir: &Path) -> Result<ImportedTrack> {
    import_track_scaled(dir, 1.0)
}

pub fn import_track_scaled(dir: &Path, scale: f32) -> Result<ImportedTrack> {
    import_track_scaled_with_overrides(dir, scale, None)
}

pub fn import_track_scaled_with_overrides(
    dir: &Path,
    scale: f32,
    overrides_path: Option<&Path>,
) -> Result<ImportedTrack> {
    if !scale.is_finite() || scale <= 0.0 {
        bail!("track scale must be a positive finite number, got {scale}");
    }
    let surfaces_path = find_file(dir, "surfaces.ini")
        .with_context(|| format!("no surfaces.ini found under {}", dir.display()))?;
    let ui_path = find_file(dir, "ui_track.json");
    let ui = ui_path
        .as_deref()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<UiTrack>(&bytes).ok());
    let (surfaces, surface_ids) = parse_surfaces(&surfaces_path)?;
    let mut warnings = vec![
        "KN5 base-colour DDS textures are imported at a maximum of 512px; normal and detail maps are ignored".to_string(),
        "alpha-tested trees and fences are skipped".to_string(),
        "collision includes the central pad, outer ring, and grass; walls/fences remain visual-only"
            .to_string(),
    ];
    let mut texture_cache: HashMap<String, u32> = HashMap::new();
    let texture_overrides = overrides_path
        .map(load_texture_overrides)
        .transpose()?
        .unwrap_or_default();

    let kn5_paths = find_kn5_files(dir)?;
    if kn5_paths.is_empty() {
        bail!("no KN5 files found under {}", dir.display());
    }
    let mut models = Vec::with_capacity(kn5_paths.len());
    for path in &kn5_paths {
        models.push((path.clone(), Kn5::load(path)?));
    }

    let spawn_node = find_spawn(&models);
    let Some((spawn_name, spawn_world)) = spawn_node else {
        bail!("no AC_START_* node found");
    };
    let (_, spawn_rotation, spawn_translation) = spawn_world.to_scale_rotation_translation();
    let spawn_rotation = spawn_rotation.normalize();
    let scale_transform = Mat4::from_scale(Vec3::splat(scale));
    let spawn_translation = scale_transform.transform_point3(spawn_translation);

    let mut visual = TrackVisual {
        meshes: Vec::new(),
        materials: Vec::new(),
        textures: Vec::new(),
        mesh_buffers: Vec::new(),
        instances: Vec::new(),
    };
    let mut collision = TrackCollision {
        vertices: Vec::new(),
        indices: Vec::new(),
        surface_ids: Vec::new(),
        bounds: empty_bounds(),
    };
    let mut visual_bounds = None;
    let mut collision_bounds = None;

    for (path, model) in &models {
        let texture_bytes = model
            .textures
            .iter()
            .map(|texture| (texture.name.as_str(), texture.bytes.as_slice()))
            .collect::<HashMap<_, _>>();
        let material_offset = visual.materials.len() as u16;
        for material in &model.materials {
            let texture_override = texture_overrides.get(&material.name);
            let texture_name = if let Some(override_texture) = texture_override {
                Some(override_texture.name.as_str())
            } else if material.shader == "ksMultilayer_fresnel_nm"
                && texture_bytes.contains_key("tracknew.DDS")
            {
                Some("tracknew.DDS")
            } else {
                material.textures.get("txDiffuse").map(String::as_str)
            };
            let base_color_texture = if skip_material(&material.name) {
                None
            } else if let Some(override_texture) = texture_override {
                Some(import_texture(
                    &mut visual.textures,
                    &mut texture_cache,
                    path,
                    &override_texture.name,
                    &override_texture.bytes,
                    512,
                    Some(&override_texture.cache_key),
                )?)
            } else if texture_skipped(&material.name) {
                None
            } else {
                texture_name
                    .and_then(|name| texture_bytes.get(name))
                    .map(|bytes| {
                        import_texture(
                            &mut visual.textures,
                            &mut texture_cache,
                            path,
                            texture_name.unwrap(),
                            bytes,
                            512,
                            None,
                        )
                    })
                    .transpose()?
            };
            let base_color = if base_color_texture.is_some() {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                let color = material_color(&material.name);
                [color[0], color[1], color[2], 1.0]
            };
            visual.materials.push(MaterialRef {
                name: material.name.clone(),
                base_color,
                metallic: 0.0,
                roughness: 0.7,
                base_color_texture,
                outline: material_outline(&material.name),
            });
        }

        for (node, world) in model.walk() {
            let Some(mesh) = &node.mesh else {
                continue;
            };
            if !node.active || node.name.starts_with("AC_") {
                continue;
            }
            let Some(material) = model.materials.get(mesh.material as usize) else {
                bail!(
                    "{} node {} references missing material {}",
                    path.display(),
                    node.name,
                    mesh.material
                );
            };
            if skip_material(&material.name) {
                continue;
            }
            let buffers = transform_mesh(mesh, scale_transform * world);
            let mut visual_buffers = buffers.clone();
            let texture_override = texture_overrides.get(&material.name);
            if let Some(override_texture) = texture_override {
                let uv_scale = override_texture.uv_scale.unwrap_or(1.0);
                match override_texture.uv_mode {
                    UvMode::Mesh => {
                        if (uv_scale - 1.0).abs() > 1e-6 {
                            for uv in &mut visual_buffers.uvs {
                                uv[0] *= uv_scale;
                                uv[1] *= uv_scale;
                            }
                        }
                    }
                    UvMode::WorldXz => {
                        for (uv, position) in
                            visual_buffers.uvs.iter_mut().zip(&visual_buffers.positions)
                        {
                            uv[0] = position[0] * uv_scale;
                            uv[1] = position[2] * uv_scale;
                        }
                    }
                }
            } else {
                let uv_scale = material_uv_scale(&material.name);
                if (uv_scale - 1.0).abs() > 1e-6 {
                    for uv in &mut visual_buffers.uvs {
                        uv[0] *= uv_scale;
                        uv[1] *= uv_scale;
                    }
                }
            }
            let mut visual_offset = visual_y_offset(&node.name) * scale;
            if file_stem(path).eq_ignore_ascii_case("terrain") {
                visual_offset -= 0.05 * scale;
            }
            if visual_offset != 0.0 {
                for position in &mut visual_buffers.positions {
                    position[1] += visual_offset;
                }
            }
            let mesh_bounds = bounds(&visual_buffers);
            visual_bounds = Some(merge_bounds(visual_bounds, &mesh_bounds));
            visual.meshes.push(mesh_ref(
                &format!("{}/{}", file_stem(path), node.name),
                &visual_buffers,
                material_offset + mesh.material as u16,
            ));
            visual.mesh_buffers.push(visual_buffers);

            if let Some(key) = collision_surface_key(&node.name) {
                let surface = surface_ids.get(key).copied().unwrap_or(0);
                let base = collision.vertices.len() as u32;
                collision.vertices.extend_from_slice(&buffers.positions);
                collision
                    .indices
                    .extend(buffers.indices.iter().map(|index| base + index));
                for _ in 0..buffers.indices.len() / 3 {
                    collision.surface_ids.push(surface);
                }
                collision_bounds = Some(merge_bounds(collision_bounds, &mesh_bounds));
            }
        }
    }

    if visual.meshes.is_empty() {
        bail!("no visual meshes imported");
    }
    if collision.indices.is_empty() {
        bail!("no collision meshes imported");
    }
    collision.bounds = collision_bounds.unwrap_or_else(empty_bounds);
    let bounds = merge_bounds(visual_bounds, &collision.bounds);
    let mut spawn_position = spawn_translation.to_array();
    spawn_position[1] = surface_height_at(&collision, spawn_position[0], spawn_position[2])
        .map(|height| height + 0.02)
        .unwrap_or(spawn_position[1] - 0.95);
    let spawn = SpawnPoint {
        name: String::from("start"),
        position: spawn_position,
        rotation_xyzw: spawn_rotation.to_array(),
    };
    let title = ui
        .as_ref()
        .and_then(|ui| ui.name.clone())
        .unwrap_or_else(|| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("AC Track")
                .to_string()
        });
    let length = ui
        .as_ref()
        .and_then(|ui| ui.length.as_deref())
        .and_then(parse_length)
        .unwrap_or(150.0)
        * scale;
    let camera = start_camera(&spawn, scale);
    let metadata = TrackMetadata {
        name: title,
        length,
        spawns: vec![spawn],
        pit_boxes: Vec::new(),
        cameras: vec![camera],
        sectors: Vec::new(),
        surfaces,
    };

    warnings.push(format!("track scale {scale:.3}"));
    warnings.push(format!("spawn derived from {spawn_name}"));
    warnings.push(format!(
        "imported {} visual meshes and {} collision triangles",
        visual.meshes.len(),
        collision.indices.len() / 3
    ));

    Ok(ImportedTrack {
        visual,
        metadata,
        collision,
        bounds,
        warnings,
    })
}

fn find_spawn(models: &[(PathBuf, Kn5)]) -> Option<(String, Mat4)> {
    for wanted in ["AC_START_0", "AC_START_1"] {
        for (_, model) in models {
            if let Some((node, world)) = model.find_node(wanted) {
                if node.active {
                    return Some((wanted.to_string(), world));
                }
            }
        }
    }
    for (_, model) in models {
        for (node, world) in model.walk() {
            if node.active && node.name.starts_with("AC_START_") {
                return Some((node.name.clone(), world));
            }
        }
    }
    None
}

pub(crate) fn surface_height_at(collision: &TrackCollision, x: f32, z: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for triangle in collision.indices.chunks(3) {
        let a = collision.vertices[triangle[0] as usize];
        let b = collision.vertices[triangle[1] as usize];
        let c = collision.vertices[triangle[2] as usize];
        let denominator = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
        if denominator.abs() < 1e-6 {
            continue;
        }
        let u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / denominator;
        let v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / denominator;
        let w = 1.0 - u - v;
        if u < 0.0 || v < 0.0 || w < 0.0 {
            continue;
        }
        let height = a[1] * w + b[1] * u + c[1] * v;
        best = Some(best.map_or(height, |current| current.max(height)));
    }
    best
}

pub(crate) fn start_camera(spawn: &SpawnPoint, scale: f32) -> CameraSpec {
    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    )
    .normalize();
    let position = Vec3::from_array(spawn.position);
    let forward = rotation * Vec3::new(0.0, 0.0, -1.0);
    let right = rotation * Vec3::new(1.0, 0.0, 0.0);
    let eye = position - forward * (14.0 * scale) + right * (5.0 * scale) + Vec3::Y * (7.0 * scale);
    CameraSpec {
        name: String::from("start"),
        position: eye.to_array(),
        target: (position + forward * (20.0 * scale) + Vec3::Y * (0.5 * scale)).to_array(),
        fov_y: 0.9,
    }
}

fn transform_mesh(mesh: &crate::kn5::Kn5Mesh, world: Mat4) -> MeshBuffers {
    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();
    let mut buffers = MeshBuffers {
        positions: Vec::with_capacity(mesh.positions.len()),
        normals: Vec::with_capacity(mesh.normals.len()),
        uvs: mesh.uvs.clone(),
        colors: Vec::new(),
        indices: mesh.indices.clone(),
    };
    for position in &mesh.positions {
        buffers.positions.push(
            world
                .transform_point3(Vec3::from_array(*position))
                .to_array(),
        );
    }
    for normal in &mesh.normals {
        buffers.normals.push(
            (normal_matrix * Vec3::from_array(*normal))
                .normalize_or_zero()
                .to_array(),
        );
    }
    buffers
}

fn mesh_ref(name: &str, mesh: &MeshBuffers, material: u16) -> MeshRef {
    MeshRef {
        name: name.to_string(),
        vertex_offset: 0,
        vertex_count: mesh.positions.len() as u32,
        index_offset: 0,
        index_count: mesh.indices.len() as u32,
        material,
        bounds: bounds(mesh),
        lod: 0,
    }
}

fn collision_surface_key(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    if upper.starts_with("1ANELLO") {
        return Some("ASPHALT");
    }
    if upper.contains("GRASS") {
        return Some("GRASS");
    }
    if let Some(key) = upper.strip_prefix("1ASP") {
        return match key {
            "HALT" => Some("ASPHALT"),
            "CEN" => Some("ASPCEN"),
            "SEC" => Some("ASPSEC"),
            "THI" => Some("ASPTHI"),
            "FOU" => Some("ASPFOU"),
            "FIF" => Some("ASPFIF"),
            "SIX" => Some("ASPSIX"),
            "OUT" => Some("ASPOUT"),
            _ => None,
        };
    }
    if upper == "TRANSITION" {
        return Some("ANELLO");
    }
    if upper.starts_with("2ROAD") || upper.starts_with("1ROAD") || upper == "CERCHI" {
        return Some("ASPHALT");
    }
    None
}

fn load_texture_overrides(path: &Path) -> Result<HashMap<String, LoadedTextureOverride>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read texture overrides {}", path.display()))?;
    let overrides = serde_json::from_str::<HashMap<String, TextureOverride>>(&text)
        .with_context(|| format!("failed to parse texture overrides {}", path.display()))?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut loaded = HashMap::new();
    for (material, entry) in overrides {
        let texture_path = base.join(&entry.texture);
        let bytes = fs::read(&texture_path)
            .with_context(|| format!("failed to read texture {}", texture_path.display()))?;
        let name = texture_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("override")
            .to_string();
        loaded.insert(
            material,
            LoadedTextureOverride {
                name,
                bytes,
                uv_scale: entry.uv_scale,
                uv_mode: entry.uv_mode,
                cache_key: format!("override:{}", texture_path.display()),
            },
        );
    }
    Ok(loaded)
}

fn import_texture(
    textures: &mut Vec<TextureRef>,
    cache: &mut HashMap<String, u32>,
    source_path: &Path,
    name: &str,
    bytes: &[u8],
    max_size: u32,
    cache_key: Option<&str>,
) -> Result<u32> {
    let key = cache_key
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}:{name}", source_path.display()));
    if let Some(index) = cache.get(&key) {
        return Ok(*index);
    }
    let image = image::load_from_memory(bytes)
        .with_context(|| format!("failed to decode texture {name}"))?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        bail!("texture {name} has zero dimensions");
    }
    let scale = (max_size as f32 / width.max(height) as f32).min(1.0);
    let image = if scale < 1.0 {
        let width = ((width as f32 * scale).round() as u32).max(1);
        let height = ((height as f32 * scale).round() as u32).max(1);
        image.resize_exact(width, height, FilterType::Triangle)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba8 = rgba.into_raw();
    let mut hasher = Sha256::new();
    hasher.update(&rgba8);
    let digest = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    let index = textures.len() as u32;
    textures.push(TextureRef {
        name: name.to_string(),
        mime: String::from("image/x-dds"),
        hash,
        width,
        height,
        rgba8,
    });
    cache.insert(key, index);
    Ok(index)
}

fn skip_material(name: &str) -> bool {
    matches!(name, "FenceMat" | "skidd" | "trees" | "22 - Default")
}

fn texture_skipped(name: &str) -> bool {
    matches!(name, "terrain" | "BarrierMat" | "tenda")
}

fn material_uv_scale(name: &str) -> f32 {
    match name {
        "Concrete2Mat" => 0.25,
        _ => 1.0,
    }
}

fn material_color(name: &str) -> [f32; 3] {
    match name {
        "wall" => [0.48, 0.5, 0.54],
        "Concrete2Mat" => [0.55, 0.56, 0.58],
        "Asphalt2Mat" | "tarmac" => [0.08, 0.09, 0.11],
        "signs" => [0.9, 0.82, 0.3],
        "tenda" => [0.62, 0.12, 0.1],
        "BarrierMat" => [0.32, 0.38, 0.45],
        "linea" => [0.92, 0.92, 0.92],
        "numeri" | "numeri2" | "22 - Default" => [0.85, 0.85, 0.8],
        "transition" => [0.47, 0.4, 0.27],
        "GrassMat" | "terrain" => [0.18, 0.34, 0.13],
        "spawn" => [0.1, 0.8, 0.2],
        _ => [0.6, 0.6, 0.6],
    }
}

fn visual_y_offset(name: &str) -> f32 {
    let upper = name.to_ascii_uppercase();
    if upper == "1ASPHALT" {
        -0.015
    } else if upper.starts_with("1ROAD") || upper == "TRANSITION" {
        0.005
    } else {
        0.0
    }
}

fn material_outline(name: &str) -> bool {
    matches!(
        name,
        "wall" | "BarrierMat" | "linea" | "signs" | "numeri" | "numeri2"
    )
}

fn parse_surfaces(path: &Path) -> Result<(Vec<SurfaceSpec>, HashMap<String, u16>)> {
    let sections = parse_ini(path)?;
    let mut surfaces = Vec::new();
    let mut ids = HashMap::new();
    for (section, values) in sections {
        if !section.starts_with("SURFACE_") {
            continue;
        }
        let Some(key) = values.get("KEY") else {
            continue;
        };
        let id = surfaces.len() as u16;
        ids.insert(key.clone(), id);
        surfaces.push(SurfaceSpec {
            id,
            name: key.clone(),
            friction: surface_friction(
                key,
                values
                    .get("FRICTION")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1.0),
            ),
            roughness: 0.02,
        });
    }
    if surfaces.is_empty() {
        bail!("surfaces.ini has no SURFACE_* entries");
    }
    Ok((surfaces, ids))
}

fn surface_friction(key: &str, source: f32) -> f32 {
    if key.eq_ignore_ascii_case("GRASS") {
        source.min(0.35)
    } else {
        source
    }
}

fn parse_length(value: &str) -> Option<f32> {
    let number = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect::<String>();
    number.parse().ok()
}

fn parse_ini(path: &Path) -> Result<Vec<(String, HashMap<String, String>)>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut sections = Vec::new();
    let mut current: Option<(String, HashMap<String, String>)> = None;
    for line in text.lines() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some((line[1..line.len() - 1].to_string(), HashMap::new()));
        } else if let Some((key, value)) = line.split_once('=') {
            if let Some((_, values)) = current.as_mut() {
                values.insert(key.trim().to_ascii_uppercase(), value.trim().to_string());
            }
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    Ok(sections)
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    if root.is_file() {
        if root.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(root.to_path_buf());
        }
        return None;
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

fn find_kn5_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_kn5(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_kn5(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_kn5(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("kn5") {
            files.push(path);
        }
    }
    Ok(())
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mesh")
        .to_string()
}

fn empty_bounds() -> Aabb {
    Aabb {
        min: [0.0; 3],
        max: [0.0; 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_skidpad_ground_surfaces() {
        assert_eq!(collision_surface_key("1ANELLO4"), Some("ASPHALT"));
        assert_eq!(collision_surface_key("1GRASS1"), Some("GRASS"));
        assert_eq!(collision_surface_key("0GRASSext"), Some("GRASS"));
        assert_eq!(collision_surface_key("1ASPHALT"), Some("ASPHALT"));
        assert_eq!(collision_surface_key("2ROAD"), Some("ASPHALT"));
        assert_eq!(collision_surface_key("TRANSITION"), Some("ANELLO"));
        assert_eq!(collision_surface_key("1WALL2_SUB0"), None);
    }

    #[test]
    fn layers_are_offset_to_avoid_coplanar_z_fighting() {
        assert!(visual_y_offset("1ASPHALT") < 0.0);
        assert!(visual_y_offset("1ROADbrakingzone") > 0.0);
        assert_eq!(visual_y_offset("2ROAD"), 0.0);
    }

    #[test]
    fn start_camera_scales_with_the_track() {
        let spawn = SpawnPoint {
            name: String::from("start"),
            position: [10.0, 1.0, 20.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        };
        let full = start_camera(&spawn, 1.0);
        let half = start_camera(&spawn, 0.5);
        for (axis, (full, half)) in full.position.iter().zip(half.position.iter()).enumerate() {
            assert!((half - (full + spawn.position[axis]) * 0.5).abs() < 1e-4);
        }
        assert!((half.target[2] - (full.target[2] + spawn.position[2]) * 0.5).abs() < 1e-4);
    }

    #[test]
    fn caps_grass_friction() {
        assert_eq!(surface_friction("GRASS", 0.9), 0.35);
        assert_eq!(surface_friction("grass", 0.2), 0.2);
        assert_eq!(surface_friction("ASPHALT", 0.97), 0.97);
    }
}
