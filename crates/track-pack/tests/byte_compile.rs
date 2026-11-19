use std::fs;
use std::path::PathBuf;

use sim_asset::Pack;
use sim_types::AssetKind;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/tracks/drift-sketchfab")
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
