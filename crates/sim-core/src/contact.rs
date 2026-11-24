//! Deterministic car-to-car contact for co-simulated vehicles.
//!
//! The worker steps the player and every AI on the same fixed tick, so contact
//! between them is resolved here as a bounded pairwise impulse pass. Each car
//! is treated as a 2D oriented box in XZ with a vertical gate so cars on
//! different terrain heights do not collide. The existing per-vehicle track and
//! wall path is untouched.

use glam::Vec3;

use crate::vehicle::Vehicle;

/// Vertical separation above which two cars are considered apart.
pub const CAR_VERTICAL_GATE: f32 = 1.0;

const RESTITUTION: f32 = 0.05;
const FRICTION: f32 = 0.4;
const PENETRATION_SLOP: f32 = 0.005;
const POSITION_CORRECTION: f32 = 0.6;
const MAX_ITERATIONS: usize = 2;
/// Radius/skin around each body probe point.
const PROBE_RADIUS: f32 = 0.01;
/// Bound on simultaneous probe contacts per pass.
const MAX_MANIFOLD_CONTACTS: usize = 4;
/// Per-pass cap on the accumulated positional correction.
const MAX_POSITION_CORRECTION: f32 = 0.02;
/// Tire-yaw resistance for car-to-car contact. The linear impulse is applied in
/// full; only the angular velocity change is scaled so a tire-constrained car
/// does not spin like a free rigid body on a side hit. This is a feel constant,
/// not a physical tire model.
const CONTACT_ANGULAR_SCALE: f32 = 0.5;

/// One car-to-car contact produced by the pairwise resolver.
#[derive(Debug, Clone, Copy)]
pub struct CarContact {
    /// First car slot. By convention `0` is the player and `1..=n` are AI.
    pub a: u32,
    /// Second car slot.
    pub b: u32,
    /// World-space contact point.
    pub point: Vec3,
    /// Contact normal pointing from `a` toward `b`.
    pub normal: Vec3,
    /// Closing speed along the normal before the impulse.
    pub relative_speed: f32,
    /// Simulation tick the contact was observed on.
    pub tick: u64,
}

/// Resolves contact between two vehicles using default slots `0` and `1`.
///
/// Returns `None` when the cars are not overlapping. The impulse and positional
/// correction are applied in place when a contact is found.
pub fn resolve_car_contact(a: &mut Vehicle, b: &mut Vehicle) -> Option<CarContact> {
    resolve_pair(a, b, 0, 1)
}

/// Resolves contact between two vehicles and labels the contact with their
/// worker slot ids.
pub fn resolve_car_contact_with_slots(
    a: &mut Vehicle,
    b: &mut Vehicle,
    slot_a: u32,
    slot_b: u32,
) -> Option<CarContact> {
    resolve_pair(a, b, slot_a, slot_b)
}

fn resolve_pair(a: &mut Vehicle, b: &mut Vehicle, slot_a: u32, slot_b: u32) -> Option<CarContact> {
    resolve_pair_with_angular_scale(a, b, slot_a, slot_b, CONTACT_ANGULAR_SCALE)
}

fn resolve_pair_with_angular_scale(
    a: &mut Vehicle,
    b: &mut Vehicle,
    slot_a: u32,
    slot_b: u32,
    angular_scale: f32,
) -> Option<CarContact> {
    let mut contact = None;
    for _ in 0..MAX_ITERATIONS {
        let Some(manifold) = detect(a, b) else {
            break;
        };
        if contact.is_none() {
            let deepest = &manifold[0];
            let relative = velocity_at(b, deepest.point) - velocity_at(a, deepest.point);
            let normal_speed = relative.dot(deepest.normal);
            contact = Some(CarContact {
                a: slot_a,
                b: slot_b,
                point: deepest.point,
                normal: deepest.normal,
                relative_speed: (-normal_speed).max(0.0),
                tick: a.state.tick.max(b.state.tick),
            });
        }
        for hit in &manifold {
            apply_response(a, b, hit.point, hit.normal, angular_scale);
        }
        separate(a, b, &manifold);
    }
    contact
}

