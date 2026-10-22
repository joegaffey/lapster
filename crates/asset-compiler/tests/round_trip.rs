use asset_compiler::bake::{bake_car, bake_track};
use sim_asset::{Pack, PackError};
use sim_types::{AssetKind, PACK_FORMAT_VERSION, PACK_HEADER_SIZE};
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
fn track_bake_is_deterministic_and_round_trips() {
    let dir = fixture_dir("fixtures/tracks/minimal");
    let first = bake_track(&dir).expect("track bake failed");
    let second = bake_track(&dir).expect("second track bake failed");
    assert_eq!(first, second);

    let pack = Pack::from_bytes(&first).expect("track pack failed validation");
    assert_eq!(pack.kind(), AssetKind::Track);

    let metadata = pack.track_metadata_owned().expect("metadata load failed");
    assert_eq!(metadata.name, "Minimal Test Track");
    assert_eq!(metadata.spawns.len(), 1);
    assert_eq!(metadata.surfaces.len(), 3);
    assert_eq!(metadata.sectors.len(), 2);

    let collision = pack.track_collision_owned().expect("collision load failed");
    assert_eq!(collision.surface_ids.len(), collision.indices.len() / 3);
    assert!(collision.indices.len() >= 18);

    let visual = pack.track_visual_owned().expect("visual load failed");
    assert_eq!(visual.meshes.len(), 7);
    assert_eq!(visual.mesh_buffers.len(), 7);
    assert_eq!(visual.materials.len(), 6);
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
