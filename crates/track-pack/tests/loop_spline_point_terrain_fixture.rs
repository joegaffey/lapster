use std::fs;
use std::path::PathBuf;

use sim_asset::Pack;
use sim_types::{AssetKind, ChunkKind, LayoutDirection, RACE_DATA_VERSION};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/tracks/loop-spline-point-terrain")
}

#[test]
fn loop_spline_point_terrain_pack_carries_race_data() {
    let dir = fixture_dir();
    let bytes = fs::read(dir.join("pack.sima")).expect("checked-in pack.sima");
    let pack = Pack::from_bytes(&bytes).expect("valid pack");
    assert_eq!(pack.kind(), AssetKind::Track);
    assert_eq!(pack.manifest().id, "loop-spline-point-terrain");

    let metadata = pack.track_metadata_owned().expect("metadata");
    assert_eq!(metadata.name, "Loop Spline Point Terrain");
    assert_eq!(metadata.spawns.len(), 1);
    assert_eq!(metadata.spawns[0].name, "start");

    let race_index = pack.manifest().entrypoints.race.expect("race entrypoint");
    assert_eq!(
        pack.manifest().chunks[race_index as usize].kind,
        ChunkKind::Race
    );

    let race = pack
        .track_race_owned()
        .expect("race accessor")
        .expect("race data");

    assert_eq!(race.version, RACE_DATA_VERSION);
    assert_eq!(race.layout.layout_id, "section-1");
    assert!(race.layout.closed);
    assert_eq!(race.layout.direction, LayoutDirection::Forward);
    assert_eq!(race.layout.points.len(), 471);
    assert_eq!(race.layout.half_width, 5.0);
    assert_eq!(race.start_finish.section_id, "section-1");
    assert!((race.start_finish.fraction - 0.123_154_98).abs() < 1e-6);
    assert_eq!(race.grid.len(), 10);
    assert_eq!(race.sectors.len(), 4);
    assert_eq!(race.start_finish.position, race.layout.points[0]);
}