/// Planar oriented box for one car, built from its own contact extents.
struct BodyBox {
    center: Vec3,
    forward: Vec3,
    right: Vec3,
    half_length: f32,
    half_width: f32,
}

impl BodyBox {
    fn from_vehicle(vehicle: &Vehicle) -> Self {
        let collision = vehicle.collision_box();
        let (half_length, half_width) = vehicle.contact_half_extents();
        let (forward, right) = planar_axes(vehicle);
        // The visual AABB can be asymmetric front-to-back (for example a long
        // rear overhang), so carry its local XZ centre into world space.
        let local_offset = Vec3::new(collision.center_x, 0.0, collision.center_z);
        let rotated_offset = vehicle.state.rotation * local_offset;
        let center = planar_center(vehicle) + Vec3::new(rotated_offset.x, 0.0, rotated_offset.z);
        Self {
            center,
            forward,
            right,
            half_length,
            half_width,
        }
    }

    /// The eight body probes used for wall contact, placed on this car's own
    /// contact box: front/rear centre, four corners, and two side midpoints.
    fn probe_points(&self) -> [Vec3; 8] {
        let length = self.half_length;
        let width = self.half_width;
        let forward = self.forward;
        let right = self.right;
        let center = self.center;
        [
            center + forward * length,
            center - right * width + forward * length,
            center + right * width + forward * length,
            center - right * width,
            center + right * width,
            center - right * width - forward * length,
            center + right * width - forward * length,
            center - forward * length,
        ]
    }

    /// Closest point on this box to `probe`, the outward normal at that point,
    /// and the penetration of a sphere of [`PROBE_RADIUS`] around the probe.
    fn closest_contact(&self, probe: Vec3) -> Option<(Vec3, Vec3, f32)> {
        let offset = probe - self.center;
        let local_right = offset.dot(self.right);
        let local_forward = offset.dot(self.forward);
        let inside =
            local_right.abs() <= self.half_width && local_forward.abs() <= self.half_length;
        if inside {
            // Probe is inside the box: expel it through the nearest face.
            let mut distance = self.half_width - local_right;
            let mut escape = self.right;
            if local_right + self.half_width < distance {
                distance = local_right + self.half_width;
                escape = -self.right;
            }
            if self.half_length - local_forward < distance {
                distance = self.half_length - local_forward;
                escape = self.forward;
            }
            if local_forward + self.half_length < distance {
                distance = local_forward + self.half_length;
                escape = -self.forward;
            }
            Some((probe, escape, PROBE_RADIUS + distance.max(0.0)))
        } else {
            let closest = self.center
                + self.right * local_right.clamp(-self.half_width, self.half_width)
                + self.forward * local_forward.clamp(-self.half_length, self.half_length);
            let delta = probe - closest;
            let distance = delta.length();
            if distance > PROBE_RADIUS {
                return None;
            }
            let normal = if distance > 1e-6 {
                delta / distance
            } else {
                self.right
            };
            Some((closest, normal, PROBE_RADIUS - distance))
        }
    }
}

struct ContactHit {
    point: Vec3,
    normal: Vec3,
    penetration: f32,
}

