use anyhow::{bail, Context, Result};
use rkyv::ser::serializers::AllocSerializer;
use sha2::{Digest, Sha256};
use sim_types::{
    Aabb, AssetKind, AssetManifest, AssetStats, CarVisual, ChunkKind, ChunkRef, Compression,
    EntryPoints, MaterialRef, MeshBuffers, MeshRef, PackHeader, TextureRef, TrackCollision,
    TrackVisual, WheelVisual, ENGINE_FRAME, ENGINE_UNITS, MAGIC, PACK_FORMAT_VERSION,
    PACK_HEADER_SIZE, SCHEMA_VERSION,
};
use std::fs;
use std::path::Path;

use crate::mesh::{bounds, build_box, build_cylinder, build_plane, merge_bounds};
use crate::source::{
    validate_source_header, CarSource, CollisionPrimitive, Primitive, TrackSource,
};

const CHUNK_ALIGN: usize = 64;

struct ChunkSpec {
    kind: ChunkKind,
    bytes: Vec<u8>,
}

pub fn bake_car(dir: &Path) -> Result<Vec<u8>> {
    let source_path = dir.join("car.json");
    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let source: CarSource = serde_json::from_slice(&source_bytes)
        .with_context(|| format!("failed to parse {}", source_path.display()))?;
    validate_source_header(
        &source.schema,
        &source.kind,
        "car",
        &source.units,
        &source.frame,
    )?;

    if source.physics.wheels.len() != 4 {
        bail!("car physics must contain exactly four wheels");
    }
    if source.visual.wheels.len() != 4 {
        bail!("car visual must contain exactly four wheels");
    }
    if source.physics.mass <= 0.0 {
        bail!("car mass must be positive");
    }

    let (visual, visual_bounds, stats) = build_car_visual(&source.visual)?;
    let physics = source.physics;
    let physics_bytes = archive(&physics)?;
    let visual_bytes = archive(&visual)?;

    let chunks = vec![
        ChunkSpec {
            kind: ChunkKind::Physics,
            bytes: physics_bytes,
        },
        ChunkSpec {
            kind: ChunkKind::Visual,
            bytes: visual_bytes,
        },
    ];

    let manifest = AssetManifest {
        magic: MAGIC,
        schema_version: SCHEMA_VERSION,
        kind: AssetKind::Car,
        id: source.id,
        source_hash: sha256(&source_bytes),
        units: ENGINE_UNITS,
        frame: ENGINE_FRAME,
        bounds: visual_bounds,
        chunks: Vec::new(),
        entrypoints: EntryPoints {
            physics: Some(0),
            visual: Some(1),
            collision: None,
            metadata: None,
            audio: None,
            texture: None,
        },
        stats,
        warnings: Vec::new(),
    };

    write_pack(manifest, chunks)
}

pub fn bake_track(dir: &Path) -> Result<Vec<u8>> {
    let source_path = dir.join("track.json");
    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let source: TrackSource = serde_json::from_slice(&source_bytes)
        .with_context(|| format!("failed to parse {}", source_path.display()))?;
    validate_source_header(
        &source.schema,
        &source.kind,
        "track",
        &source.units,
        &source.frame,
    )?;

    if source.metadata.spawns.is_empty() {
        bail!("track metadata must contain at least one spawn point");
    }

    let (visual, visual_bounds, stats) = build_track_visual(&source.visual)?;
    let (collision, collision_bounds) = build_track_collision(&source.collision.primitives)?;
    let metadata = source.metadata;

    let visual_bytes = archive(&visual)?;
    let collision_bytes = archive(&collision)?;
    let metadata_bytes = archive(&metadata)?;

    let chunks = vec![
        ChunkSpec {
            kind: ChunkKind::Visual,
            bytes: visual_bytes,
        },
        ChunkSpec {
            kind: ChunkKind::Collision,
            bytes: collision_bytes,
        },
        ChunkSpec {
            kind: ChunkKind::Metadata,
            bytes: metadata_bytes,
        },
    ];

    let manifest = AssetManifest {
        magic: MAGIC,
        schema_version: SCHEMA_VERSION,
        kind: AssetKind::Track,
        id: source.id,
        source_hash: sha256(&source_bytes),
        units: ENGINE_UNITS,
        frame: ENGINE_FRAME,
        bounds: merge_bounds(Some(visual_bounds), &collision_bounds),
        chunks: Vec::new(),
        entrypoints: EntryPoints {
            physics: None,
            visual: Some(0),
            collision: Some(1),
            metadata: Some(2),
            audio: None,
            texture: None,
        },
        stats,
        warnings: Vec::new(),
    };

    write_pack(manifest, chunks)
}

