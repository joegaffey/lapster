use asset_compiler::bake::{bake_car, bake_car_glb, bake_glb_track, bake_track, set_category};
use glam::Vec3;
use sim_asset::{Pack, PackError};
use sim_types::{AssetKind, CarCategory, ChunkKind, PACK_FORMAT_VERSION, PACK_HEADER_SIZE};
use std::path::PathBuf;

fn fixture_dir(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn car_bake_is_deterministic_and_round_trips() {
    let dir = fixture_dir("fixtures/cars/minimal");
    let first = bake_car(&dir).expect("car bake failed");
    let second = bake_car(&dir).expect("second car bake failed");
    assert_eq!(first, second);

    let pack = Pack::from_bytes(&first).expect("car pack failed validation");
    assert_eq!(pack.kind(), AssetKind::Car);
    assert_eq!(pack.manifest().category, Some(CarCategory::Road));

    let physics = pack.car_physics_owned().expect("physics load failed");
    assert_eq!(physics.mass, 1200.0);
    assert_eq!(physics.wheelbase, 2.6);
    assert_eq!(physics.wheels[0].name, "FL");
    assert_eq!(physics.wheels[0].position, [-0.9, 0.34, -1.3]);
    assert!(physics.wheels[0].is_front);
    assert!(physics.wheels[2].driven);
    assert_eq!(physics.engine.torque_curve.len(), 5);
    assert_eq!(physics.drivetrain.gear_ratios.len(), 6);

    let visual = pack.car_visual_owned().expect("visual load failed");
    assert_eq!(visual.meshes.len(), 3);
    assert_eq!(visual.mesh_buffers.len(), 3);
    assert_eq!(visual.wheels.len(), 4);
    assert_eq!(visual.materials.len(), 3);
    assert!(!visual.mesh_buffers[0].indices.is_empty());
    assert!(!visual.mesh_buffers[1].indices.is_empty());
}

#[test]
fn glb_car_bake_emits_category() {
    let input = fixture_dir("fixtures/cars/mazda-rx7/mazda-rx7.glb");
    let bytes = bake_car_glb(&input, CarCategory::Gp67).expect("GLB car bake failed");
    let pack = Pack::from_bytes(&bytes).expect("GLB car pack failed validation");
    assert_eq!(pack.manifest().category, Some(CarCategory::Gp67));
}

#[test]
fn set_category_preserves_chunks_and_sets_category() {
    let dir = fixture_dir("fixtures/cars/minimal");
    let original = bake_car(&dir).expect("car bake failed");
    let before = Pack::from_bytes(&original).expect("original pack parses");
    assert_eq!(before.manifest().category, Some(CarCategory::Road));

    let updated = set_category(&original, CarCategory::Gp67).expect("category rewrite");
    let after = Pack::from_bytes(&updated).expect("rewritten pack parses");
    assert_eq!(after.manifest().category, Some(CarCategory::Gp67));

    for kind in [ChunkKind::Physics, ChunkKind::Visual] {
        let before_bytes = before.chunk_bytes(kind).expect("before chunk");
        let after_bytes = after.chunk_bytes(kind).expect("after chunk");
        assert_eq!(before_bytes, after_bytes, "chunk {kind} bytes changed");
        assert_eq!(
            sim_asset::sha256(before_bytes),
            sim_asset::sha256(after_bytes),
            "chunk {kind} hash changed"
        );
    }
    assert_eq!(
        before
            .manifest()
            .chunks
            .iter()
            .map(|chunk| chunk.hash)
            .collect::<Vec<_>>(),
        after
            .manifest()
            .chunks
            .iter()
            .map(|chunk| chunk.hash)
            .collect::<Vec<_>>(),
        "manifest chunk hashes changed"
    );
    assert_eq!(before.manifest().stats, after.manifest().stats);
}

#[test]
fn car_source_requires_category() {
    let source = std::fs::read_to_string(fixture_dir("fixtures/cars/minimal/car.json"))
        .expect("minimal car source");
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("car source JSON");
    value
        .as_object_mut()
        .expect("car source object")
        .remove("category");

    let dir = std::env::temp_dir().join(format!(
        "lapster-car-without-category-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp car directory");
    std::fs::write(
        dir.join("car.json"),
        serde_json::to_vec(&value).expect("car source bytes"),
    )
    .expect("car source write");

    let error = bake_car(&dir).expect_err("car bake without category should fail");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        format!("{error:#}").contains("category"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn track_bake_is_deterministic_and_round_trips() {
    let dir = fixture_dir("fixtures/tracks/minimal");
    let first = bake_track(&dir).expect("track bake failed");
    let second = bake_track(&dir).expect("second track bake failed");
    assert_eq!(first, second);

    let pack = Pack::from_bytes(&first).expect("track pack failed validation");
    assert_eq!(pack.kind(), AssetKind::Track);
    assert_eq!(pack.manifest().category, None);

    let metadata = pack.track_metadata_owned().expect("metadata load failed");
    assert_eq!(metadata.name, "Minimal Test Track");
    assert_eq!(metadata.spawns.len(), 1);
    assert_eq!(metadata.surfaces.len(), 3);
    assert_eq!(metadata.sectors.len(), 2);

    let collision = pack.track_collision_owned().expect("collision load failed");
    assert_eq!(collision.surface_ids.len(), collision.indices.len() / 3);
    assert!(collision.indices.len() >= 18);

    let visual = pack.track_visual_owned().expect("visual load failed");
    assert_eq!(visual.meshes.len(), 8);
    assert_eq!(visual.mesh_buffers.len(), 8);
    assert_eq!(visual.materials.len(), 7);
    assert_eq!(visual.textures.len(), 1);
    assert_eq!(visual.textures[0].width, 64);
    assert_eq!(visual.textures[0].height, 64);
    assert_eq!(visual.textures[0].rgba8.len(), 64 * 64 * 4);
}

#[test]
fn glb_track_bake_is_deterministic_and_round_trips() {
    let dir = fixture_dir("fixtures/tracks/drift-sketchfab");
    let first = bake_glb_track(&dir).expect("GLB track bake failed");
    let second = bake_glb_track(&dir).expect("second GLB track bake failed");
    assert_eq!(first, second);

    let pack = Pack::from_bytes(&first).expect("GLB track pack failed validation");
    assert_eq!(pack.kind(), AssetKind::Track);

    let metadata = pack.track_metadata_owned().expect("metadata load failed");
    assert_eq!(metadata.name, "Drift Tracks");
    assert_eq!(metadata.spawns.len(), 1);
    assert_eq!(metadata.surfaces.len(), 5);
    assert_eq!(metadata.surfaces[0].name, "asphalt");
    let wall = metadata
        .surfaces
        .iter()
        .find(|surface| surface.name == "wall")
        .expect("wall surface");
    assert_eq!(wall.id, 4);

    let collision = pack.track_collision_owned().expect("collision load failed");
    assert_eq!(collision.surface_ids.len(), collision.indices.len() / 3);
    assert!(collision.indices.len() / 3 > 1000);

    let kerb = metadata
        .surfaces
        .iter()
        .find(|surface| surface.name == "kerb")
        .expect("kerb surface");
    let mut kerb_triangles = 0;
    for (index, triangle) in collision.indices.chunks(3).enumerate() {
        if collision.surface_ids[index] != kerb.id {
            continue;
        }
        let a = Vec3::from_array(collision.vertices[triangle[0] as usize]);
        let b = Vec3::from_array(collision.vertices[triangle[1] as usize]);
        let c = Vec3::from_array(collision.vertices[triangle[2] as usize]);
        let normal = (b - a).cross(c - a).normalize();
        assert!(
            normal.y.abs() > 0.3,
            "mountable kerb collision should not contain vertical walls"
        );
        kerb_triangles += 1;
    }
    assert!(kerb_triangles > 0);

    let visual = pack.track_visual_owned().expect("visual load failed");
    assert_eq!(visual.meshes.len(), 12);
    assert_eq!(visual.mesh_buffers.len(), 12);
    assert_eq!(visual.textures.len(), 10);
    assert_eq!(visual.materials.len(), 10);
    assert!(visual
        .materials
        .iter()
        .any(|material| material.base_color_texture.is_some()));
    for material in &visual.materials {
        if let Some(index) = material.base_color_texture {
            assert!((index as usize) < visual.textures.len());
        }
    }
    for texture in &visual.textures {
        assert_eq!(
            texture.rgba8.len(),
            texture.width as usize * texture.height as usize * 4
        );
    }

    let road = visual
        .meshes
        .iter()
        .position(|mesh| mesh.name == "Drift Track_Drift_Track_Albedo_0")
        .expect("Drift Track road mesh");
    let road = &visual.mesh_buffers[road];
    let t1_seam = road
        .positions
        .iter()
        .zip(&road.uvs)
        .filter(|(position, _)| {
            (position[0] + 118.855).abs() < 0.05 && (position[2] + 41.014).abs() < 0.05
        })
        .map(|(_, uv)| uv[1])
        .collect::<Vec<_>>();
    assert!(
        t1_seam.len() >= 2,
        "expected both sides of the T1 road UV seam"
    );
    for v in t1_seam {
        assert!(
            (v - 0.5349).abs() < 0.001,
            "T1 lane marking UV seam should be continuous, got {v}"
        );
    }
}

#[test]
fn skidpad_fixture_has_baked_textures() {
    let path = fixture_dir("fixtures/tracks/skidpad05/pack.sima");
    let bytes = std::fs::read(&path).expect("skidpad pack");
    let pack = Pack::from_bytes(&bytes).expect("skidpad pack failed validation");
    let visual = pack.track_visual_owned().expect("visual load failed");
    assert_eq!(visual.textures.len(), 10);
    assert!(visual
        .materials
        .iter()
        .any(|material| material.name == "GrassMat" && material.base_color_texture.is_some()));
    assert!(visual
        .materials
        .iter()
        .any(|material| material.name == "transition" && material.base_color_texture.is_some()));
    for texture in &visual.textures {
        assert!(texture.width <= 512 && texture.height <= 512);
        assert_eq!(
            texture.rgba8.len(),
            texture.width as usize * texture.height as usize * 4
        );
    }
}

#[test]
fn skidpad_fixture_includes_solid_wall_collision() {
    let path = fixture_dir("fixtures/tracks/skidpad05/pack.sima");
    let bytes = std::fs::read(&path).expect("skidpad pack");
    let pack = Pack::from_bytes(&bytes).expect("skidpad pack failed validation");
    let metadata = pack.track_metadata_owned().expect("metadata load failed");
    let wall_id = metadata
        .surfaces
        .iter()
        .find(|surface| surface.name.eq_ignore_ascii_case("wall"))
        .map(|surface| surface.id)
        .expect("skidpad wall surface");
    let collision = pack.track_collision_owned().expect("collision load failed");
    let wall_triangles = collision
        .surface_ids
        .iter()
        .filter(|surface| **surface == wall_id)
        .count();
    assert!(
        wall_triangles > 20_000,
        "expected solid wall triangles, found {wall_triangles}"
    );
}

#[test]
fn rejects_bad_pack_header() {
    let bytes = bake_car(&fixture_dir("fixtures/cars/minimal")).expect("car bake failed");

    let mut bad_magic = bytes.clone();
    bad_magic[0] ^= 0xff;
    assert!(matches!(
        Pack::from_bytes(&bad_magic),
        Err(PackError::BadMagic)
    ));

    let mut bad_version = bytes.clone();
    bad_version[4..6].copy_from_slice(&(PACK_FORMAT_VERSION + 1).to_le_bytes());
    assert!(matches!(
        Pack::from_bytes(&bad_version),
        Err(PackError::UnsupportedVersion(_))
    ));

    let mut bad_manifest_offset = bytes.clone();
    bad_manifest_offset[8..16].copy_from_slice(&(PACK_HEADER_SIZE as u64 + 1).to_le_bytes());
    assert!(matches!(
        Pack::from_bytes(&bad_manifest_offset),
        Err(PackError::MisalignedManifest { .. })
    ));

    let truncated = &bytes[..bytes.len() - 1];
    assert!(matches!(
        Pack::from_bytes(truncated),
        Err(PackError::LengthMismatch { .. })
    ));
}

#[test]
fn rejects_corrupt_chunk_without_reading_it() {
    let bytes = bake_track(&fixture_dir("fixtures/tracks/minimal")).expect("track bake failed");
    let offset = {
        let pack = Pack::from_bytes(&bytes).expect("track pack failed validation");
        pack.manifest().chunks[1].offset as usize
    };
    let mut corrupt = bytes;
    corrupt[offset] ^= 0xff;
    assert!(matches!(
        Pack::from_bytes(&corrupt),
        Err(PackError::HashMismatch { .. })
    ));
}