fn detect(a: &Vehicle, b: &Vehicle) -> Option<Vec<ContactHit>> {
    if (a.state.position.y - b.state.position.y).abs() > CAR_VERTICAL_GATE {
        return None;
    }

    let box_a = BodyBox::from_vehicle(a);
    let box_b = BodyBox::from_vehicle(b);

    let offset = box_b.center - box_a.center;
    let planar_offset = Vec3::new(offset.x, 0.0, offset.z);
    let radius_a =
        (box_a.half_length * box_a.half_length + box_a.half_width * box_a.half_width).sqrt();
    let radius_b =
        (box_b.half_length * box_b.half_length + box_b.half_width * box_b.half_width).sqrt();
    let max_distance = radius_a + radius_b + PROBE_RADIUS;
    if planar_offset.length_squared() > max_distance * max_distance {
        return None;
    }

    let delta = planar_offset.normalize_or_zero();
    let direction = if delta.length_squared() > 1e-8 {
        delta
    } else {
        Vec3::X
    };
    let point_y = (a.state.position.y + b.state.position.y) * 0.5;
    let mut hits = Vec::with_capacity(16);

    // A's probes against B's box, then B's probes against A's box, in a fixed
    // order. Every normal is oriented from slot `a` toward slot `b`.
    for probe in box_a.probe_points() {
        if let Some((closest, surface_normal, penetration)) = box_b.closest_contact(probe) {
            let mut normal = -surface_normal;
            if normal.dot(direction) < 0.0 {
                normal = -normal;
            }
            hits.push(ContactHit {
                point: Vec3::new(closest.x, point_y, closest.z),
                normal,
                penetration,
            });
        }
    }
    for probe in box_b.probe_points() {
        if let Some((closest, surface_normal, penetration)) = box_a.closest_contact(probe) {
            let mut normal = surface_normal;
            if normal.dot(direction) < 0.0 {
                normal = -normal;
            }
            hits.push(ContactHit {
                point: Vec3::new(closest.x, point_y, closest.z),
                normal,
                penetration,
            });
        }
    }

    if hits.is_empty() {
        return None;
    }
    // Only genuine interpenetration drives the response. Probes that merely
    // touch the probe skin on the other body are ignored so the manifold stays
    // symmetric under swapping the two cars.
    hits.retain(|hit| hit.penetration > PROBE_RADIUS);
    if hits.is_empty() {
        return None;
    }
    // Deepest first; stable sort keeps insertion order for ties so the
    // manifold is deterministic. Only a bounded subset is resolved per pass.
    hits.sort_by(|left, right| right.penetration.total_cmp(&left.penetration));
    hits.truncate(MAX_MANIFOLD_CONTACTS);
    Some(hits)
}

fn planar_axes(vehicle: &Vehicle) -> (Vec3, Vec3) {
    let forward = vehicle.state.rotation * Vec3::NEG_Z;
    let mut forward = Vec3::new(forward.x, 0.0, forward.z);
    if forward.length_squared() < 1e-6 {
        forward = Vec3::NEG_Z;
    } else {
        forward = forward.normalize();
    }
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let right = if right.length_squared() < 1e-6 {
        Vec3::X
    } else {
        right
    };
    (forward, right)
}

fn planar_center(vehicle: &Vehicle) -> Vec3 {
    Vec3::new(vehicle.state.position.x, 0.0, vehicle.state.position.z)
}

fn velocity_at(vehicle: &Vehicle, point: Vec3) -> Vec3 {
    let lever = point - vehicle.center_of_gravity_world();
    vehicle.state.velocity + vehicle.state.angular_velocity.cross(lever)
}

fn effective_inverse_mass(vehicle: &Vehicle, point: Vec3, normal: Vec3, angular_scale: f32) -> f32 {
    let lever = point - vehicle.center_of_gravity_world();
    let angular = vehicle.inv_inertia_apply(lever.cross(normal)).cross(lever);
    vehicle.inverse_mass() + angular_scale * normal.dot(angular)
}

fn apply_impulse(vehicle: &mut Vehicle, point: Vec3, impulse: Vec3, angular_scale: f32) {
    let lever = point - vehicle.center_of_gravity_world();
    let inverse_mass = vehicle.inverse_mass();
    vehicle.state.velocity += impulse * inverse_mass;
    let angular = vehicle.inv_inertia_apply(lever.cross(impulse)) * angular_scale;
    vehicle.state.angular_velocity += angular;
}