fn build_car_visual(
    source: &crate::source::CarVisualSource,
) -> Result<(CarVisual, Aabb, AssetStats)> {
    let material_indices = material_indices(&source.materials);
    let mut meshes = Vec::with_capacity(source.primitives.len());
    let mut mesh_buffers = Vec::with_capacity(source.primitives.len());
    let mut total = None;

    for primitive in &source.primitives {
        let mesh = build_primitive(primitive, &material_indices)?;
        let mesh_bounds = bounds(&mesh);
        total = Some(merge_bounds(total, &mesh_bounds));
        meshes.push(mesh_ref(
            primitive.id(),
            &mesh,
            material_indices[primitive.material()],
        ));
        mesh_buffers.push(mesh);
    }

    let wheels: [WheelVisual; 4] = source
        .wheels
        .clone()
        .try_into()
        .map_err(|_| anyhow::anyhow!("car visual must contain exactly four wheels"))?;
    for wheel in &wheels {
        if wheel.mesh as usize >= meshes.len() {
            bail!(
                "wheel {} references missing mesh {}",
                wheel.name,
                wheel.mesh
            );
        }
    }

    let visual = CarVisual {
        meshes,
        materials: source.materials.clone(),
        textures: source.textures.clone(),
        mesh_buffers,
        body: source.body,
        wheels,
        steering_wheel: source.steering_wheel,
    };
    let stats = stats(
        &visual.meshes,
        &visual.materials,
        &visual.textures,
        &visual.mesh_buffers,
    );
    Ok((visual, total.unwrap_or(empty_bounds()), stats))
}

fn build_track_visual(
    source: &crate::source::TrackVisualSource,
) -> Result<(TrackVisual, Aabb, AssetStats)> {
    let material_indices = material_indices(&source.materials);
    let mut meshes = Vec::with_capacity(source.primitives.len());
    let mut mesh_buffers = Vec::with_capacity(source.primitives.len());
    let mut total = None;

    for primitive in &source.primitives {
        let mesh = build_primitive(primitive, &material_indices)?;
        let mesh_bounds = bounds(&mesh);
        total = Some(merge_bounds(total, &mesh_bounds));
        meshes.push(mesh_ref(
            primitive.id(),
            &mesh,
            material_indices[primitive.material()],
        ));
        mesh_buffers.push(mesh);
    }

    let visual = TrackVisual {
        meshes,
        materials: source.materials.clone(),
        textures: source.textures.clone(),
        mesh_buffers,
        instances: Vec::new(),
    };
    let stats = stats(
        &visual.meshes,
        &visual.materials,
        &visual.textures,
        &visual.mesh_buffers,
    );
    Ok((visual, total.unwrap_or(empty_bounds()), stats))
}

fn build_track_collision(primitives: &[CollisionPrimitive]) -> Result<(TrackCollision, Aabb)> {
    let mut collision = TrackCollision {
        vertices: Vec::new(),
        indices: Vec::new(),
        surface_ids: Vec::new(),
        bounds: empty_bounds(),
    };
    let mut total = None;

    for primitive in primitives {
        let mesh = match primitive {
            CollisionPrimitive::Plane { size, position, .. } => build_plane(*size, *position),
            CollisionPrimitive::BoxShape { size, position, .. } => build_box(*size, *position),
        };
        let base = collision.vertices.len() as u32;
        collision.vertices.extend_from_slice(&mesh.positions);
        collision
            .indices
            .extend(mesh.indices.iter().map(|index| index + base));
        for _ in 0..mesh.indices.len() / 3 {
            collision.surface_ids.push(primitive.surface());
        }
        let mesh_bounds = bounds(&mesh);
        total = Some(merge_bounds(total, &mesh_bounds));
    }

    collision.bounds = total.unwrap_or(empty_bounds());
    let bounds = collision.bounds;
    Ok((collision, bounds))
}

