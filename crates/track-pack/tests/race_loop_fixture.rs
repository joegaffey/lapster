use std::fs;
use std::path::PathBuf;

use sim_asset::Pack;
use sim_types::{AssetKind, ChunkKind};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/tracks/race-loop")
}

#[test]
fn race_loop_pack_matches_source_and_carries_race_data() {
    let dir = fixture_dir();
    let compiled = track_pack::bake_glb_track_dir(&dir).expect("race-loop compiles");
    let checked_in = fs::read(dir.join("pack.sima")).expect("checked-in pack.sima");

    assert_eq!(
        compiled, checked_in,
        "fixtures/tracks/race-loop/pack.sima is stale; re-bake it"
    );

    let pack = Pack::from_bytes(&compiled).expect("valid pack");
    assert_eq!(pack.kind(), AssetKind::Track);
    assert_eq!(pack.manifest().id, "race-loop");

    let race_index = pack.manifest().entrypoints.race.expect("race entrypoint");
    assert_eq!(
        pack.manifest().chunks[race_index as usize].kind,
        ChunkKind::Race
    );

    let race = pack
        .track_race_owned()
        .expect("race accessor")
        .expect("race data");

    assert_eq!(race.version, sim_types::RACE_DATA_VERSION);
    assert_eq!(race.layout.layout_id, "main");
    assert!(race.layout.closed);
    assert_eq!(race.layout.points.len(), 180);
    assert_eq!(race.grid.len(), 8);
    assert_eq!(race.sectors.len(), 4);
    assert_eq!(race.start_finish.position, race.layout.points[0]);
}

#[test]
fn race_loop_compile_is_deterministic() {
    let dir = fixture_dir();
    let first = track_pack::bake_glb_track_dir(&dir).expect("first compile");
    let second = track_pack::bake_glb_track_dir(&dir).expect("second compile");
    assert_eq!(first, second);
}