fn apply_response(a: &mut Vehicle, b: &mut Vehicle, point: Vec3, normal: Vec3, angular_scale: f32) {
    let relative = velocity_at(b, point) - velocity_at(a, point);
    let normal_speed = relative.dot(normal);
    if normal_speed >= 0.0 {
        return;
    }
    let inverse_mass = effective_inverse_mass(a, point, normal, angular_scale)
        + effective_inverse_mass(b, point, normal, angular_scale);
    if inverse_mass <= 1e-6 {
        return;
    }
    let normal_impulse = -(1.0 + RESTITUTION) * normal_speed / inverse_mass;
    if normal_impulse <= 0.0 {
        return;
    }
    let impulse = normal * normal_impulse;
    apply_impulse(a, point, -impulse, angular_scale);
    apply_impulse(b, point, impulse, angular_scale);

    let post_relative = velocity_at(b, point) - velocity_at(a, point);
    // Contact is planar: ignore the vertical component so friction cannot pump
    // energy into the body through the contact lever.
    let post_relative = Vec3::new(post_relative.x, 0.0, post_relative.z);
    let tangent = post_relative - normal * post_relative.dot(normal);
    let tangent_speed = tangent.length();
    if tangent_speed <= 1e-4 {
        return;
    }
    let tangent = tangent / tangent_speed;
    let tangent_inverse_mass = effective_inverse_mass(a, point, tangent, angular_scale)
        + effective_inverse_mass(b, point, tangent, angular_scale);
    if tangent_inverse_mass <= 1e-6 {
        return;
    }
    let friction_limit = FRICTION * normal_impulse;
    let tangent_impulse =
        (-post_relative.dot(tangent) / tangent_inverse_mass).clamp(-friction_limit, friction_limit);
    let friction = tangent * tangent_impulse;
    apply_impulse(a, point, -friction, angular_scale);
    apply_impulse(b, point, friction, angular_scale);
}

