use glam::{Mat4, Quat, Vec3};
use sim_types::{CarPhysics, CarTuning, PacejkaCoefficients, SpawnPoint, TorquePoint};

use crate::collider::TrackCollider;

pub const FIXED_DT: f32 = 1.0 / 500.0;
const GRAVITY: f32 = 9.81;
const AIR_DENSITY: f32 = 1.225;
const WHEEL_MASS: f32 = 20.0;
const BUMP_STOP_RATE_MULTIPLIER: f32 = 12.0;
const STEERING_SPEED_FALLOFF: f32 = 0.09;
const BODY_PROBE_HEIGHT: f32 = 5.0;

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
    pub surface_id: u16,
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
            surface_id: 0,
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
    default_tuning: CarTuning,
    collider: TrackCollider,
    spawn_position: Vec3,
    spawn_rotation: Quat,
    wheel_attach_local: [Vec3; 4],
    wheel_radius: [f32; 4],
    rest_length: [f32; 4],
    wheel_inertia: [f32; 4],
    previous_compression: [f32; 4],
    filtered_slip_ratio: [f32; 4],
    filtered_slip_angle: [f32; 4],
    traction_scale: f32,
    traction_control: bool,
    traction_strength: f32,
    abs_control: bool,
    abs_scale: [f32; 4],
    stability_control: bool,
    stability_strength: f32,
    steering_sensitivity: f32,
    esc_activity: f32,
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

        let default_tuning = CarTuning::from_physics(&physics);
        let (wheel_attach_local, wheel_radius, rest_length, wheel_inertia) =
            wheel_dimensions(&physics);

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
            default_tuning,
            collider,
            spawn_position,
            spawn_rotation: rotation,
            wheel_attach_local,
            wheel_radius,
            rest_length,
            wheel_inertia,
            previous_compression: [0.0; 4],
            filtered_slip_ratio: [0.0; 4],
            filtered_slip_angle: [0.0; 4],
            traction_scale: 1.0,
            traction_control: true,
            traction_strength: 1.0,
            abs_control: true,
            abs_scale: [1.0; 4],
            stability_control: true,
            stability_strength: 1.0,
            steering_sensitivity: 1.0,
            esc_activity: 0.0,
            steer_angle: 0.0,
        }
    }

    pub fn set_traction_control(&mut self, enabled: bool) {
        self.traction_control = enabled;
        if !enabled {
            self.traction_scale = 1.0;
        }
    }

    pub fn traction_control(&self) -> bool {
        self.traction_control
    }

    pub fn set_traction_strength(&mut self, strength: f32) {
        self.traction_strength = strength.clamp(0.0, 1.0);
    }

    pub fn traction_strength(&self) -> f32 {
        self.traction_strength
    }

    pub fn set_abs_control(&mut self, enabled: bool) {
        self.abs_control = enabled;
        if !enabled {
            self.abs_scale = [1.0; 4];
        }
    }

    pub fn abs_control(&self) -> bool {
        self.abs_control
    }

    pub fn abs_active(&self) -> bool {
        self.abs_scale.iter().any(|scale| *scale < 0.99)
    }

    pub fn set_stability_control(&mut self, enabled: bool) {
        self.stability_control = enabled;
        if !enabled {
            self.esc_activity = 0.0;
        }
    }

    pub fn stability_control(&self) -> bool {
        self.stability_control
    }

    pub fn set_stability_strength(&mut self, strength: f32) {
        self.stability_strength = strength.clamp(0.0, 1.0);
    }

    pub fn stability_strength(&self) -> f32 {
        self.stability_strength
    }

    pub fn set_steering_sensitivity(&mut self, sensitivity: f32) {
        self.steering_sensitivity = sensitivity.clamp(0.25, 2.0);
    }

    pub fn steering_sensitivity(&self) -> f32 {
        self.steering_sensitivity
    }

    pub fn esc_activity(&self) -> f32 {
        self.esc_activity
    }

    pub fn setup(&self) -> CarTuning {
        CarTuning::from_physics(&self.physics)
    }

    pub fn default_setup(&self) -> CarTuning {
        self.default_tuning.clone()
    }

    pub fn apply_setup(&mut self, tuning: &CarTuning) {
        let mut tuning = tuning.clone();
        tuning.sanitize();
        tuning.apply_to(&mut self.physics);
        self.refresh_wheel_dimensions();
        self.filtered_slip_ratio = [0.0; 4];
        self.filtered_slip_angle = [0.0; 4];
        self.traction_scale = 1.0;
        self.abs_scale = [1.0; 4];
        self.esc_activity = 0.0;
    }

    pub fn reset_setup(&mut self) {
        self.apply_setup(&self.default_tuning.clone());
    }

    fn refresh_wheel_dimensions(&mut self) {
        let (attach, radius, rest, inertia) = wheel_dimensions(&self.physics);
        self.wheel_attach_local = attach;
        self.wheel_radius = radius;
        self.rest_length = rest;
        self.wheel_inertia = inertia;
        self.previous_compression = [0.0; 4];
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
        self.filtered_slip_ratio = [0.0; 4];
        self.filtered_slip_angle = [0.0; 4];
        self.traction_scale = 1.0;
        self.abs_scale = [1.0; 4];
        self.esc_activity = 0.0;
        self.steer_angle = 0.0;
    }

    pub fn step(&mut self, controls: Controls) {
        let dt = FIXED_DT;
        let wheel_count = self.physics.wheels.len().min(4);
        for wheel in self.state.wheels.iter_mut().skip(wheel_count) {
            *wheel = WheelState::default();
        }

        let steer_speed = Vec3::new(self.state.velocity.x, 0.0, self.state.velocity.z).length();
        let steer_limit = (self.physics.steering.max_angle
            / (1.0 + steer_speed * STEERING_SPEED_FALLOFF))
            .max(0.08);
        let steer_input =
            (controls.steering.clamp(-1.0, 1.0) * self.steering_sensitivity).clamp(-1.0, 1.0);
        self.steer_angle = steer_input * steer_limit;
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
        let body_up = self.state.rotation * Vec3::Y;
        let cg_world = self.state.position
            + self.state.rotation * Vec3::from_array(self.physics.center_of_gravity);
        let previous_compression = self.previous_compression;
        let previous_contact = self.state.wheels.map(|wheel| wheel.contact);
        if self.traction_control {
            let driven_slip = self
                .physics
                .wheels
                .iter()
                .enumerate()
                .filter(|(_, wheel)| wheel.driven)
                .map(|(index, _)| self.state.wheels[index].slip_ratio.abs())
                .fold(0.0, f32::max);
            let target_slip = self
                .physics
                .wheels
                .iter()
                .find(|wheel| wheel.driven)
                .map(|wheel| (wheel.tire.peak_slip_ratio * 1.5).max(0.05))
                .unwrap_or(1.0);
            let excess = driven_slip - target_slip;
            let desired_traction = if excess > 0.0 {
                (1.0 - excess * 2.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.traction_scale += (desired_traction - self.traction_scale) * 0.2;
        } else {
            self.traction_scale = 1.0;
        }
        let mut esc_activity = 0.0;
        let mut esc_brake_wheel = None;
        if self.stability_control {
            let forward_speed = self.state.velocity.dot(car_forward);
            let car_right = car_forward.cross(Vec3::Y).normalize_or_zero();
            let lateral_speed = self.state.velocity.dot(car_right);
            let body_slip = lateral_speed.atan2(forward_speed.abs().max(1.0));
            let target_yaw =
                forward_speed * self.steer_angle.tan() / self.physics.wheelbase.max(0.1);
            let actual_yaw = self.state.angular_velocity.y;
            let yaw_excess = actual_yaw.abs() - target_yaw.abs();
            let slip_excess = body_slip.abs() - 0.12;
            if forward_speed > 3.0 && (slip_excess > 0.0 || yaw_excess > 0.08) {
                esc_activity = (slip_excess / 0.4).max(yaw_excess / 0.5).clamp(0.0, 1.0);
                esc_brake_wheel = Some(if body_slip > 0.0 { 1 } else { 0 });
            }
        }
        let esc_activity = esc_activity * self.stability_strength;
        self.esc_activity = esc_activity;
        let drive_torque = drive_torque * (1.0 - 0.5 * esc_activity);
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
            let steer_forward = Quat::from_rotation_y(steer_angle) * car_forward;
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
            let driver_brake =
                self.physics.brakes.max_torque[index] * controls.brake.clamp(0.0, 1.0);
            let esc_brake = if esc_brake_wheel == Some(index) {
                self.physics.brakes.max_torque[index] * 0.8 * esc_activity
            } else {
                0.0
            };
            let mut brake = (driver_brake + esc_brake).min(self.physics.brakes.max_torque[index]);

            if let Some(hit) = hit {
                let projected = (steer_forward - hit.normal * steer_forward.dot(hit.normal))
                    .normalize_or_zero();
                let wheel_forward = if projected.length_squared() > 1e-6 {
                    projected
                } else {
                    car_forward
                };
                let wheel_right = wheel_forward.cross(hit.normal).normalize_or_zero();
                let wheel_center = hit.point + hit.normal * radius;
                let suspension_length = (attach_world - wheel_center).dot(body_up).max(0.0);
                let raw_compression = self.rest_length[index] - suspension_length;
                let compression = raw_compression.clamp(0.0, wheel.suspension.travel);
                let bump_compression = (raw_compression - wheel.suspension.travel).max(0.0);
                let compression_velocity =
                    ((compression - previous_compression[index]) / dt).clamp(-8.0, 8.0);
                let damper = if compression_velocity >= 0.0 {
                    wheel.suspension.damper_bump
                } else {
                    wheel.suspension.damper_rebound
                };
                let other = if index % 2 == 0 { index + 1 } else { index - 1 };
                let anti_roll_force = if previous_contact[other] {
                    wheel.suspension.anti_roll * (compression - previous_compression[other])
                } else {
                    0.0
                };
                let bump_stop_force =
                    bump_compression * wheel.suspension.spring_rate * BUMP_STOP_RATE_MULTIPLIER;
                let max_load = self.physics.mass * GRAVITY * 3.0;
                let normal_load = (wheel.suspension.spring_rate * compression
                    + damper * compression_velocity
                    + anti_roll_force
                    + bump_stop_force)
                    .clamp(0.0, max_load);
                let suspension_force = hit.normal * normal_load;
                total_force += suspension_force;
                total_torque += (hit.point - cg_world).cross(suspension_force);

                let contact_velocity = self.state.velocity
                    + self
                        .state
                        .angular_velocity
                        .cross(hit.point - self.state.position);
                let long_velocity = contact_velocity.dot(wheel_forward);
                let lateral_velocity = contact_velocity.dot(wheel_right);
                let denominator = long_velocity.abs().max(1.0);
                let raw_slip_ratio = ((self.state.wheels[index].angular_velocity * radius
                    - long_velocity)
                    / denominator)
                    .clamp(-2.0, 2.0);
                let abs_target = (wheel.tire.peak_slip_ratio * 1.5).max(0.05);
                let desired_abs =
                    if self.abs_control && controls.brake > 0.01 && long_velocity.abs() > 1.0 {
                        let excess = raw_slip_ratio.abs() - abs_target;
                        if excess > 0.0 {
                            (1.0 - excess * 5.0).clamp(0.0, 1.0)
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };
                self.abs_scale[index] += (desired_abs - self.abs_scale[index]) * 0.25;
                brake *= self.abs_scale[index];
                let slip_angle = (-lateral_velocity).atan2(denominator).clamp(-1.5, 1.5);
                let relaxation_length = wheel.tire.relaxation_length.max(0.01);
                let relaxation_speed = long_velocity.abs().max(2.0);
                let relaxation_rate = (relaxation_speed / relaxation_length).min(1.0 / dt);
                let relaxation_blend = 1.0 - (-relaxation_rate * dt).exp();
                self.filtered_slip_ratio[index] +=
                    (raw_slip_ratio - self.filtered_slip_ratio[index]) * relaxation_blend;
                self.filtered_slip_angle[index] +=
                    (slip_angle - self.filtered_slip_angle[index]) * relaxation_blend;
                let slip_ratio = self.filtered_slip_ratio[index];
                let slip_angle = self.filtered_slip_angle[index];
                let mu = hit.friction * wheel.tire.friction_scale;
                if wheel.driven {
                    drive *= 1.0 - (1.0 - self.traction_scale) * self.traction_strength;
                }
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
                total_torque += (hit.point - cg_world).cross(tire_force);
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
                wheel_state.surface_id = hit.surface;
                wheel_state.contact_point = hit.point;
                wheel_state.contact_normal = hit.normal;
                wheel_state.wheel_center = wheel_center;
                self.previous_compression[index] = compression;
            } else {
                self.previous_compression[index] = 0.0;
                self.abs_scale[index] = 1.0;
                self.filtered_slip_ratio[index] *= 1.0 - 10.0 * dt;
                self.filtered_slip_angle[index] *= 1.0 - 10.0 * dt;
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
        if controls.throttle < 0.01 && horizontal_speed < 0.25 {
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
            let body_damping = (1.0 - 4.0 * dt).max(0.0);
            self.state.angular_velocity.x *= body_damping;
            self.state.angular_velocity.y *= (1.0 - 2.0 * dt).max(0.0);
            self.state.angular_velocity.z *= body_damping;
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

        if controls.throttle < 0.01 {
            let horizontal_speed =
                Vec3::new(self.state.velocity.x, 0.0, self.state.velocity.z).length();
            if horizontal_speed < 0.06 {
                self.state.velocity.x = 0.0;
                self.state.velocity.z = 0.0;
                for wheel in &mut self.state.wheels {
                    wheel.angular_velocity = 0.0;
                }
                let stop_damping = (1.0 - 8.0 * dt).max(0.0);
                self.state.angular_velocity.x *= stop_damping;
                self.state.angular_velocity.y = 0.0;
                self.state.angular_velocity.z *= stop_damping;
            }
        }

        self.resolve_wall_collisions();
        self.resolve_body_contacts();
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

    fn resolve_body_contacts(&mut self) {
        let offsets = [
            Vec3::new(-0.8, 0.15, -1.4),
            Vec3::new(0.8, 0.15, -1.4),
            Vec3::new(-0.8, 0.15, 1.4),
            Vec3::new(0.8, 0.15, 1.4),
        ];
        let mut lift = Vec3::ZERO;
        for offset in offsets {
            let point = self.state.position + self.state.rotation * offset;
            let origin = point + Vec3::Y * BODY_PROBE_HEIGHT;
            let Some(hit) = self
                .collider
                .raycast(origin, Vec3::NEG_Y, BODY_PROBE_HEIGHT + 5.0)
            else {
                continue;
            };
            let clearance = point.y - hit.point.y;
            let penetration = 0.03 - clearance;
            if penetration > 0.0 {
                let correction = hit.normal * penetration;
                if correction.length_squared() > lift.length_squared() {
                    lift = correction;
                }
            }
        }
        if lift.length_squared() > 0.0 {
            self.state.position += lift;
            let into_surface = self.state.velocity.dot(lift.normalize_or_zero());
            if into_surface < 0.0 {
                self.state.velocity -= lift.normalize_or_zero() * into_surface;
            }
        }
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

fn wheel_dimensions(physics: &CarPhysics) -> ([Vec3; 4], [f32; 4], [f32; 4], [f32; 4]) {
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
    (wheel_attach_local, wheel_radius, rest_length, wheel_inertia)
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
                        rpm: 2_000.0,
                        torque_nm: 260.0,
                    },
                    TorquePoint {
                        rpm: 3_000.0,
                        torque_nm: 300.0,
                    },
                    TorquePoint {
                        rpm: 4_500.0,
                        torque_nm: 314.0,
                    },
                    TorquePoint {
                        rpm: 6_000.0,
                        torque_nm: 300.0,
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
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 500.0],
                [500.0, 0.0, -500.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: Aabb {
                min: [-500.0, 0.0, -500.0],
                max: [500.0, 0.0, 500.0],
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

    fn slope_collider() -> TrackCollider {
        let slope = 0.3;
        let vertices = vec![
            [-50.0, -50.0 * slope, -50.0],
            [-50.0, -50.0 * slope, 50.0],
            [50.0, 50.0 * slope, 50.0],
            [50.0, 50.0 * slope, -50.0],
        ];
        let collision = TrackCollision {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: Aabb {
                min: [-50.0, -15.0, -50.0],
                max: [50.0, 15.0, 50.0],
            },
        };
        let metadata = TrackMetadata {
            name: String::from("slope"),
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
    fn settles_on_slope_without_nan() {
        let mut vehicle = Vehicle::new(physics(), slope_collider(), &spawn());
        for _ in 0..3000 {
            vehicle.step(Controls::default());
        }
        assert!(vehicle.state.position.is_finite());
        assert!(vehicle.state.velocity.is_finite());
        let up = vehicle.state.rotation * Vec3::Y;
        assert!(up.y > 0.8, "car tipped on slope: up={up:?}");
        assert!(vehicle.state.wheels.iter().all(|wheel| wheel.contact));
    }

    #[test]
    fn slope_maneuver_stays_upright() {
        let mut vehicle = Vehicle::new(physics(), slope_collider(), &spawn());
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 0.45,
                steering: 0.35,
                gear: 1,
                ..Controls::default()
            });
        }
        assert!(vehicle.state.position.is_finite());
        assert!(vehicle.state.velocity.is_finite());
        let up = vehicle.state.rotation * Vec3::Y;
        assert!(up.y > 0.7, "car tipped on slope: up={up:?}");
        assert!(
            vehicle
                .state
                .wheels
                .iter()
                .filter(|wheel| wheel.contact)
                .count()
                >= 3
        );
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
    fn stability_maneuver_remains_finite_and_upright() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        let phases = [
            (
                900,
                Controls {
                    throttle: 1.0,
                    gear: 1,
                    ..Controls::default()
                },
            ),
            (
                700,
                Controls {
                    throttle: 1.0,
                    gear: 2,
                    ..Controls::default()
                },
            ),
            (
                1100,
                Controls {
                    throttle: 0.55,
                    steering: 0.55,
                    gear: 2,
                    ..Controls::default()
                },
            ),
            (
                700,
                Controls {
                    brake: 0.85,
                    steering: -0.35,
                    gear: 2,
                    ..Controls::default()
                },
            ),
            (
                1100,
                Controls {
                    steering: -0.2,
                    gear: 2,
                    ..Controls::default()
                },
            ),
        ];
        for (steps, controls) in phases {
            for _ in 0..steps {
                vehicle.step(controls);
                assert!(vehicle.state.position.is_finite());
                assert!(vehicle.state.velocity.is_finite());
                assert!(vehicle.state.angular_velocity.is_finite());
            }
            let up = vehicle.state.rotation * Vec3::Y;
            assert!(
                up.y > 0.55,
                "car tipped over: up={up:?} speed={}",
                vehicle.state.velocity.length()
            );
        }
        assert!(
            vehicle
                .state
                .wheels
                .iter()
                .filter(|wheel| wheel.contact)
                .count()
                >= 3
        );
        assert!(vehicle.state.angular_velocity.length() < 3.0);
        assert!(vehicle.state.velocity.length() < 40.0);
    }

    #[test]
    fn braking_to_stop_while_steering_settles() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..900 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1500 {
            vehicle.step(Controls {
                brake: 1.0,
                steering: 0.6,
                gear: 1,
                ..Controls::default()
            });
        }
        assert!(vehicle.state.velocity.length() < 0.05);
        for _ in 0..500 {
            vehicle.step(Controls {
                brake: 1.0,
                steering: 0.6,
                gear: 1,
                ..Controls::default()
            });
            assert!(vehicle.state.angular_velocity.length() < 0.01);
        }
        let up = vehicle.state.rotation * Vec3::Y;
        assert!(up.y > 0.99);
        assert!(up.x.abs() < 0.02);
        assert!(up.z.abs() < 0.02);
    }

    #[test]
    fn steering_limit_reduces_with_speed() {
        let physics = physics();
        let mut slow = Vehicle::new(physics.clone(), collider(), &spawn());
        slow.step(Controls {
            steering: 1.0,
            ..Controls::default()
        });
        assert!(slow.steer_angle > physics.steering.max_angle * 0.9);

        let mut fast = Vehicle::new(physics.clone(), collider(), &spawn());
        for _ in 0..1200 {
            fast.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        fast.step(Controls {
            steering: 1.0,
            gear: 1,
            ..Controls::default()
        });
        assert!(
            fast.steer_angle < physics.steering.max_angle * 0.6,
            "high-speed steering limit was not reduced: {}",
            fast.steer_angle
        );
    }

    #[test]
    fn powered_cornering_keeps_rear_slip_below_front() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 2,
                ..Controls::default()
            });
        }
        for _ in 0..800 {
            vehicle.step(Controls {
                throttle: 1.0,
                steering: 0.5,
                gear: 2,
                ..Controls::default()
            });
        }
        let front = (vehicle.state.wheels[0].slip_angle.abs()
            + vehicle.state.wheels[1].slip_angle.abs())
            * 0.5;
        let rear = (vehicle.state.wheels[2].slip_angle.abs()
            + vehicle.state.wheels[3].slip_angle.abs())
            * 0.5;
        let ratio = (vehicle.state.wheels[2].slip_ratio.abs()
            + vehicle.state.wheels[3].slip_ratio.abs())
            * 0.5;
        assert!(
            rear < front,
            "rear slip exceeded front: front={front:.3} rear={rear:.3}"
        );
        assert!(
            ratio < 0.3,
            "rear wheels spun too much under power: {ratio:.3}"
        );
    }

    fn powered_corner(traction: bool, stability: bool) -> (f32, f32, f32, f32) {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        vehicle.set_traction_control(traction);
        vehicle.set_stability_control(stability);
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 2,
                ..Controls::default()
            });
        }
        for _ in 0..800 {
            vehicle.step(Controls {
                throttle: 1.0,
                steering: 0.5,
                gear: 2,
                ..Controls::default()
            });
        }
        let forward = vehicle.state.rotation * Vec3::NEG_Z;
        let right = vehicle.state.rotation * Vec3::X;
        let v_lat = vehicle.state.velocity.dot(right);
        let v_long = vehicle.state.velocity.dot(forward);
        let body_slip = v_lat.atan2(v_long.abs().max(1.0));
        (
            vehicle.state.angular_velocity.y,
            vehicle.state.velocity.length(),
            vehicle.esc_activity(),
            body_slip.abs(),
        )
    }

    #[test]
    fn stability_control_reduces_powered_oversteer() {
        let (off_yaw, off_speed, off_activity, off_slip) = powered_corner(false, false);
        let (on_yaw, on_speed, on_activity, on_slip) = powered_corner(false, true);
        assert!(off_activity == 0.0);
        assert!(on_activity > 0.0);
        assert!(
            on_yaw.abs() < off_yaw.abs() * 0.9,
            "ESC did not reduce yaw: off={off_yaw} on={on_yaw}"
        );
        assert!(
            on_slip < off_slip * 0.9,
            "ESC did not reduce body slip: off={off_slip} on={on_slip}"
        );
        assert!(off_speed.is_finite());
        assert!(on_yaw.is_finite() && on_speed.is_finite());
    }

    #[test]
    fn setup_apply_and_reset() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        let default = vehicle.setup();
        let mut tuned = default.clone();
        tuned.rear.tire.friction_scale = 1.25;
        tuned.front.suspension.spring_rate = 70_000.0;
        tuned.front.suspension.travel = 0.12;
        vehicle.apply_setup(&tuned);
        assert_eq!(vehicle.setup(), tuned);
        vehicle.reset_setup();
        assert_eq!(vehicle.setup(), default);
    }

    fn braking_lock(use_abs: bool) -> (usize, bool) {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        vehicle.set_abs_control(use_abs);
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..120 {
            vehicle.step(Controls {
                brake: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        let locked = vehicle
            .state
            .wheels
            .iter()
            .filter(|wheel| wheel.angular_velocity.abs() < 0.1)
            .count();
        (locked, vehicle.abs_active())
    }

    #[test]
    fn abs_prevents_wheel_lock() {
        let (locked_with_abs, active_with_abs) = braking_lock(true);
        let (locked_without_abs, active_without_abs) = braking_lock(false);
        assert_eq!(locked_without_abs, 4);
        assert!(locked_with_abs < 2);
        assert!(active_with_abs);
        assert!(!active_without_abs);
    }

    #[test]
    fn assists_can_be_toggled_and_adjusted() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        assert!(vehicle.traction_control());
        assert!(vehicle.stability_control());
        assert!(vehicle.abs_control());
        assert_eq!(vehicle.traction_strength(), 1.0);
        assert_eq!(vehicle.stability_strength(), 1.0);
        assert_eq!(vehicle.steering_sensitivity(), 1.0);
        vehicle.set_traction_control(false);
        vehicle.set_stability_control(false);
        vehicle.set_abs_control(false);
        vehicle.set_traction_strength(0.4);
        vehicle.set_stability_strength(0.6);
        vehicle.set_steering_sensitivity(1.5);
        assert!(!vehicle.traction_control());
        assert!(!vehicle.stability_control());
        assert!(!vehicle.abs_control());
        assert_eq!(vehicle.traction_strength(), 0.4);
        assert_eq!(vehicle.stability_strength(), 0.6);
        assert_eq!(vehicle.steering_sensitivity(), 1.5);
        assert_eq!(vehicle.esc_activity(), 0.0);
        vehicle.set_steering_sensitivity(9.0);
        assert_eq!(vehicle.steering_sensitivity(), 2.0);
    }

    #[test]
    fn zero_stability_strength_disables_esc() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        vehicle.set_stability_strength(0.0);
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 2,
                ..Controls::default()
            });
        }
        for _ in 0..800 {
            vehicle.step(Controls {
                throttle: 1.0,
                steering: 0.5,
                gear: 2,
                ..Controls::default()
            });
        }
        assert_eq!(vehicle.esc_activity(), 0.0);
    }

    #[test]
    fn traction_control_limits_rear_wheelspin() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 2,
                ..Controls::default()
            });
        }
        for _ in 0..800 {
            vehicle.step(Controls {
                throttle: 1.0,
                steering: 0.2,
                gear: 2,
                ..Controls::default()
            });
        }
        let rear_slip_ratio = (vehicle.state.wheels[2].slip_ratio.abs()
            + vehicle.state.wheels[3].slip_ratio.abs())
            * 0.5;
        assert!(
            rear_slip_ratio < 0.4,
            "rear wheels spun too much: {rear_slip_ratio:.3}"
        );
        assert!(vehicle.state.rotation.is_finite());
    }

    #[test]
    fn cornering_transfers_load_to_outside_wheels() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..900 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..350 {
            vehicle.step(Controls {
                throttle: 0.45,
                steering: 0.35,
                gear: 1,
                ..Controls::default()
            });
        }
        let left = vehicle.state.wheels[0].normal_load + vehicle.state.wheels[2].normal_load;
        let right = vehicle.state.wheels[1].normal_load + vehicle.state.wheels[3].normal_load;
        let up = vehicle.state.rotation * Vec3::Y;
        assert!(up.y > 0.9);
        assert!(
            right > left + 50.0,
            "expected outside load transfer: left={left:.1} right={right:.1}"
        );
    }

    fn cornering_roll(anti_roll: f32) -> f32 {
        let mut physics = physics();
        for wheel in &mut physics.wheels {
            wheel.suspension.anti_roll = anti_roll;
        }
        let mut vehicle = Vehicle::new(physics, collider(), &spawn());
        for _ in 0..900 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..350 {
            vehicle.step(Controls {
                throttle: 0.45,
                steering: 0.35,
                gear: 1,
                ..Controls::default()
            });
        }
        let up = vehicle.state.rotation * Vec3::Y;
        up.x.abs()
    }

    #[test]
    fn anti_roll_bar_reduces_body_roll() {
        let without = cornering_roll(0.0);
        let with = cornering_roll(18_000.0);
        assert!(
            with < without * 0.9,
            "anti-roll did not reduce roll: without={without:.4} with={with:.4}"
        );
    }

    #[test]
    fn high_speed_lane_change_stays_stable() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 1,
                ..Controls::default()
            });
        }
        for _ in 0..1200 {
            vehicle.step(Controls {
                throttle: 1.0,
                gear: 2,
                ..Controls::default()
            });
        }
        assert!(vehicle.state.velocity.length() > 20.0);
        let phases = [
            (
                500,
                Controls {
                    throttle: 0.3,
                    steering: 0.12,
                    gear: 2,
                    ..Controls::default()
                },
            ),
            (
                500,
                Controls {
                    throttle: 0.3,
                    steering: -0.12,
                    gear: 2,
                    ..Controls::default()
                },
            ),
            (
                500,
                Controls {
                    brake: 0.5,
                    gear: 2,
                    ..Controls::default()
                },
            ),
        ];
        for (steps, controls) in phases {
            for _ in 0..steps {
                vehicle.step(controls);
                assert!(vehicle.state.position.is_finite());
                assert!(vehicle.state.velocity.is_finite());
                assert!(vehicle.state.angular_velocity.is_finite());
            }
            let up = vehicle.state.rotation * Vec3::Y;
            assert!(
                up.y > 0.7,
                "car lost stability: up={up:?} speed={} yaw={}",
                vehicle.state.velocity.length(),
                vehicle.state.angular_velocity.y
            );
        }
        assert!(
            vehicle
                .state
                .wheels
                .iter()
                .filter(|wheel| wheel.contact)
                .count()
                >= 3
        );
        assert!(vehicle.state.angular_velocity.length() < 3.0);
    }

    #[test]
    fn hard_landing_stays_finite_and_settles() {
        let mut vehicle = Vehicle::new(physics(), collider(), &spawn());
        vehicle.state.position.y += 0.5;
        let max_load = vehicle.physics.mass * GRAVITY * 3.0;
        for _ in 0..2500 {
            vehicle.step(Controls::default());
            assert!(vehicle.state.position.is_finite());
            assert!(vehicle.state.velocity.is_finite());
            assert!(vehicle.state.angular_velocity.is_finite());
            for wheel in &vehicle.state.wheels {
                assert!(wheel.normal_load <= max_load + 1.0);
                assert!(wheel.suspension_compression <= 0.15 + f32::EPSILON);
            }
        }
        assert!(vehicle.state.position.y > -0.2 && vehicle.state.position.y < 0.2);
        assert!(vehicle.state.wheels.iter().all(|wheel| wheel.contact));
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
