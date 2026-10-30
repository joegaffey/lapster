use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use sim_core::VehicleState;

use crate::scene::CarRig;
use crate::tuning::{ParticleSurface, ParticleTuning};

pub const MAX_PARTICLES: usize = 1024;
const TAU: f32 = std::f32::consts::TAU;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ParticleInstance {
    pub center_scale: [f32; 4],
    pub rotation: [f32; 4],
    pub color_alpha: [f32; 4],
    pub shape: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
enum SurfaceKind {
    Asphalt,
    Concrete,
    Grass,
    Kerb,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    age: f32,
    life: f32,
    start_size: f32,
    end_size: f32,
    color: Vec3,
    alpha: f32,
    rise: f32,
    drag: f32,
    shape_kind: f32,
    shape_variant: f32,
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    instances: Vec<ParticleInstance>,
    accumulators: [f32; 4],
    rng: u32,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            instances: Vec::with_capacity(MAX_PARTICLES),
            accumulators: [0.0; 4],
            rng: 0x9e37_79b9,
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.instances.clear();
        self.accumulators = [0.0; 4];
    }

    pub fn count(&self) -> usize {
        self.particles.len()
    }

    pub fn instances(&self) -> &[ParticleInstance] {
        &self.instances
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f32,
        car_world: Mat4,
        state: &VehicleState,
        rig: &CarRig,
        surface_names: &[String],
        tuning: &ParticleTuning,
    ) {
        let dt = dt.clamp(0.0, tuning.max_dt);
        if dt <= 0.0 {
            return;
        }
        self.step(dt);

        let max_active = tuning.max_active.min(MAX_PARTICLES);
        let speed = state.velocity.length();
        if max_active > 0 && speed > tuning.min_speed {
            for (index, wheel) in state.wheels.iter().enumerate() {
                if !wheel.contact {
                    self.accumulators[index] = 0.0;
                    continue;
                }
                let Some(rig_wheel) = rig.wheels.get(index) else {
                    self.accumulators[index] = 0.0;
                    continue;
                };
                let surface_kind = surface_kind(
                    surface_names
                        .get(wheel.surface_id as usize)
                        .map(String::as_str)
                        .unwrap_or(""),
                );
                let surface = match surface_kind {
                    SurfaceKind::Asphalt => &tuning.surfaces.asphalt,
                    SurfaceKind::Concrete => &tuning.surfaces.concrete,
                    SurfaceKind::Grass => &tuning.surfaces.grass,
                    SurfaceKind::Kerb => &tuning.surfaces.kerb,
                };
                let front_gate = if rig_wheel.is_front {
                    ((speed - tuning.front_gate_speed) / tuning.front_gate_range).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                if front_gate <= 0.0 {
                    self.accumulators[index] = 0.0;
                    continue;
                }
                let lateral = (wheel.slip_angle.abs() - tuning.lateral_threshold).max(0.0)
                    * tuning.lateral_scale;
                let longitudinal =
                    (wheel.slip_ratio.abs() - tuning.longitudinal_threshold).max(0.0);
                let skid = lateral.max(longitudinal);
                let load = (wheel.normal_load / tuning.load_reference).clamp(tuning.load_min, 1.0);
                let speed_factor =
                    ((speed - tuning.min_speed) / tuning.speed_factor_range).clamp(0.0, 1.0);
                let rolling = (speed / surface.rolling_speed.max(0.01)).clamp(0.0, 1.0)
                    * surface.rolling_amount;
                let rate = (skid * surface.rate_scale + rolling * surface.rolling_rate)
                    .min(surface.rate_cap)
                    * load
                    * speed_factor
                    * front_gate;
                if rate <= 0.0 {
                    self.accumulators[index] = 0.0;
                    continue;
                }
                self.accumulators[index] =
                    (self.accumulators[index] + rate * dt).min(tuning.accumulator_max);
                let mut spawned = 0;
                while self.accumulators[index] >= 1.0
                    && self.particles.len() < max_active
                    && spawned < tuning.max_spawn_per_step
                {
                    self.accumulators[index] -= 1.0;
                    spawned += 1;
                    self.spawn(
                        car_world,
                        state,
                        rig_wheel,
                        wheel,
                        surface,
                        surface_kind,
                        tuning,
                    );
                }
                if self.particles.len() >= max_active {
                    self.accumulators[index] = 0.0;
                }
            }
        } else {
            self.accumulators = [0.0; 4];
        }

        self.rebuild_instances(tuning);
    }

    fn step(&mut self, dt: f32) {
        let mut index = 0;
        while index < self.particles.len() {
            let particle = &mut self.particles[index];
            particle.age += dt;
            if particle.age >= particle.life {
                self.particles.swap_remove(index);
                continue;
            }
            particle.velocity.y += particle.rise * dt;
            particle.velocity *= (-particle.drag * dt).exp();
            particle.position += particle.velocity * dt;
            let spin = Quat::from_scaled_axis(particle.angular_velocity * dt);
            particle.rotation = (spin * particle.rotation).normalize();
            index += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn(
        &mut self,
        car_world: Mat4,
        state: &VehicleState,
        rig_wheel: &crate::scene::RigWheel,
        wheel: &sim_core::WheelState,
        surface: &ParticleSurface,
        surface_kind: SurfaceKind,
        tuning: &ParticleTuning,
    ) {
        let center = car_world.transform_point3(rig_wheel.local_model.w_axis.truncate());
        let normal = wheel.contact_normal.normalize_or(Vec3::Y);
        let contact = center - normal * (rig_wheel.radius + tuning.spawn.contact_offset);
        let forward = car_world
            .transform_vector3(Vec3::NEG_Z)
            .normalize_or(Vec3::NEG_Z);
        let right = car_world.transform_vector3(Vec3::X).normalize_or(Vec3::X);
        let speed = state.velocity.length();
        let outward = if rig_wheel.local_model.w_axis.x >= 0.0 {
            right
        } else {
            -right
        };
        let jitter = right
            * self.range(
                -rig_wheel.width * tuning.spawn.width_jitter_fraction,
                rig_wheel.width * tuning.spawn.width_jitter_fraction,
            )
            + forward * self.range(-tuning.spawn.forward_jitter, tuning.spawn.forward_jitter)
            + Vec3::Y * self.range(tuning.spawn.up_jitter_min, tuning.spawn.up_jitter_max);
        let velocity = state.velocity * tuning.spawn.velocity_inherit
            - forward
                * (tuning.spawn.forward_speed_base + speed * tuning.spawn.forward_speed_scale)
            + right * self.range(-tuning.spawn.lateral_spread, tuning.spawn.lateral_spread)
            + outward * self.range(tuning.spawn.outward_min, tuning.spawn.outward_max)
            + Vec3::Y * self.range(tuning.spawn.up_velocity_min, tuning.spawn.up_velocity_max);
        let color = Vec3::from_array(surface.color);
        let start_size = self.range(surface.start_size_min, surface.start_size_max);
        let end_size = self.range(surface.end_size_min, surface.end_size_max)
            * self.range(surface.end_variance_min, surface.end_variance_max);
        let life = self.range(surface.life_min, surface.life_max);
        let alpha = surface.alpha;
        let rise = surface.rise;
        let drag = surface.drag;
        let tint = self.range(tuning.spawn.tint_min, tuning.spawn.tint_max);
        let rotation = Quat::from_rotation_z(self.range(0.0, TAU));
        let angular_velocity = Vec3::new(
            0.0,
            0.0,
            self.range(tuning.spawn.spin_min, tuning.spawn.spin_max),
        );
        let shape_kind = if matches!(surface_kind, SurfaceKind::Grass) {
            1.0
        } else {
            0.0
        };
        let shape_variant = self.range(0.8, 1.2);
        self.particles.push(Particle {
            position: contact + jitter,
            velocity,
            rotation,
            angular_velocity,
            age: 0.0,
            life,
            start_size,
            end_size,
            color: color * tint,
            alpha,
            rise,
            drag,
            shape_kind,
            shape_variant,
        });
    }

    fn rebuild_instances(&mut self, tuning: &ParticleTuning) {
        self.instances.clear();
        let fade_in_end = tuning.fade.fade_in_end.clamp(0.0, 1.0);
        let fade_out_start = tuning.fade.fade_out_start.max(fade_in_end).clamp(0.0, 1.0);
        for particle in &self.particles {
            let t = (particle.age / particle.life).clamp(0.0, 1.0);
            let grow = 1.0 - (1.0 - t).powf(tuning.fade.grow_power.max(0.1));
            let size = particle.start_size + (particle.end_size - particle.start_size) * grow;
            let fade_in = smoothstep(0.0, fade_in_end.max(0.0001), t);
            let fade_out = 1.0 - smoothstep(fade_out_start, 1.0, t);
            let alpha = particle.alpha * fade_in * fade_out;
            self.instances.push(ParticleInstance {
                center_scale: [
                    particle.position.x,
                    particle.position.y,
                    particle.position.z,
                    size,
                ],
                rotation: [
                    particle.rotation.x,
                    particle.rotation.y,
                    particle.rotation.z,
                    particle.rotation.w,
                ],
                color_alpha: [particle.color.x, particle.color.y, particle.color.z, alpha],
                shape: [particle.shape_kind, particle.shape_variant, 0.0, 0.0],
            });
        }
    }

    fn next(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0x00ff_ffff) as f32 / 16_777_216.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next()
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn surface_kind(name: &str) -> SurfaceKind {
    let upper = name.to_ascii_uppercase();
    if upper.contains("GRASS") {
        SurfaceKind::Grass
    } else if upper.contains("KERB") {
        SurfaceKind::Kerb
    } else if upper.contains("ANELLO") {
        SurfaceKind::Concrete
    } else {
        SurfaceKind::Asphalt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{CarRig, RigWheel};
    use sim_core::WheelState;

    fn car_rig() -> CarRig {
        CarRig {
            body_mesh_index: 0,
            body_local: Mat4::IDENTITY,
            glass_mesh_index: None,
            brake_light_mesh_index: None,
            reverse_light_mesh_index: None,
            dashboard_mesh_index: None,
            dashboard_local: Mat4::IDENTITY,
            speedometer_face_mesh_index: None,
            rpm_face_mesh_index: None,
            cockpit_needles: Vec::new(),
            gear_segments: Vec::new(),
            wheels: (0..4)
                .map(|index| RigWheel {
                    mesh_index: index,
                    local_model: Mat4::from_translation(Vec3::new(
                        if index % 2 == 0 { -0.8 } else { 0.8 },
                        0.35,
                        if index < 2 { -1.2 } else { 1.2 },
                    )),
                    is_front: index < 2,
                    radius: 0.32,
                    width: 0.225,
                })
                .collect(),
            steering_mesh_index: None,
            steering_local: Mat4::IDENTITY,
            decal_mesh_index: None,
            decal_width: 0.0,
            decal_length: 0.0,
        }
    }

    fn sliding_state() -> VehicleState {
        let mut state = VehicleState {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::new(0.0, 0.0, -20.0),
            angular_velocity: Vec3::ZERO,
            wheels: [WheelState::default(); 4],
            engine_rpm: 0.0,
            gear: 1,
            steering_rack_torque: 0.0,
            tick: 0,
        };
        for wheel in &mut state.wheels {
            wheel.contact = true;
            wheel.normal_load = 4_000.0;
            wheel.slip_angle = 0.3;
            wheel.contact_normal = Vec3::Y;
        }
        state
    }

    #[test]
    fn emits_smoke_only_above_skid_threshold() {
        let mut particles = ParticleSystem::new();
        let state = sliding_state();
        for _ in 0..4 {
            particles.update(
                1.0 / 60.0,
                Mat4::IDENTITY,
                &state,
                &car_rig(),
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
        }
        assert!(particles.count() > 0);

        let mut calm = state;
        for wheel in &mut calm.wheels {
            wheel.slip_angle = 0.02;
            wheel.slip_ratio = 0.02;
        }
        particles.clear();
        particles.update(
            1.0 / 60.0,
            Mat4::IDENTITY,
            &calm,
            &car_rig(),
            &["ASPHALT".to_string()],
            &ParticleTuning::default(),
        );
        assert_eq!(particles.count(), 0);
    }

    #[test]
    fn front_wheels_do_not_emit_at_low_speed() {
        let mut state = sliding_state();
        state.velocity = Vec3::new(0.0, 0.0, -5.0);
        for (index, wheel) in state.wheels.iter_mut().enumerate() {
            if index < 2 {
                wheel.slip_angle = 0.3;
            } else {
                wheel.slip_angle = 0.0;
            }
        }
        let mut particles = ParticleSystem::new();
        for _ in 0..8 {
            particles.update(
                1.0 / 60.0,
                Mat4::IDENTITY,
                &state,
                &car_rig(),
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
        }
        assert_eq!(particles.count(), 0);

        for (index, wheel) in state.wheels.iter_mut().enumerate() {
            wheel.slip_angle = if index < 2 { 0.0 } else { 0.3 };
        }
        for _ in 0..8 {
            particles.update(
                1.0 / 60.0,
                Mat4::IDENTITY,
                &state,
                &car_rig(),
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
        }
        assert!(particles.count() > 0);
    }

    #[test]
    fn emitters_start_inside_the_tyre_track() {
        let mut particles = ParticleSystem::new();
        let rig = car_rig();
        let state = sliding_state();
        let tuning = ParticleTuning::default();
        for index in [2, 3] {
            particles.spawn(
                Mat4::IDENTITY,
                &state,
                &rig.wheels[index],
                &state.wheels[index],
                &tuning.surfaces.asphalt,
                SurfaceKind::Asphalt,
                &tuning,
            );
            let particle = particles.particles.last().expect("particle");
            assert!(
                particle.position.x.abs() < 1.0,
                "particle spawned too far outside the car: {}",
                particle.position.x
            );
        }
    }

    #[test]
    fn emits_dust_on_concrete_and_grass() {
        for surface in ["ANELLO", "GRASS"] {
            let mut particles = ParticleSystem::new();
            let state = sliding_state();
            for _ in 0..4 {
                particles.update(
                    1.0 / 60.0,
                    Mat4::IDENTITY,
                    &state,
                    &car_rig(),
                    &[surface.to_string()],
                    &ParticleTuning::default(),
                );
            }
            assert!(particles.count() > 0, "{surface}");
            let shape = if surface == "GRASS" { 1.0 } else { 0.0 };
            assert!(
                particles.instances().iter().all(|p| p.shape[0] == shape),
                "{surface} particle shape"
            );
        }
    }

    #[test]
    fn respects_the_particle_cap_and_lifecycle() {
        let mut particles = ParticleSystem::new();
        let state = sliding_state();
        for _ in 0..600 {
            particles.update(
                1.0 / 30.0,
                Mat4::IDENTITY,
                &state,
                &car_rig(),
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
        }
        assert!(particles.count() <= MAX_PARTICLES);
        particles.clear();
        assert_eq!(particles.count(), 0);
        assert!(particles.instances().is_empty());
    }

    #[test]
    fn is_deterministic_for_the_same_seed() {
        let state = sliding_state();
        let rig = car_rig();
        let mut a = ParticleSystem::new();
        let mut b = ParticleSystem::new();
        for _ in 0..30 {
            a.update(
                1.0 / 60.0,
                Mat4::IDENTITY,
                &state,
                &rig,
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
            b.update(
                1.0 / 60.0,
                Mat4::IDENTITY,
                &state,
                &rig,
                &["ASPHALT".to_string()],
                &ParticleTuning::default(),
            );
        }
        assert_eq!(a.instances(), b.instances());
    }
}
