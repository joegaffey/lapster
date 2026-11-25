use std::fs;
use std::path::PathBuf;

use serde_json::json;
use sim_asset::Pack;
use sim_types::{AssetKind, ChunkKind};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/tracks/drift-sketchfab")
}

fn fixture() -> (Vec<u8>, Vec<u8>) {
    let dir = fixture_dir();
    (
        fs::read(dir.join("track.json")).expect("track.json"),
        fs::read(dir.join("drift_tracks.glb")).expect("drift_tracks.glb"),
    )
}

fn valid_race() -> serde_json::Value {
    json!({
        "version": 1,
        "layout": {
            "layout_id": "main",
            "closed": true,
            "direction": "forward",
            "points": [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, 10.0]],
            "half_width": 5.0
        },
        "start_finish": {
            "section_id": "main",
            "fraction": 0.0,
            "position": [0.0, 0.0, 0.0],
            "rotation_xyzw": [0.0, 0.0, 0.0, 1.0]
        },
        "grid": [
            {
                "name": "grid_0",
                "position": [0.0, 0.0, 2.0],
                "rotation_xyzw": [0.0, 0.0, 0.0, 1.0]
            }
        ],
        "sectors": [
            { "name": "sector_1", "start_fraction": 0.0, "end_fraction": 0.5 },
            { "name": "sector_2", "start_fraction": 0.5, "end_fraction": 1.0 }
        ]
    })
}

fn grid_slots(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|index| {
            json!({
                "name": format!("grid_{index}"),
                "position": [index as f32 * 0.5, 0.0, 2.0],
                "rotation_xyzw": [0.0, 0.0, 0.0, 1.0]
            })
        })
        .collect()
}

fn source_with_race(race: &serde_json::Value) -> Vec<u8> {
    let (track_json, _) = fixture();
    let mut source: serde_json::Value =
        serde_json::from_slice(&track_json).expect("parse track.json");
    source["schema"] = json!("sim-track-glb-v2");
    source["race"] = race.clone();
    serde_json::to_vec(&source).expect("serialize track.json")
}

fn bake(source: &[u8]) -> Vec<u8> {
    let (_, glb) = fixture();
    track_pack::bake_glb_track_bytes(source, &glb, "race-test").expect("track compiles")
}

#[test]
fn byte_compile_matches_directory_compile() {
    let dir = fixture_dir();
    let from_dir = track_pack::bake_glb_track_dir(&dir).expect("directory compile");
    let track_json = fs::read(dir.join("track.json")).expect("track.json");
    let glb = fs::read(dir.join("drift_tracks.glb")).expect("drift_tracks.glb");
    let from_bytes = track_pack::bake_glb_track_bytes(&track_json, &glb, "drift-sketchfab")
        .expect("byte compile");
    assert_eq!(from_dir, from_bytes);

    let pack = Pack::from_bytes(&from_bytes).expect("valid pack");
    assert_eq!(pack.kind(), AssetKind::Track);
    assert_eq!(pack.manifest().id, "drift-sketchfab");
    let metadata = pack.track_metadata_owned().expect("metadata");
    assert_eq!(metadata.name, "Drift Tracks");
}

#[test]
fn v1_source_without_race_has_no_race_chunk() {
    let (track_json, _) = fixture();
    let bytes = bake(&track_json);
    let pack = Pack::from_bytes(&bytes).expect("valid pack");
    assert!(pack.manifest().entrypoints.race.is_none());
    assert!(pack
        .manifest()
        .chunks
        .iter()
        .all(|chunk| chunk.kind != ChunkKind::Race));
    assert_eq!(pack.track_race_owned().expect("race accessor"), None);
}

#[test]
fn v2_source_with_valid_race_round_trips() {
    let source = source_with_race(&valid_race());
    let bytes = bake(&source);
    let pack = Pack::from_bytes(&bytes).expect("valid pack");
    let race_index = pack.manifest().entrypoints.race.expect("race entrypoint");
    assert_eq!(
        pack.manifest().chunks[race_index as usize].kind,
        ChunkKind::Race
    );
    let race = pack
        .track_race_owned()
        .expect("race accessor")
        .expect("race data");
    assert_eq!(race.layout.layout_id, "main");
    assert!(race.layout.closed);
    assert_eq!(race.layout.points.len(), 3);
    assert_eq!(race.grid.len(), 1);
    assert_eq!(race.sectors.len(), 2);
}

