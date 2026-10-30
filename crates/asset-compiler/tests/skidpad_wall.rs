use glam::{Quat, Vec3};
use sim_asset::Pack;
use sim_core::{Controls, TrackCollider, Vehicle};
use sim_types::SpawnPoint;
use std::path::PathBuf;

fn fixture_dir(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn skidpad_wall_contact_does_not_tip_or_trap_the_car() {
    let track_bytes =
        std::fs::read(fixture_dir("fixtures/tracks/skidpad05/pack.sima")).expect("skidpad pack");
    let track = Pack::from_bytes(&track_bytes).expect("skidpad pack failed validation");
    let metadata = track.track_metadata_owned().expect("track metadata");
    let collision = track.track_collision_owned().expect("track collision");
    let wall_id = metadata
        .surfaces
        .iter()
        .find(|surface| surface.name.eq_ignore_ascii_case("wall"))
        .map(|surface| surface.id)
        .expect("wall surface");
    let spawn = Vec3::from_array(metadata.spawns[0].position);

    let mut best: Option<(f32, Vec3)> = None;
    for (triangle, surface) in collision.surface_ids.iter().enumerate() {
        if *surface != wall_id {
            continue;
        }
        let base = triangle * 3;
        let a = Vec3::from_array(collision.vertices[collision.indices[base] as usize]);
        let b = Vec3::from_array(collision.vertices[collision.indices[base + 1] as usize]);
        let c = Vec3::from_array(collision.vertices[collision.indices[base + 2] as usize]);
        let normal = (b - a).cross(c - a).normalize_or_zero();
        if normal.y.abs() > 0.5 {
            continue;
        }
        let centroid = (a + b + c) / 3.0;
        let dx = centroid.x - spawn.x;
        let dz = centroid.z - spawn.z;
        let distance = dx * dx + dz * dz;
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, centroid));
        }
    }
    let (_, wall_point) = best.expect("vertical wall triangle");
    let inward = Vec3::new(spawn.x - wall_point.x, 0.0, spawn.z - wall_point.z).normalize_or_zero();
    assert!(inward.length_squared() > 0.5, "wall point equals spawn");

    let start = wall_point + inward * 10.0;
    let collider = TrackCollider::from_collision(&collision, &metadata);
    let ground = collider
        .raycast(Vec3::new(start.x, 100.0, start.z), Vec3::NEG_Y, 500.0)
        .expect("ground under wall approach");
    let car_position = Vec3::new(start.x, ground.point.y, start.z);
    let toward_wall = -inward;
    let yaw = (-toward_wall.x).atan2(-toward_wall.z);
    let spawn = SpawnPoint {
        name: String::from("wall-approach"),
        position: car_position.to_array(),
        rotation_xyzw: Quat::from_rotation_y(yaw).to_array(),
    };

    let car_bytes =
        std::fs::read(fixture_dir("fixtures/cars/minimal/pack.sima")).expect("minimal car pack");
    let car = Pack::from_bytes(&car_bytes).expect("minimal car pack failed validation");
    let physics = car.car_physics_owned().expect("car physics");
    let mut vehicle = Vehicle::new(physics, collider, &spawn);
    for _ in 0..3000 {
        vehicle.step(Controls {
            throttle: 1.0,
            gear: 1,
            ..Controls::default()
        });
    }

    let up = vehicle.state.rotation * Vec3::Y;
    assert!(
        up.y > 0.7,
        "car tipped against the wall: up={up:?} position={:?}",
        vehicle.state.position
    );
    assert!(
        vehicle.state.position.y < car_position.y + 4.0,
        "car climbed the wall: start_y={} car_y={}",
        car_position.y,
        vehicle.state.position.y
    );
    let distance_inside = (vehicle.state.position - wall_point).dot(inward);
    assert!(
        distance_inside > 0.0,
        "car crossed the wall: inside_distance={distance_inside} position={:?}",
        vehicle.state.position
    );
    assert!(
        distance_inside < 4.0,
        "car did not reach the wall: inside_distance={distance_inside}"
    );
}