fn build_primitive(
    primitive: &Primitive,
    material_indices: &std::collections::HashMap<&str, usize>,
) -> Result<MeshBuffers> {
    if !material_indices.contains_key(primitive.material()) {
        bail!(
            "primitive {} references unknown material {}",
            primitive.id(),
            primitive.material()
        );
    }
    match primitive {
        Primitive::BoxShape { size, position, .. } => Ok(build_box(*size, *position)),
        Primitive::Plane { size, position, .. } => Ok(build_plane(*size, *position)),
        Primitive::Cylinder {
            radius,
            width,
            axis,
            segments,
            position,
            ..
        } => build_cylinder(*radius, *width, axis, *segments, *position),
    }
}

fn material_indices(materials: &[MaterialRef]) -> std::collections::HashMap<&str, usize> {
    materials
        .iter()
        .enumerate()
        .map(|(index, material)| (material.name.as_str(), index))
        .collect()
}

fn mesh_ref(name: &str, mesh: &MeshBuffers, material: usize) -> MeshRef {
    MeshRef {
        name: name.to_string(),
        vertex_offset: 0,
        vertex_count: mesh.positions.len() as u32,
        index_offset: 0,
        index_count: mesh.indices.len() as u32,
        material: material as u16,
        bounds: bounds(mesh),
        lod: 0,
    }
}

fn stats(
    meshes: &[MeshRef],
    materials: &[MaterialRef],
    textures: &[TextureRef],
    buffers: &[MeshBuffers],
) -> AssetStats {
    AssetStats {
        vertices: buffers.iter().map(|mesh| mesh.positions.len() as u32).sum(),
        triangles: buffers
            .iter()
            .map(|mesh| mesh.indices.len() as u32 / 3)
            .sum(),
        materials: materials.len() as u16,
        textures: textures.len() as u16,
        draw_calls: meshes.len() as u16,
    }
}

fn write_pack(manifest: AssetManifest, chunks: Vec<ChunkSpec>) -> Result<Vec<u8>> {
    let mut manifest = manifest;
    manifest.chunks = chunks
        .iter()
        .map(|chunk| ChunkRef {
            kind: chunk.kind,
            offset: 0,
            len: chunk.bytes.len() as u64,
            align: CHUNK_ALIGN as u32,
            hash: sha256(&chunk.bytes),
            compression: Compression::None,
            schema_version: SCHEMA_VERSION,
        })
        .collect();
    manifest.validate()?;

    let mut manifest_bytes = serde_json::to_vec(&manifest)?;
    for _ in 0..8 {
        let base = align_up(PACK_HEADER_SIZE + manifest_bytes.len(), CHUNK_ALIGN);
        let mut offset = base;
        for chunk in &mut manifest.chunks {
            chunk.offset = offset as u64;
            offset = align_up(offset + chunk.len as usize, CHUNK_ALIGN);
        }
        let next = serde_json::to_vec(&manifest)?;
        let stable = next.len() == manifest_bytes.len()
            && align_up(PACK_HEADER_SIZE + next.len(), CHUNK_ALIGN) == base;
        manifest_bytes = next;
        if stable {
            break;
        }
    }

    let base = align_up(PACK_HEADER_SIZE + manifest_bytes.len(), CHUNK_ALIGN);
    if manifest.chunks.first().map(|chunk| chunk.offset as usize) != Some(base)
        && !manifest.chunks.is_empty()
    {
        bail!("failed to stabilize pack chunk offsets");
    }

    let mut out = vec![0u8; PACK_HEADER_SIZE];
    out.extend_from_slice(&manifest_bytes);
    out.resize(base, 0);

    for (chunk, spec) in manifest.chunks.iter().zip(chunks.iter()) {
        let offset = chunk.offset as usize;
        if out.len() > offset {
            bail!("chunk offset overlaps previous data");
        }
        out.resize(offset, 0);
        out.extend_from_slice(&spec.bytes);
    }

    let pack_len = out.len() as u64;
    let header = PackHeader {
        magic: MAGIC,
        format_version: PACK_FORMAT_VERSION,
        header_size: PACK_HEADER_SIZE as u16,
        manifest_offset: PACK_HEADER_SIZE as u64,
        manifest_len: manifest_bytes.len() as u64,
        pack_len,
        reserved: [0; 32],
    };
    write_header(&mut out[..PACK_HEADER_SIZE], &header);

    sim_asset::Pack::from_bytes(&out).context("baked pack failed round-trip validation")?;
    Ok(out)
}