#[test]
fn grid_accepts_ten_slots_and_rejects_eleven() {
    let mut race = valid_race();
    race["grid"] = json!(grid_slots(10));
    let bytes = bake(&source_with_race(&race));
    let pack = Pack::from_bytes(&bytes).expect("valid pack");
    let race = pack
        .track_race_owned()
        .expect("race accessor")
        .expect("race data");
    assert_eq!(race.grid.len(), 10);

    let mut too_many = valid_race();
    too_many["grid"] = json!(grid_slots(11));
    let source = source_with_race(&too_many);
    let (_, glb) = fixture();
    let result = track_pack::bake_glb_track_bytes(&source, &glb, "race-test");
    assert!(result.is_err(), "expected 11 grid slots to be rejected");
}

#[test]
fn invalid_race_is_rejected() {
    let mut not_closed = valid_race();
    not_closed["layout"]["closed"] = json!(false);
    let mut too_few_points = valid_race();
    too_few_points["layout"]["points"] = json!([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]]);
    let mut bad_half_width = valid_race();
    bad_half_width["layout"]["half_width"] = json!(0.0);
    let mut start_finish_mismatch = valid_race();
    start_finish_mismatch["start_finish"]["position"] = json!([1.0, 0.0, 0.0]);
    let mut empty_grid = valid_race();
    empty_grid["grid"] = json!([]);
    let mut gap_in_sectors = valid_race();
    gap_in_sectors["sectors"] =
        json!([{ "name": "sector_1", "start_fraction": 0.0, "end_fraction": 0.5 }]);

    for (name, race) in [
        ("not closed", not_closed),
        ("too few points", too_few_points),
        ("bad half_width", bad_half_width),
        ("start/finish mismatch", start_finish_mismatch),
        ("empty grid", empty_grid),
        ("sectors do not cover", gap_in_sectors),
    ] {
        let source = source_with_race(&race);
        let (_, glb) = fixture();
        let result = track_pack::bake_glb_track_bytes(&source, &glb, "race-test");
        assert!(result.is_err(), "expected {name} to be rejected");
    }
}

#[test]
fn start_finish_fraction_out_of_range_is_rejected() {
    for fraction in [-0.1, 1.0] {
        let mut race = valid_race();
        race["start_finish"]["fraction"] = json!(fraction);
        let source = source_with_race(&race);
        let (_, glb) = fixture();
        let result = track_pack::bake_glb_track_bytes(&source, &glb, "race-test");
        assert!(
            result.is_err(),
            "expected start/finish fraction {fraction} to be rejected"
        );
    }
}

#[test]
fn grid_slot_outside_route_width_is_rejected() {
    let mut race = valid_race();
    race["grid"] = json!([
        {
            "name": "grid_0",
            "position": [5.0, 0.0, 50.0],
            "rotation_xyzw": [0.0, 0.0, 0.0, 1.0]
        }
    ]);
    let source = source_with_race(&race);
    let (_, glb) = fixture();
    let result = track_pack::bake_glb_track_bytes(&source, &glb, "race-test");
    assert!(
        result.is_err(),
        "expected grid slot outside route width to be rejected"
    );
}

#[test]
fn grid_slot_near_road_edge_compiles() {
    let mut race = valid_race();
    race["grid"][0]["position"] = json!([0.0, 0.0, 4.9]);
    let source = source_with_race(&race);
    let bytes = bake(&source);
    let pack = Pack::from_bytes(&bytes).expect("valid pack");
    let race = pack
        .track_race_owned()
        .expect("race accessor")
        .expect("race data");
    assert_eq!(race.grid[0].position, [0.0, 0.0, 4.9]);
}

#[test]
fn identical_inputs_produce_identical_bytes() {
    let source = source_with_race(&valid_race());
    assert_eq!(bake(&source), bake(&source));
}

#[test]
fn changing_race_data_changes_source_hash() {
    let first = bake(&source_with_race(&valid_race()));
    let mut changed = valid_race();
    changed["layout"]["half_width"] = json!(6.0);
    let second = bake(&source_with_race(&changed));

    let first = Pack::from_bytes(&first).expect("valid pack");
    let second = Pack::from_bytes(&second).expect("valid pack");
    assert_ne!(first.manifest().source_hash, second.manifest().source_hash);
}
