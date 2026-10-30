use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sim_types::{
    Aabb, CameraSpec, MaterialRef, SpawnPoint, SurfaceSpec, TextureRef, TrackCollision,
    TrackMetadata, TrackVisual,
};

use crate::ac_track::{start_camera, surface_height_at};
use crate::glb::{append_primitive, empty_mesh, mesh_ref};
use crate::mesh::{bounds, merge_bounds};

pub struct ImportedTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub bounds: Aabb,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GlbTrackSource {
    schema: String,
    kind: String,
    name: String,
    #[serde(default)]
    length: f32,
    spawn: SpawnSource,
    #[serde(default)]
    cameras: Vec<CameraSpec>,
    visual: GlbVisualSource,
    collision: GlbCollisionSource,
    surfaces: Vec<GlbSurfaceRule>,
    #[serde(default)]
    default_surface: u16,
}

#[derive(Debug, Deserialize)]
struct SpawnSource {
    position: [f32; 3],
    rotation_xyzw: [f32; 4],
}

#[derive(Debug, Deserialize)]
struct GlbVisualSource {
    glb: String,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default = "default_scale")]
    scale: f32,
    #[serde(default)]
    yaw_degrees: f32,
    #[serde(default)]
    offset: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct GlbCollisionSource {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GlbSurfaceRule {
    id: u16,
    name: String,
    friction: f32,
    #[serde(default)]
    roughness: f32,
    #[serde(default)]
    materials: Vec<String>,
    #[serde(default)]
    mountable: bool,
}

fn default_scale() -> f32 {
    1.0
}

pub fn import_track(dir: &Path) -> Result<ImportedTrack> {
    let source_path = dir.join("track.json");
    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let source: GlbTrackSource = serde_json::from_slice(&source_bytes)
        .with_context(|| format!("failed to parse {}", source_path.display()))?;
    validate_source(&source)?;

    let glb_path = dir.join(&source.visual.glb);
    let (document, buffers, images) = gltf::import(&glb_path)
        .with_context(|| format!("failed to import {}", glb_path.display()))?;
    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .context("GLB has no scene")?;

    let scale = source.visual.scale;
    let adjust = Mat4::from_translation(Vec3::from_array(source.visual.offset))
        * Mat4::from_rotation_y(source.visual.yaw_degrees.to_radians())
        * Mat4::from_scale(Vec3::splat(scale));

    let textures = import_textures(&images)?;
    let mut materials = Vec::new();
    let mut material_map = Vec::new();
    for material in document.materials() {
        material_map.push(materials.len() as u16);
        materials.push(import_material(&material));
    }
    let default_material = materials.len() as u16;
    materials.push(MaterialRef {
        name: String::from("glb_default"),
        base_color: [1.0, 1.0, 1.0, 1.0],
        metallic: 0.0,
        roughness: 0.85,
        base_color_texture: None,
        outline: false,
    });
    let mut visual = TrackVisual {
        meshes: Vec::new(),
        materials,
        textures,
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
    let mut warnings = Vec::new();
    let mut unmapped_materials = BTreeSet::new();
    let mut included_visual = 0usize;
    let mut included_collision = 0usize;
    let mut visual_triangles = 0usize;
    let mut collision_triangles = 0usize;

    let mut stack = scene
        .nodes()
        .map(|node| (node, Mat4::IDENTITY))
        .collect::<Vec<_>>();
    while let Some((node, parent)) = stack.pop() {
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = adjust * parent * local;
        let node_name = node.name().unwrap_or("").to_string();

        if let Some(mesh) = node.mesh() {
            let mesh_name = mesh.name().unwrap_or("").to_string();
            let display_name = if node_name.is_empty() {
                mesh_name.as_str()
            } else {
                node_name.as_str()
            };

            if matches_any(display_name, &source.visual.include)
                && !matches_any(display_name, &source.visual.exclude)
            {
                let primitive_count = mesh.primitives().len();
                for (primitive_index, primitive) in mesh.primitives().enumerate() {
                    let mut visual_buffers = empty_mesh();
                    append_primitive(
                        &mut visual_buffers,
                        &primitive,
                        &buffers,
                        &images,
                        world,
                        None,
                        Some([1.0, 1.0, 1.0]),
                    )?;
                    if visual_buffers.indices.is_empty() {
                        continue;
                    }
                    let material = primitive
                        .material()
                        .index()
                        .and_then(|index| material_map.get(index).copied())
                        .unwrap_or(default_material);
                    let name = if primitive_count > 1 {
                        format!("{display_name}#{primitive_index}")
                    } else {
                        display_name.to_string()
                    };
                    let mesh_bounds = bounds(&visual_buffers);
                    visual_bounds = Some(merge_bounds(visual_bounds, &mesh_bounds));
                    visual_triangles += visual_buffers.indices.len() / 3;
                    visual
                        .meshes
                        .push(mesh_ref(&name, &visual_buffers, material));
                    visual.mesh_buffers.push(visual_buffers);
                    included_visual += 1;
                }
            }

            if matches_any(display_name, &source.collision.include)
                && !matches_any(display_name, &source.collision.exclude)
            {
                let mut added = false;
                for primitive in mesh.primitives() {
                    let material_name = primitive.material().name().unwrap_or("");
                    let (surface, mountable) =
                        surface_for_material(&source, material_name, &mut unmapped_materials);
                    if append_collision_primitive(
                        &mut collision,
                        &primitive,
                        &buffers,
                        world,
                        surface,
                        mountable,
                    )? {
                        added = true;
                        collision_triangles += 1;
                    }
                }
                if added {
                    included_collision += 1;
                }
            }
        }

        for child in node.children() {
            stack.push((child, world));
        }
    }

    if visual.meshes.is_empty() {
        bail!("no visual meshes matched the GLB track selectors");
    }
    if collision.indices.is_empty() {
        bail!("no collision meshes matched the GLB track selectors");
    }
    prune_visual(&mut visual);
    collision.bounds = collision_bounds(&collision);
    let bounds = merge_bounds(visual_bounds, &collision.bounds);

    let spawn_rotation = Quat::from_xyzw(
        source.spawn.rotation_xyzw[0],
        source.spawn.rotation_xyzw[1],
        source.spawn.rotation_xyzw[2],
        source.spawn.rotation_xyzw[3],
    )
    .normalize();
    let mut spawn_position = Vec3::from_array(source.spawn.position);
    spawn_position.y = surface_height_at(&collision, spawn_position.x, spawn_position.z)
        .map(|height| height + 0.02)
        .unwrap_or(spawn_position.y);
    let spawn = SpawnPoint {
        name: String::from("start"),
        position: spawn_position.to_array(),
        rotation_xyzw: spawn_rotation.to_array(),
    };
    let camera_scale = if scale > 0.0 { scale } else { 1.0 };
    let cameras = if source.cameras.is_empty() {
        vec![start_camera(&spawn, camera_scale)]
    } else {
        source.cameras.clone()
    };
    let surfaces = source
        .surfaces
        .iter()
        .map(|surface| SurfaceSpec {
            id: surface.id,
            name: surface.name.clone(),
            friction: surface.friction,
            roughness: surface.roughness,
        })
        .collect::<Vec<_>>();

    for material in unmapped_materials {
        warnings.push(format!(
            "material {material} has no surface mapping; using surface {}",
            source.default_surface
        ));
    }
    warnings.push(format!(
        "imported {included_visual} visual meshes ({visual_triangles} triangles) and {included_collision} collision meshes ({collision_triangles} triangles)"
    ));
    warnings.push(format!(
        "track scale {scale:.3}, yaw {:.1} degrees",
        source.visual.yaw_degrees
    ));

    let metadata = TrackMetadata {
        name: source.name,
        length: source.length,
        spawns: vec![spawn],
        pit_boxes: Vec::new(),
        cameras,
        sectors: Vec::new(),
        surfaces,
    };

    Ok(ImportedTrack {
        visual,
        metadata,
        collision,
        bounds,
        warnings,
    })
}

fn validate_source(source: &GlbTrackSource) -> Result<()> {
    if source.schema != "sim-track-glb-v1" {
        bail!(
            "unsupported source schema {}; expected sim-track-glb-v1",
            source.schema
        );
    }
    if source.kind != "track" {
        bail!("expected a track source, got {}", source.kind);
    }
    if !source.visual.scale.is_finite() || source.visual.scale <= 0.0 {
        bail!("visual scale must be positive and finite");
    }
    if !source.visual.yaw_degrees.is_finite() {
        bail!("visual yaw_degrees must be finite");
    }
    if source.visual.offset.iter().any(|value| !value.is_finite()) {
        bail!("visual offset must be finite");
    }
    if source.spawn.position.iter().any(|value| !value.is_finite()) {
        bail!("spawn position must be finite");
    }
    if source
        .spawn
        .rotation_xyzw
        .iter()
        .any(|value| !value.is_finite())
    {
        bail!("spawn rotation must be finite");
    }
    if source.surfaces.is_empty() {
        bail!("at least one surface is required");
    }
    let ids = source
        .surfaces
        .iter()
        .map(|surface| surface.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != source.surfaces.len() {
        bail!("surface IDs must be unique");
    }
    if !ids.contains(&source.default_surface) {
        bail!("default_surface must reference a declared surface");
    }
    Ok(())
}

fn matches_any(name: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| pattern_matches(pattern, name))
}

fn pattern_matches(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase().into_bytes();
    let text = text.to_ascii_lowercase().into_bytes();
    let mut pattern_index = 0usize;
    let mut text_index = 0usize;
    let mut star = None;
    let mut star_match = 0usize;

    while text_index < text.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == text[text_index])
        {
            pattern_index += 1;
            text_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star = Some(pattern_index);
            star_match = text_index;
            pattern_index += 1;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            star_match += 1;
            text_index = star_match;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn surface_for_material(
    source: &GlbTrackSource,
    material: &str,
    unmapped: &mut BTreeSet<String>,
) -> (u16, bool) {
    for rule in &source.surfaces {
        if rule
            .materials
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(material))
        {
            return (rule.id, rule.mountable);
        }
    }
    if !material.is_empty() {
        unmapped.insert(material.to_string());
    }
    let mountable = source
        .surfaces
        .iter()
        .find(|rule| rule.id == source.default_surface)
        .map(|rule| rule.mountable)
        .unwrap_or(false);
    (source.default_surface, mountable)
}

fn append_collision_primitive(
    collision: &mut TrackCollision,
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    matrix: Mat4,
    surface: u16,
    mountable: bool,
) -> Result<bool> {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        return Ok(false);
    }
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
    let Some(positions) = reader.read_positions() else {
        return Ok(false);
    };
    let positions = positions.collect::<Vec<_>>();
    if positions.is_empty() {
        return Ok(false);
    }
    let mut indices = reader
        .read_indices()
        .map(|indices| indices.into_u32().collect::<Vec<_>>())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    indices.truncate(indices.len() / 3 * 3);
    if indices.is_empty() {
        return Ok(false);
    }

    let base = collision.vertices.len() as u32;
    let mut transformed = Vec::with_capacity(positions.len());
    for position in positions {
        let position = matrix
            .transform_point3(Vec3::from_array(position))
            .to_array();
        transformed.push(position);
        collision.vertices.push(position);
    }

    let mut kept = 0usize;
    for triangle in indices.as_chunks::<3>().0 {
        if mountable {
            let a = Vec3::from_array(transformed[triangle[0] as usize]);
            let b = Vec3::from_array(transformed[triangle[1] as usize]);
            let c = Vec3::from_array(transformed[triangle[2] as usize]);
            let normal = (b - a).cross(c - a);
            if normal.length_squared() < 1e-10 || normal.normalize().y.abs() <= 0.3 {
                continue;
            }
        }
        collision
            .indices
            .extend(triangle.iter().map(|index| base + *index));
        collision.surface_ids.push(surface);
        kept += 1;
    }
    Ok(kept > 0)
}

fn empty_bounds() -> Aabb {
    Aabb {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    }
}

fn collision_bounds(collision: &TrackCollision) -> Aabb {
    let mut bounds = empty_bounds();
    for position in &collision.vertices {
        for (axis, value) in position.iter().enumerate().take(3) {
            bounds.min[axis] = bounds.min[axis].min(*value);
            bounds.max[axis] = bounds.max[axis].max(*value);
        }
    }
    bounds
}

fn import_material(material: &gltf::Material<'_>) -> MaterialRef {
    let pbr = material.pbr_metallic_roughness();
    let factor = pbr.base_color_factor();
    let base_color_texture = pbr
        .base_color_texture()
        .map(|info| info.texture().source().index() as u32);
    MaterialRef {
        name: material.name().unwrap_or("glb_material").to_string(),
        base_color: factor,
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        base_color_texture,
        outline: false,
    }
}

fn import_textures(images: &[gltf::image::Data]) -> Result<Vec<TextureRef>> {
    let mut textures = Vec::with_capacity(images.len());
    for (index, image) in images.iter().enumerate() {
        if image.width == 0 || image.height == 0 {
            bail!("texture {index} has zero dimensions");
        }
        if image.width > 2048 || image.height > 2048 {
            bail!(
                "texture {index} is {}x{}; maximum is 2048",
                image.width,
                image.height
            );
        }
        let rgba8 = image_to_rgba8(image)?;
        let mut hasher = Sha256::new();
        hasher.update(&rgba8);
        let digest = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        textures.push(TextureRef {
            name: format!("texture_{index}"),
            mime: String::from("image/png"),
            hash,
            width: image.width,
            height: image.height,
            rgba8,
        });
    }
    Ok(textures)
}

fn image_to_rgba8(image: &gltf::image::Data) -> Result<Vec<u8>> {
    let pixel_count = image.width as usize * image.height as usize;
    let mut out = Vec::with_capacity(pixel_count * 4);
    match image.format {
        gltf::image::Format::R8 => {
            for &value in &image.pixels {
                out.extend_from_slice(&[value, value, value, 255]);
            }
        }
        gltf::image::Format::R8G8 => {
            for pixel in image.pixels.as_chunks::<2>().0 {
                out.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
        }
        gltf::image::Format::R8G8B8 => {
            for pixel in image.pixels.as_chunks::<3>().0 {
                out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        gltf::image::Format::R8G8B8A8 => out.extend_from_slice(&image.pixels),
        gltf::image::Format::R16 => {
            for pixel in image.pixels.as_chunks::<2>().0 {
                let value = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
                out.extend_from_slice(&[value, value, value, 255]);
            }
        }
        gltf::image::Format::R16G16 => {
            for pixel in image.pixels.as_chunks::<4>().0 {
                let r = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([pixel[2], pixel[3]]) >> 8) as u8;
                out.extend_from_slice(&[r, g, 0, 255]);
            }
        }
        gltf::image::Format::R16G16B16 => {
            for pixel in image.pixels.as_chunks::<6>().0 {
                let r = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([pixel[2], pixel[3]]) >> 8) as u8;
                let b = (u16::from_le_bytes([pixel[4], pixel[5]]) >> 8) as u8;
                out.extend_from_slice(&[r, g, b, 255]);
            }
        }
        gltf::image::Format::R16G16B16A16 => {
            for pixel in image.pixels.as_chunks::<8>().0 {
                let r = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([pixel[2], pixel[3]]) >> 8) as u8;
                let b = (u16::from_le_bytes([pixel[4], pixel[5]]) >> 8) as u8;
                let a = (u16::from_le_bytes([pixel[6], pixel[7]]) >> 8) as u8;
                out.extend_from_slice(&[r, g, b, a]);
            }
        }
        gltf::image::Format::R32G32B32FLOAT => {
            for pixel in image.pixels.as_chunks::<12>().0 {
                for channel in 0..3 {
                    let offset = channel * 4;
                    let value = f32::from_le_bytes([
                        pixel[offset],
                        pixel[offset + 1],
                        pixel[offset + 2],
                        pixel[offset + 3],
                    ]);
                    out.push((value.clamp(0.0, 1.0) * 255.0) as u8);
                }
                out.push(255);
            }
        }
        gltf::image::Format::R32G32B32A32FLOAT => {
            for pixel in image.pixels.as_chunks::<16>().0 {
                for channel in 0..4 {
                    let offset = channel * 4;
                    let value = f32::from_le_bytes([
                        pixel[offset],
                        pixel[offset + 1],
                        pixel[offset + 2],
                        pixel[offset + 3],
                    ]);
                    out.push((value.clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }
    }
    if out.len() != pixel_count * 4 {
        bail!(
            "decoded image has {} bytes; expected {}",
            out.len(),
            pixel_count * 4
        );
    }
    Ok(out)
}

fn prune_visual(visual: &mut TrackVisual) {
    let used_materials = visual
        .meshes
        .iter()
        .map(|mesh| mesh.material)
        .collect::<BTreeSet<_>>();
    let mut material_remap = vec![None; visual.materials.len()];
    let mut materials = Vec::new();
    for (old_index, material) in visual.materials.iter().enumerate() {
        if used_materials.contains(&(old_index as u16)) {
            material_remap[old_index] = Some(materials.len() as u16);
            materials.push(material.clone());
        }
    }

    let mut texture_remap = vec![None; visual.textures.len()];
    let mut textures = Vec::new();
    for material in &mut materials {
        let Some(old_texture) = material.base_color_texture else {
            continue;
        };
        let old_index = old_texture as usize;
        let new_index = if let Some(index) = texture_remap[old_index] {
            index
        } else {
            let index = textures.len() as u32;
            texture_remap[old_index] = Some(index);
            textures.push(visual.textures[old_index].clone());
            index
        };
        material.base_color_texture = Some(new_index);
    }

    for mesh in &mut visual.meshes {
        mesh.material = material_remap[mesh.material as usize].expect("used material");
    }
    visual.materials = materials;
    visual.textures = textures;
}