fn separate(a: &mut Vehicle, b: &mut Vehicle, manifold: &[ContactHit]) {
    let inverse_a = a.inverse_mass();
    let inverse_b = b.inverse_mass();
    let inverse_sum = inverse_a + inverse_b;
    if inverse_sum <= 1e-6 {
        return;
    }
    let mut correction = Vec3::ZERO;
    for hit in manifold {
        let depth = hit.penetration - PENETRATION_SLOP;
        if depth > 0.0 {
            correction += hit.normal * (depth * POSITION_CORRECTION);
        }
    }
    let length = correction.length();
    if length <= 1e-6 {
        return;
    }
    if length > MAX_POSITION_CORRECTION {
        correction *= MAX_POSITION_CORRECTION / length;
    }
    a.state.position -= correction * (inverse_a / inverse_sum);
    b.state.position += correction * (inverse_b / inverse_sum);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collider::TrackCollider;
    use glam::Quat;
    use sim_types::{
        Aabb, AeroSpec, BrakeSpec, CarPhysics, DriveLayout, DrivetrainSpec, EngineSpec,
        PacejkaCoefficients, SpawnPoint, SteeringSpec, SurfaceSpec, SuspensionSpec, TireSpec,
        TorquePoint, TrackCollision, TrackMetadata, WheelPhysics,
    };

    fn physics() -> CarPhysics {
        let suspension = SuspensionSpec {
            travel: 0.15,
            spring_rate: 55_000.0,
            damper_bump: 7_000.0,
            damper_rebound: 9_000.0,
            anti_roll: 18_000.0,
            ride_height: 0.34,
        };
        let tire = TireSpec {
            lateral: PacejkaCoefficients {
                b: 10.0,
                c: 1.9,
                d: 1.0,
                e: 0.97,
            },
            longitudinal: PacejkaCoefficients {
                b: 12.0,
                c: 1.65,
                d: 1.0,
                e: 0.9,
            },
            relaxation_length: 0.3,
            peak_slip_ratio: 0.12,
            peak_slip_angle: 0.12,
            friction_scale: 1.0,
        };
        let wheels = [
            ("FL", [-0.73, 0.34, -1.2125], true, true, false),
            ("FR", [0.73, 0.34, -1.2125], true, false, false),
            ("RL", [-0.73, 0.34, 1.2125], false, true, true),
            ("RR", [0.73, 0.34, 1.2125], false, false, true),
        ]
        .map(|(name, position, front, left, driven)| WheelPhysics {
            name: String::from(name),
            is_front: front,
            is_left: left,
            position,
            radius: 0.33,
            width: 0.225,
            steerable: front,
            driven,
            suspension: SuspensionSpec {
                anti_roll: if front { 18_000.0 } else { 8_000.0 },
                ..suspension
            },
            tire: TireSpec {
                friction_scale: if front { 1.0 } else { 1.05 },
                ..tire
            },
        });
        CarPhysics {
            mass: 1_270.0,
            inertia: [650.0, 1_300.0, 450.0],
            center_of_gravity: [0.0, 0.40, 0.0],
            wheelbase: 2.425,
            track_width: [1.46, 1.46],
            wheels,
            engine: EngineSpec {
                idle_rpm: 900.0,
                redline_rpm: 8_000.0,
                torque_curve: vec![
                    TorquePoint {
                        rpm: 1_000.0,
                        torque_nm: 180.0,
                    },
                    TorquePoint {
                        rpm: 4_500.0,
                        torque_nm: 314.0,
                    },
                    TorquePoint {
                        rpm: 7_500.0,
                        torque_nm: 250.0,
                    },
                ],
            },
            drivetrain: DrivetrainSpec {
                gear_ratios: vec![3.483, 2.015, 1.391, 1.0, 0.719, 0.6],
                final_drive: 4.1,
                clutch_max_torque: 400.0,
                drive_layout: DriveLayout::Rear,
            },
            brakes: BrakeSpec {
                max_torque: [2_200.0, 2_200.0, 1_400.0, 1_400.0],
                bias: 0.6,
            },
            steering: SteeringSpec {
                max_angle: 0.6,
                ratio: 14.0,
                caster: 0.09,
                kingpin_inclination: 0.12,
            },
            aero: AeroSpec {
                drag_coefficient: 0.29,
                frontal_area: 1.85,
                downforce_coefficient: 0.0,
            },
        }
    }

    fn collider() -> TrackCollider {
        let collision = TrackCollision {
            vertices: vec![
                [-100.0, 0.0, -100.0],
                [-100.0, 0.0, 100.0],
                [100.0, 0.0, 100.0],
                [100.0, 0.0, -100.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: Aabb {
                min: [-100.0, 0.0, -100.0],
                max: [100.0, 0.0, 100.0],
            },
        };
        let metadata = TrackMetadata {
            name: String::from("contact"),
            length: 40.0,
            spawns: Vec::new(),
            pit_boxes: Vec::new(),
            cameras: Vec::new(),
            sectors: Vec::new(),
            surfaces: vec![SurfaceSpec {
                id: 0,
                name: String::from("asphalt"),
                friction: 1.0,
                roughness: 0.0,
            }],
        };
        TrackCollider::from_collision(&collision, &metadata)
    }

    fn vehicle_with_physics_at(physics: CarPhysics, position: Vec3) -> Vehicle {
        let spawn = SpawnPoint {
            name: String::from("contact"),
            position: position.to_array(),
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        };
        let mut vehicle = Vehicle::new(physics, collider(), &spawn);
        vehicle.state.position = position;
        vehicle.state.rotation = Quat::IDENTITY;
        vehicle.state.velocity = Vec3::ZERO;
        vehicle.state.angular_velocity = Vec3::ZERO;
        vehicle.state.tick = 0;
        vehicle
    }

    fn vehicle_at(position: Vec3) -> Vehicle {
        vehicle_with_physics_at(physics(), position)
    }

    fn vehicle_with_wheelbase_at(wheelbase: f32, track: f32, position: Vec3) -> Vehicle {
        let mut physics = physics();
        physics.wheelbase = wheelbase;
        physics.track_width = [track, track];
        vehicle_with_physics_at(physics, position)
    }

    fn assert_finite(vehicle: &Vehicle) {
        assert!(vehicle.state.position.is_finite());
        assert!(vehicle.state.velocity.is_finite());
        assert!(vehicle.state.angular_velocity.is_finite());
        assert!(vehicle.state.rotation.is_finite());
    }

    /// Largest remaining box penetration of either car's probes (0 when the
    /// bodies are clear). The probe skin is removed so this measures the
    /// physical overlap rather than the contact tolerance.
    fn max_probe_penetration(a: &Vehicle, b: &Vehicle) -> f32 {
        let box_a = BodyBox::from_vehicle(a);
        let box_b = BodyBox::from_vehicle(b);
        let mut max = 0.0f32;
        for probe in box_a.probe_points() {
            if let Some((_, _, penetration)) = box_b.closest_contact(probe) {
                max = max.max(penetration - PROBE_RADIUS);
            }
        }
        for probe in box_b.probe_points() {
            if let Some((_, _, penetration)) = box_a.closest_contact(probe) {
                max = max.max(penetration - PROBE_RADIUS);
            }
        }
        max
    }

    #[test]
    fn head_on_overlap_produces_separating_impulse() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(0.0, 0.25, 3.3));
        a.state.velocity = Vec3::new(0.0, 0.0, 5.0);
        b.state.velocity = Vec3::new(0.0, 0.0, -5.0);
        a.state.tick = 42;
        b.state.tick = 42;

        let contact = resolve_car_contact(&mut a, &mut b).expect("head-on contact");

        assert!(contact.normal.z > 0.9, "normal={:?}", contact.normal);
        assert!(contact.relative_speed > 5.0);
        assert_eq!(contact.tick, 42);
        let separation =
            (velocity_at(&b, contact.point) - velocity_at(&a, contact.point)).dot(contact.normal);
        assert!(
            separation >= -1e-3,
            "contact point still closing after impulse: {separation}"
        );
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn side_and_rear_contacts_choose_and_separate() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(1.5, 0.25, 0.0));
        a.state.velocity = Vec3::new(5.0, 0.0, 0.0);
        b.state.velocity = Vec3::new(-5.0, 0.0, 0.0);

        let side = resolve_car_contact(&mut a, &mut b).expect("side contact");
        assert!(side.normal.x > 0.9, "normal={:?}", side.normal);
        let side_separation =
            (velocity_at(&b, side.point) - velocity_at(&a, side.point)).dot(side.normal);
        assert!(
            side_separation >= -1e-3,
            "side still closing: {side_separation}"
        );
        assert_finite(&a);
        assert_finite(&b);

        let mut rear = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut ahead = vehicle_at(Vec3::new(0.0, 0.25, 3.3));
        rear.state.velocity = Vec3::new(0.0, 0.0, 5.0);
        ahead.state.velocity = Vec3::ZERO;

        let rear_contact = resolve_car_contact(&mut rear, &mut ahead).expect("rear contact");
        assert!(
            rear_contact.normal.z > 0.9,
            "normal={:?}",
            rear_contact.normal
        );
        let rear_separation = (velocity_at(&ahead, rear_contact.point)
            - velocity_at(&rear, rear_contact.point))
        .dot(rear_contact.normal);
        assert!(
            rear_separation >= -1e-3,
            "rear still closing: {rear_separation}"
        );
        assert_finite(&rear);
        assert_finite(&ahead);
    }

    #[test]
    fn resting_overlap_settles_without_jitter() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(0.0, 0.25, 3.3));
        let (a_length, _) = a.contact_half_extents();
        let (b_length, _) = b.contact_half_extents();
        let combined = a_length + b_length;

        for _ in 0..180 {
            resolve_car_contact(&mut a, &mut b);
        }

        let separation = b.state.position.z - a.state.position.z;
        assert!(
            separation >= combined - 0.02,
            "boxes did not separate: {separation} < {combined}"
        );
        assert!(a.state.velocity.length() < 0.01, "a gained energy");
        assert!(b.state.velocity.length() < 0.01, "b gained energy");
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn nose_to_tail_cars_separate_at_combined_half_lengths() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(0.0, 0.25, 3.3));
        let (a_length, _) = a.contact_half_extents();
        let (b_length, _) = b.contact_half_extents();
        let combined = a_length + b_length;

        // Two ~4.3 m cars must not resolve to a ~3.3 m box.
        assert!(
            (4.05..=4.45).contains(&combined),
            "combined length {combined} is not car sized"
        );

        for _ in 0..180 {
            resolve_car_contact(&mut a, &mut b);
        }

        let separation = b.state.position.z - a.state.position.z;
        assert!(
            (separation - combined).abs() < 0.02,
            "nose-to-tail separation {separation} != combined half lengths {combined}"
        );
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn side_by_side_contact_separates_at_combined_half_widths() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(1.0, 0.25, 0.0));
        let (_, a_width) = a.contact_half_extents();
        let (_, b_width) = b.contact_half_extents();
        let combined = a_width + b_width;

        for _ in 0..180 {
            resolve_car_contact(&mut a, &mut b);
        }

        let separation = b.state.position.x - a.state.position.x;
        assert!(
            (separation - combined).abs() < 0.02,
            "side-by-side separation {separation} != combined half widths {combined}"
        );
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn mixed_car_sizes_use_their_own_extents() {
        let short = vehicle_with_wheelbase_at(1.8, 1.3, Vec3::new(0.0, 0.25, 0.0));
        let long = vehicle_with_wheelbase_at(3.2, 1.9, Vec3::new(0.0, 0.25, 3.3));
        let (short_length, short_width) = short.contact_half_extents();
        let (long_length, long_width) = long.contact_half_extents();

        assert_eq!(short_length, 1.9, "short car should hit the minimum length");
        assert_eq!(long_length, 2.3, "long car should hit the maximum length");
        assert!(long_width > short_width, "wide car should be wider");
        let combined = short_length + long_length;

        let mut short = short;
        let mut long = long;
        for _ in 0..180 {
            resolve_car_contact(&mut short, &mut long);
        }

        let separation = (long.state.position - short.state.position).length();
        assert!(
            (separation - combined).abs() < 0.02,
            "mixed-size separation {separation} != combined half lengths {combined}"
        );
        assert_finite(&short);
        assert_finite(&long);
    }

    #[test]
    fn resolver_is_deterministic_and_symmetric() {
        let make_pair = || {
            let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
            let mut b = vehicle_at(Vec3::new(0.0, 0.25, 3.3));
            a.state.velocity = Vec3::new(0.0, 0.0, 5.0);
            b.state.velocity = Vec3::new(0.0, 0.0, -5.0);
            (a, b)
        };

        let (mut a1, mut b1) = make_pair();
        let first = resolve_car_contact(&mut a1, &mut b1).expect("first contact");
        let (mut a2, mut b2) = make_pair();
        let second = resolve_car_contact(&mut a2, &mut b2).expect("second contact");

        assert_eq!(first.point, second.point);
        assert_eq!(first.normal, second.normal);
        assert_eq!(first.relative_speed, second.relative_speed);
        assert_eq!(a1.state.velocity, a2.state.velocity);
        assert_eq!(b1.state.velocity, b2.state.velocity);

        let (mut a3, mut b3) = make_pair();
        let forward = resolve_car_contact(&mut a3, &mut b3).expect("forward contact");
        let (mut a4, mut b4) = make_pair();
        let reversed = resolve_car_contact(&mut b4, &mut a4).expect("reversed contact");

        assert!((forward.relative_speed - reversed.relative_speed).abs() < 1e-4);
        assert!((forward.normal + reversed.normal).length() < 1e-4);
        assert!((a3.state.velocity - a4.state.velocity).length() < 1e-4);
        assert!((b3.state.velocity - b4.state.velocity).length() < 1e-4);
    }

    #[test]
    fn separated_or_vertical_offset_produces_no_contact() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut far = vehicle_at(Vec3::new(0.0, 0.25, 10.0));
        assert!(resolve_car_contact(&mut a, &mut far).is_none());

        let mut low = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut high = vehicle_at(Vec3::new(0.0, 5.25, 2.0));
        assert!(resolve_car_contact(&mut low, &mut high).is_none());
    }

    #[test]
    fn angled_corner_contact_separates_without_deep_penetration() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(0.8, 0.25, 3.4));
        b.state.rotation = Quat::from_rotation_y(0.4);
        a.state.velocity = Vec3::new(0.0, 0.0, 3.0);

        let before = (b.state.position - a.state.position).length();
        let contact = resolve_car_contact(&mut a, &mut b).expect("angled corner contact");

        assert!(contact.normal.is_finite(), "normal={:?}", contact.normal);
        assert!(
            (contact.normal.length() - 1.0).abs() < 1e-3,
            "normal is not unit: {:?}",
            contact.normal
        );
        let to_b = Vec3::new(
            b.state.position.x - a.state.position.x,
            0.0,
            b.state.position.z - a.state.position.z,
        )
        .normalize_or_zero();
        assert!(
            contact.normal.dot(to_b) > 0.1,
            "normal {:?} does not point toward b",
            contact.normal
        );

        for _ in 0..120 {
            resolve_car_contact(&mut a, &mut b);
        }

        let after = (b.state.position - a.state.position).length();
        assert!(
            after > before,
            "angled contact did not separate: {before} -> {after}"
        );
        let remaining = max_probe_penetration(&a, &b);
        assert!(
            remaining < 0.03,
            "deep interpenetration remains after resolution: {remaining}"
        );
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn front_side_contact_bounds_yaw_with_linear_separation() {
        // A hits the right-front side of a stationary B while moving along -X.
        // The contact points are offset from B's centre of gravity, so a
        // free-body response would yaw B strongly. The tire-yaw scale must keep
        // that bounded without removing the linear push or the separation.
        let make_pair = || {
            let mut a = vehicle_at(Vec3::new(1.2, 0.25, 1.0));
            let b = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
            a.state.velocity = Vec3::new(-6.0, 0.0, 0.0);
            (a, b)
        };

        let (mut a, mut b) = make_pair();
        let before = (b.state.position - a.state.position).length();
        let contact = resolve_car_contact(&mut a, &mut b).expect("front-side contact");
        assert!(contact.relative_speed > 0.0, "expected a closing contact");
        assert!(
            (contact.normal.length() - 1.0).abs() < 1e-3,
            "front-side normal is not unit: {:?}",
            contact.normal
        );
        let first_yaw = b.state.angular_velocity.y.abs();
        assert!(
            b.state.velocity.length() > 0.5,
            "hit car lost its linear push: {:?}",
            b.state.velocity
        );
        assert!(
            first_yaw > 0.0,
            "expected a yaw response from the front-side hit"
        );

        for _ in 0..120 {
            resolve_car_contact(&mut a, &mut b);
        }
        let after = (b.state.position - a.state.position).length();
        let scaled_yaw = b.state.angular_velocity.y.abs();

        // The same setup with free-body angular response is the upper bound.
        let (mut free_a, mut free_b) = make_pair();
        for _ in 0..120 {
            let _ = resolve_pair_with_angular_scale(&mut free_a, &mut free_b, 0, 1, 1.0);
        }
        let free_yaw = free_b.state.angular_velocity.y.abs();

        assert!(
            after > before,
            "front-side contact did not separate: {before} -> {after}"
        );
        let remaining = max_probe_penetration(&a, &b);
        assert!(
            remaining < 0.03,
            "front-side contact left deep overlap: {remaining}"
        );
        assert!(
            scaled_yaw < free_yaw,
            "tire-yaw scale did not reduce yaw: {scaled_yaw} >= {free_yaw}"
        );
        assert!(
            scaled_yaw <= 0.35,
            "front-side yaw {scaled_yaw} rad/s looks like a free-body spin"
        );
        assert_finite(&a);
        assert_finite(&b);
    }

    #[test]
    fn offset_front_contact_still_separates() {
        let mut a = vehicle_at(Vec3::new(0.0, 0.25, 0.0));
        let mut b = vehicle_at(Vec3::new(0.9, 0.25, 3.6));
        a.state.velocity = Vec3::new(0.0, 0.0, 4.0);

        let before = (b.state.position - a.state.position).length();
        let contact = resolve_car_contact(&mut a, &mut b).expect("offset front contact");
        assert!(
            contact.normal.z > 0.3,
            "offset front normal should mostly face b: {:?}",
            contact.normal
        );

        for _ in 0..120 {
            resolve_car_contact(&mut a, &mut b);
        }

        let after = (b.state.position - a.state.position).length();
        assert!(
            after > before,
            "offset front contact did not separate: {before} -> {after}"
        );
        let remaining = max_probe_penetration(&a, &b);
        assert!(
            remaining < 0.03,
            "offset front contact left deep overlap: {remaining}"
        );
        assert_finite(&a);
        assert_finite(&b);
    }
}
