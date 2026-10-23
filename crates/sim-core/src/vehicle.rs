use glam::{Mat4, Quat, Vec3};
use sim_types::{CarPhysics, PacejkaCoefficients, SpawnPoint, TorquePoint};

use crate::collider::TrackCollider;

pub const FIXED_DT: f32 = 1.0 / 500.0;
const GRAVITY: f32 = 9.81;
const AIR_DENSITY: f32 = 1.225;
const WHEEL_MASS: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controls {
    pub steering: f32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub gear: i32,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            steering: 0.0,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            gear: 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WheelState {
    pub angular_velocity: f32,
    pub steer_angle: f32,
    pub spin_angle: f32,
    pub contact: bool,
    pub suspension_compression: f32,
    pub normal_load: f32,
    pub slip_ratio: f32,
    pub slip_angle: f32,
    pub surface_friction: f32,
    pub contact_point: Vec3,
    pub contact_normal: Vec3,
    pub wheel_center: Vec3,
}

impl Default for WheelState {
    fn default() -> Self {
        Self {
            angular_velocity: 0.0,
            steer_angle: 0.0,
            spin_angle: 0.0,
            contact: false,
            suspension_compression: 0.0,
            normal_load: 0.0,
            slip_ratio: 0.0,
            slip_angle: 0.0,
            surface_friction: 1.0,
            contact_point: Vec3::ZERO,
            contact_normal: Vec3::Y,
            wheel_center: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VehicleState {
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub wheels: [WheelState; 4],
    pub engine_rpm: f32,
    pub gear: i32,
    pub steering_rack_torque: f32,
    pub tick: u64,
}

pub struct Vehicle {
    pub state: VehicleState,
    physics: CarPhysics,
    collider: TrackCollider,
    spawn_position: Vec3,
    spawn_rotation: Quat,
    wheel_attach_local: [Vec3; 4],
    wheel_radius: [f32; 4],
    rest_length: [f32; 4],
    wheel_inertia: [f32; 4],
    previous_compression: [f32; 4],
    steer_angle: f32,
}

impl Vehicle {
    pub fn new(physics: CarPhysics, collider: TrackCollider, spawn: &SpawnPoint) -> Self {
        let rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let rotation = if rotation.length_squared() > 1e-8 {
            rotation.normalize()
        } else {
            Quat::IDENTITY
        };
        let spawn_position = Vec3::from_array(spawn.position) + Vec3::Y * 0.25;

        let mut wheel_attach_local = [Vec3::ZERO; 4];
        let mut wheel_radius = [0.33; 4];
        let mut rest_length = [0.19; 4];
        let mut wheel_inertia = [1.0; 4];
        for (index, wheel) in physics.wheels.iter().enumerate() {
            let local = Vec3::from_array(wheel.position);
            let radius = wheel.radius.max(0.05);
            wheel_attach_local[index] = local + Vec3::Y * wheel.suspension.travel;
            wheel_radius[index] = radius;
            rest_length[index] = (wheel_attach_local[index].y - radius).max(0.01);
            wheel_inertia[index] = 0.5 * WHEEL_MASS * radius * radius;
        }

        let state = VehicleState {
            position: spawn_position,
            rotation,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            wheels: [WheelState::default(); 4],
            engine_rpm: physics.engine.idle_rpm,
            gear: 1,
            steering_rack_torque: 0.0,
            tick: 0,
        };

        Self {
            state,
            physics,
            collider,
            spawn_position,
            spawn_rotation: rotation,
            wheel_attach_local,
            wheel_radius,
            rest_length,
            wheel_inertia,
            previous_compression: [0.0; 4],
            steer_angle: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.state = VehicleState {
            position: self.spawn_position,
            rotation: self.spawn_rotation,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            wheels: [WheelState::default(); 4],
            engine_rpm: self.physics.engine.idle_rpm,
            gear: 1,
            steering_rack_torque: 0.0,
            tick: 0,
        };
        self.previous_compression = [0.0; 4];
        self.steer_angle = 0.0;
    }

    pub fn step(&mut self, controls: Controls) {
        let dt = FIXED_DT;
        let wheel_count = self.physics.wheels.len().min(4);
        for wheel in self.state.wheels.iter_mut().skip(wheel_count) {
            *wheel = WheelState::default();
        }

        self.steer_angle = controls.steering.clamp(-1.0, 1.0) * self.physics.steering.max_angle;
        let gear = controls
            .gear
            .clamp(-1, self.physics.drivetrain.gear_ratios.len() as i32);
        self.state.gear = gear;
        let gear_ratio = self.gear_ratio(gear);

        let driven_count = self
            .physics
            .wheels
            .iter()
            .filter(|wheel| wheel.driven)
            .count()
            .max(1);
        let average_driven_omega = self
            .physics
            .wheels
            .iter()
            .enumerate()
            .filter(|(_, wheel)| wheel.driven)
            .map(|(index, _)| self.state.wheels[index].angular_velocity)
            .sum::<f32>()
            / driven_count as f32;
        let rpm = self.engine_rpm(average_driven_omega, gear_ratio);
        self.state.engine_rpm = rpm;
        let engine_torque = self.engine_torque(rpm) * controls.throttle.clamp(0.0, 1.0);
        let drive_torque = engine_torque * gear_ratio * self.physics.drivetrain.final_drive * 0.85;

        let car_transform =
            Mat4::from_rotation_translation(self.state.rotation, self.state.position);
        let car_forward = horizontal_forward(self.state.rotation);
        let mut total_force = Vec3::new(0.0, -self.physics.mass * GRAVITY, 0.0);
        let mut total_torque = Vec3::ZERO;
        let mut rack_torque = 0.0;

        for index in 0..wheel_count {
            let wheel = &self.physics.wheels[index];
            let radius = self.wheel_radius[index];
            let steer_angle = if wheel.steerable {
                self.steer_angle
            } else {
                0.0
            };
            let wheel_forward = Quat::from_rotation_y(steer_angle) * car_forward;
            let wheel_right = wheel_forward.cross(Vec3::Y).normalize_or_zero();
            let attach_world = car_transform.transform_point3(self.wheel_attach_local[index]);
            let hit = self.collider.raycast(attach_world, Vec3::NEG_Y, 5.0);

            let mut wheel_state = WheelState {
                steer_angle,
                ..WheelState::default()
            };
            let mut drive = 0.0;
            if wheel.driven {
                drive = drive_torque / driven_count as f32;
            }
            let brake = self.physics.brakes.max_torque[index] * controls.brake.clamp(0.0, 1.0);

            if let Some(hit) = hit {
                let wheel_center = hit.point + hit.normal * radius;
                let suspension_length = (attach_world - wheel_center).length();
                let compression = (self.rest_length[index] - suspension_length)
                    .clamp(0.0, wheel.suspension.travel);
                let compression_velocity =
                    ((compression - self.previous_compression[index]) / dt).clamp(-8.0, 8.0);
                let damper = if compression_velocity >= 0.0 {
                    wheel.suspension.damper_bump
                } else {
                    wheel.suspension.damper_rebound
                };
                let max_load = self.physics.mass * GRAVITY * 3.0;
                let normal_load = (wheel.suspension.spring_rate * compression
                    + damper * compression_velocity)
                    .clamp(0.0, max_load);
                let suspension_force = hit.normal * normal_load;
                total_force += suspension_force;
                total_torque += (hit.point - self.state.position).cross(suspension_force);

                let contact_velocity = self.state.velocity
                    + self
                        .state
                        .angular_velocity
                        .cross(hit.point - self.state.position);
                let long_velocity = contact_velocity.dot(wheel_forward);
                let lateral_velocity = contact_velocity.dot(wheel_right);
                let denominator = long_velocity.abs().max(1.0);
                let slip_ratio = ((self.state.wheels[index].angular_velocity * radius
                    - long_velocity)
                    / denominator)
                    .clamp(-2.0, 2.0);
                let slip_angle = (-lateral_velocity).atan2(denominator).clamp(-1.5, 1.5);
                let mu = hit.friction * wheel.tire.friction_scale;
                if drive != 0.0 {
                    let max_drive_torque = mu * normal_load * radius;
                    drive = drive.clamp(-max_drive_torque, max_drive_torque);
                }
                let fx = normal_load * magic_formula(slip_ratio, &wheel.tire.longitudinal);
                let fy = normal_load * magic_formula(slip_angle, &wheel.tire.lateral);
                let max_tire_force = (mu * normal_load).max(1.0);
                let magnitude = (fx * fx + fy * fy).sqrt();
                let scale = if magnitude > max_tire_force {
                    max_tire_force / magnitude
                } else {
                    1.0
                };
                let tire_force = wheel_forward * (fx * scale) + wheel_right * (fy * scale);
                total_force += tire_force;
                total_torque += (hit.point - self.state.position).cross(tire_force);
                rack_torque += fy * 0.01;

                let tire_torque = fx * scale * radius;
                let omega_before = self.state.wheels[index].angular_velocity;
                if brake > 0.0 && omega_before.abs() < 0.5 && controls.throttle < 0.05 {
                    self.state.wheels[index].angular_velocity = 0.0;
                } else {
                    let brake_torque = brake * omega_before.signum();
                    let wheel_torque = drive - tire_torque - brake_torque;
                    let next_omega = (omega_before + wheel_torque / self.wheel_inertia[index] * dt)
                        .clamp(-200.0, 200.0);
                    self.state.wheels[index].angular_velocity =
                        if brake > 0.0 && next_omega.signum() != omega_before.signum() {
                            0.0
                        } else {
                            next_omega
                        };
                }

                wheel_state.angular_velocity = self.state.wheels[index].angular_velocity;
                wheel_state.contact = true;
                wheel_state.suspension_compression = compression;
                wheel_state.normal_load = normal_load;
                wheel_state.slip_ratio = slip_ratio;
                wheel_state.slip_angle = slip_angle;
                wheel_state.surface_friction = hit.friction;
                wheel_state.contact_point = hit.point;
                wheel_state.contact_normal = hit.normal;
                wheel_state.wheel_center = wheel_center;
                self.previous_compression[index] = compression;
            } else {
                self.previous_compression[index] = 0.0;
                let brake_torque = brake * self.state.wheels[index].angular_velocity.signum();
                let wheel_torque = drive - brake_torque;
                self.state.wheels[index].angular_velocity = (self.state.wheels[index]
                    .angular_velocity
                    + wheel_torque / self.wheel_inertia[index] * dt)
                    .clamp(-200.0, 200.0);
                wheel_state.angular_velocity = self.state.wheels[index].angular_velocity;
                wheel_state.wheel_center = attach_world - Vec3::Y * self.rest_length[index];
            }

            wheel_state.spin_angle =
                self.state.wheels[index].spin_angle + wheel_state.angular_velocity * dt;
            self.state.wheels[index] = wheel_state;
        }

        let speed = self.state.velocity.length();
        let horizontal_speed =
            Vec3::new(self.state.velocity.x, 0.0, self.state.velocity.z).length();
        if controls.throttle < 0.01 && controls.brake < 0.01 && horizontal_speed < 0.25 {
            let damping = (1.0 - 8.0 * dt).max(0.0);
            self.state.velocity.x *= damping;
            self.state.velocity.z *= damping;
            if Vec3::new(self.state.velocity.x, 0.0, self.state.velocity.z).length() < 0.02 {
                self.state.velocity.x = 0.0;
                self.state.velocity.z = 0.0;
            }
            for wheel in &mut self.state.wheels {
                wheel.angular_velocity *= damping;
                if wheel.angular_velocity.abs() < 0.05 {
                    wheel.angular_velocity = 0.0;
                }
            }
        }
        if speed > 0.1 {
            let drag = 0.5
                * AIR_DENSITY
                * self.physics.aero.drag_coefficient
                * self.physics.aero.frontal_area
                * speed
                * speed;
            total_force -= self.state.velocity.normalize() * drag;
        }

        let acceleration = total_force / self.physics.mass;
        self.state.velocity += acceleration * dt;
        self.state.velocity = clamp_length(self.state.velocity, 60.0);

        let body_torque = self.state.rotation.inverse() * total_torque;
        let inertia = Vec3::from_array(self.physics.inertia).max(Vec3::splat(1.0));
        let angular_acceleration = Vec3::new(
            body_torque.x / inertia.x,
            body_torque.y / inertia.y,
            body_torque.z / inertia.z,
        );
        self.state.angular_velocity += (self.state.rotation * angular_acceleration) * dt;
        self.state.angular_velocity *= 0.999;
        self.state.angular_velocity = clamp_length(self.state.angular_velocity, 8.0);

        self.state.position += self.state.velocity * dt;
        let delta_rotation = Quat::from_scaled_axis(self.state.angular_velocity * dt);
        self.state.rotation = (delta_rotation * self.state.rotation).normalize();

        if controls.throttle < 0.01 && controls.brake < 0.01 {
            let horizontal_speed =
                Vec3::new(self.state.velocity.x, 0.0, self.state.velocity.z).length();
            if horizontal_speed < 0.06 {
                self.state.velocity.x = 0.0;
                self.state.velocity.z = 0.0;
                for wheel in &mut self.state.wheels {
                    wheel.angular_velocity = 0.0;
                }
            }
        }

        self.resolve_wall_collisions();
        self.state.steering_rack_torque = rack_torque;

        if !self.state.position.is_finite()
            || !self.state.velocity.is_finite()
            || !self.state.rotation.is_finite()
        {
            self.reset();
            return;
        }

        self.state.tick += 1;
    }

    fn resolve_wall_collisions(&mut self) {
        let rotation = self.state.rotation;
        let offsets = [Vec3::new(0.0, 0.4, -1.3), Vec3::new(0.0, 0.4, 1.3)];
        for offset in offsets {
            let center = self.state.position + rotation * offset;
            for contact in self.collider.wall_contacts(center, 0.8) {
                self.state.position += contact.normal * contact.penetration;
                let normal_velocity = self.state.velocity.dot(contact.normal);
                if normal_velocity < 0.0 {
                    self.state.velocity -= contact.normal * normal_velocity * 1.1;
                }
                self.state.angular_velocity *= 0.97;
            }
        }
    }

    fn gear_ratio(&self, gear: i32) -> f32 {
        if gear > 0 {
            self.physics
                .drivetrain
                .gear_ratios
                .get(gear as usize - 1)
                .copied()
                .unwrap_or(0.0)
        } else if gear < 0 {
            -self
                .physics
                .drivetrain
                .gear_ratios
                .first()
                .copied()
                .unwrap_or(3.0)
        } else {
            0.0
        }
    }

    fn engine_rpm(&self, average_driven_omega: f32, gear_ratio: f32) -> f32 {
        let ratio = gear_ratio.abs() * self.physics.drivetrain.final_drive;
        let rpm = average_driven_omega.abs() * ratio * 60.0 / core::f32::consts::TAU;
        rpm.max(self.physics.engine.idle_rpm)
            .min(self.physics.engine.redline_rpm)
    }

    fn engine_torque(&self, rpm: f32) -> f32 {
        interpolate_torque(&self.physics.engine.torque_curve, rpm)
    }
}

fn horizontal_forward(rotation: Quat) -> Vec3 {
    let forward = rotation * Vec3::NEG_Z;
    let horizontal = Vec3::new(forward.x, 0.0, forward.z);
    horizontal.normalize_or_zero()
}

fn magic_formula(slip: f32, coefficients: &PacejkaCoefficients) -> f32 {
    let b = coefficients.b;
    let c = coefficients.c;
    let d = coefficients.d;
    let e = coefficients.e;
    let bs = b * slip;
    let inner = c * (bs - e * (bs - bs.atan())).atan();
    d * inner.sin()
}

fn interpolate_torque(curve: &[TorquePoint], rpm: f32) -> f32 {
    if curve.is_empty() {
        return 0.0;
    }
    if rpm <= curve[0].rpm {
        return curve[0].torque_nm;
    }
    for pair in curve.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if rpm <= b.rpm {
            let t = (rpm - a.rpm) / (b.rpm - a.rpm).max(1.0);
            return a.torque_nm + (b.torque_nm - a.torque_nm) * t;
        }
    }
    curve[curve.len() - 1].torque_nm
}

fn clamp_length(vector: Vec3, max_length: f32) -> Vec3 {
    let length = vector.length();
    if length > max_length {
        vector * (max_length / length)
    } else {
        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collider::TrackCollider;
    use sim_types::{
        Aabb, AeroSpec, BrakeSpec, DriveLayout, DrivetrainSpec, EngineSpec, SteeringSpec,
        SurfaceSpec, SuspensionSpec, TireSpec, TorquePoint, TrackCollision, TrackMetadata,
        WheelPhysics,
    };

    fn physics() -> CarPhysics {
        let suspension = SuspensionSpec {
            travel: 0.18,
            spring_rate: 200000.0,
            damper_bump: 12000.0,
            damper_rebound: 15000.0,
            anti_roll: 8000.0,
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
            relaxation_length: 0.1,
            peak_slip_ratio: 0.12,
            peak_slip_angle: 0.12,
            friction_scale: 1.0,
        };
        let wheels = [
            ("FL", [-0.9, 0.34, -1.3], true, true, false),
            ("FR", [0.9, 0.34, -1.3], true, false, false),
            ("RL", [-0.9, 0.34, 1.3], false, true, true),
            ("RR", [0.9, 0.34, 1.3], false, false, true),
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
            suspension,
            tire,
        });
        CarPhysics {
            mass: 1200.0,
            inertia: [450.0, 600.0, 350.0],
            center_of_gravity: [0.0, 0.35, 0.0],
            wheelbase: 2.6,
            track_width: [1.8, 1.8],
            wheels,
            engine: EngineSpec {
                idle_rpm: 900.0,
                redline_rpm: 7000.0,
                torque_curve: vec![
                    TorquePoint {
                        rpm: 1000.0,
                        torque_nm: 180.0,
                    },
                    TorquePoint {
                        rpm: 3500.0,
                        torque_nm: 320.0,
                    },
                    TorquePoint {
                        rpm: 6500.0,
                        torque_nm: 260.0,
                    },
                ],
            },
            drivetrain: DrivetrainSpec {
                gear_ratios: vec![3.2, 2.1, 1.5, 1.2, 1.0, 0.8],
                final_drive: 3.7,
                clutch_max_torque: 400.0,
                drive_layout: DriveLayout::Rear,
            },
            brakes: BrakeSpec {
                max_torque: [1800.0, 1800.0, 1200.0, 1200.0],
                bias: 0.6,
            },
            steering: SteeringSpec {
                max_angle: 0.6,
                ratio: 14.0,
                caster: 0.09,
                kingpin_inclination: 0.12,
            },
            aero: AeroSpec {
                drag_coefficient: 0.32,
                frontal_area: 2.0,
                downforce_coefficient: 0.0,
            },
        }
    }

    fn collider() -> TrackCollider {
        let collision = TrackCollision {
            vertices: vec![
                [-50.0, 0.0, -50.0],
                [-50.0, 0.0, 50.0],
                [50.0, 0.0, 50.0],
                [50.0, 0.0, -50.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: Aabb {
                min: [-50.0, 0.0, -50.0],
                max: [50.0, 0.0, 50.0],
            },
        };
        let metadata = TrackMetadata {
            name: String::from("test"),
            length: 100.0,
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

    fn spawn() -> SpawnPoint {
        SpawnPoint {
            name: String::from("start"),
            position: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn settles_on_ground_without_nan() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..2000 {
            vehicle.step(Controls::default());
        }
        assert!(vehicle.state.position.is_finite());
        assert!(vehicle.state.position.y > -0.2 && vehicle.state.position.y < 0.5);
        assert!(vehicle.state.wheels.iter().all(|wheel| wheel.contact));
        assert!(vehicle.state.velocity.length() < 0.2);
    }

    #[test]
    fn accelerates_and_moves_forward() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        let controls = Controls {
            throttle: 1.0,
            gear: 1,
            ..Controls::default()
        };
        for _ in 0..1000 {
            vehicle.step(controls);
        }
        assert!(vehicle.state.position.is_finite());
        assert!(vehicle.state.velocity.length() > 2.0);
        assert!(vehicle.state.position.z < -0.5);
        assert!(vehicle.state.engine_rpm > 900.0);
    }

    #[test]
    fn steering_changes_yaw() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        let initial_yaw = vehicle.state.rotation.to_euler(glam::EulerRot::XYZ).1;
        let controls = Controls {
            throttle: 0.5,
            steering: 0.5,
            gear: 1,
            ..Controls::default()
        };
        for _ in 0..500 {
            vehicle.step(controls);
        }
        let yaw = vehicle.state.rotation.to_euler(glam::EulerRot::XYZ).1;
        assert!((yaw - initial_yaw).abs() > 0.01);
    }

    #[test]
    fn reverse_then_brake_stops() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..750 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: -1,
                ..Controls::default()
            });
        }
        assert!(vehicle.state.velocity.length() > 1.0);
        for _ in 0..1000 {
            vehicle.step(Controls {
                throttle: 0.0,
                brake: 1.0,
                gear: -1,
                ..Controls::default()
            });
        }
        assert!(
            vehicle.state.velocity.length() < 0.5,
            "speed {}",
            vehicle.state.velocity.length()
        );
    }

    #[test]
    fn reset_restores_spawn() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..100 {
            vehicle.step(Controls {
                throttle: 1.0,
                ..Controls::default()
            });
        }
        vehicle.reset();
        assert_eq!(vehicle.state.tick, 0);
        assert!(vehicle.state.velocity.length() < f32::EPSILON);
        assert!((vehicle.state.position.x).abs() < 1e-5);
    }
}