fn write_header(out: &mut [u8], header: &PackHeader) {
    out[0..4].copy_from_slice(&header.magic);
    out[4..6].copy_from_slice(&header.format_version.to_le_bytes());
    out[6..8].copy_from_slice(&header.header_size.to_le_bytes());
    out[8..16].copy_from_slice(&header.manifest_offset.to_le_bytes());
    out[16..24].copy_from_slice(&header.manifest_len.to_le_bytes());
    out[24..32].copy_from_slice(&header.pack_len.to_le_bytes());
    out[32..64].copy_from_slice(&header.reserved);
}

fn archive<T>(value: &T) -> Result<Vec<u8>>
where
    T: rkyv::Serialize<AllocSerializer<256>>,
{
    let bytes = rkyv::to_bytes::<_, 256>(value)?;
    Ok(bytes.to_vec())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn empty_bounds() -> Aabb {
    Aabb {
        min: [0.0; 3],
        max: [0.0; 3],
    }
}

pub fn inspect(bytes: &[u8]) -> Result<()> {
    let pack = sim_asset::Pack::from_bytes(bytes)?;
    let manifest = pack.manifest();
    println!("asset: {} ({:?})", manifest.id, manifest.kind);
    println!(
        "schema: {}  chunks: {}  bounds: {:?}..{:?}",
        manifest.schema_version,
        manifest.chunks.len(),
        manifest.bounds.min,
        manifest.bounds.max
    );
    println!(
        "stats: {} verts, {} tris, {} materials, {} textures, {} draw calls",
        manifest.stats.vertices,
        manifest.stats.triangles,
        manifest.stats.materials,
        manifest.stats.textures,
        manifest.stats.draw_calls
    );

    for (index, chunk) in manifest.chunks.iter().enumerate() {
        println!(
            "  [{index}] {:<10} off={:<8} len={:<8} align={} hash={}",
            chunk.kind.as_str(),
            chunk.offset,
            chunk.len,
            chunk.align,
            hex_prefix(&chunk.hash)
        );
    }

    match manifest.kind {
        AssetKind::Car => {
            let physics = pack.car_physics_owned()?;
            println!(
                "car: mass={} kg wheelbase={} m drive={:?}",
                physics.mass, physics.wheelbase, physics.drivetrain.drive_layout
            );
            println!(
                "engine: idle={} rpm redline={} rpm gears={}",
                physics.engine.idle_rpm,
                physics.engine.redline_rpm,
                physics.drivetrain.gear_ratios.len()
            );
            for wheel in &physics.wheels {
                println!(
                    "  wheel {:<2} pos={:?} r={} front={} driven={}",
                    wheel.name, wheel.position, wheel.radius, wheel.is_front, wheel.driven
                );
            }
            let visual = pack.car_visual_owned()?;
            println!(
                "visual: {} meshes, {} material(s), {} wheel node(s)",
                visual.meshes.len(),
                visual.materials.len(),
                visual.wheels.len()
            );
        }
        AssetKind::Track => {
            let metadata = pack.track_metadata_owned()?;
            let collision = pack.track_collision_owned()?;
            let visual = pack.track_visual_owned()?;
            println!(
                "track: {} length={} m spawns={} surfaces={} sectors={}",
                metadata.name,
                metadata.length,
                metadata.spawns.len(),
                metadata.surfaces.len(),
                metadata.sectors.len()
            );
            println!(
                "collision: {} vertices, {} triangles",
                collision.vertices.len(),
                collision.indices.len() / 3
            );
            println!(
                "visual: {} meshes, {} material(s), {} mesh buffer(s)",
                visual.meshes.len(),
                visual.materials.len(),
                visual.mesh_buffers.len()
            );
        }
    }

    Ok(())
}

pub fn validate(bytes: &[u8]) -> Result<()> {
    let pack = sim_asset::Pack::from_bytes(bytes)?;
    println!(
        "ok: {} ({:?}), {} chunks, {} bytes",
        pack.manifest().id,
        pack.manifest().kind,
        pack.manifest().chunks.len(),
        bytes.len()
    );
    Ok(())
}

fn hex_prefix(hash: &[u8; 32]) -> String {
    hash.iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
