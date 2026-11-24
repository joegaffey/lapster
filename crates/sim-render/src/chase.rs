//! Secondary motion for the chase camera.
//!
//! The chase camera is the only shot that is allowed to lag the car. Horizontal
//! position (xz) and yaw follow the car with framerate-independent exponential
//! smoothing; vertical position and the car's pitch/roll stay rigid so crests
//! do not bob and the sim feel is preserved. A small decaying shake, driven by
//! per-physics-step velocity changes, adds arcade impact on landings/contacts.
//!
//! Everything here is pure state so the behaviour can be unit tested without a
//! renderer.

use glam::{Quat, Vec2, Vec3};

use crate::tuning::ChaseTuning;

const TAU: f32 = std::f32::consts::TAU;

/// Persistent animation state for the chase camera.
#[derive(Clone, Debug, Default)]
pub struct ChaseAnimation {
    initialized: bool,
    yaw: f32,
    position_xz: Vec2,
    last_car_xz: Vec2,
    shake_amplitude: f32,
    shake_phase: f32,
}

/// Camera inputs derived from the animated state.
#[derive(Clone, Copy, Debug)]
pub struct ChasePose {
    /// Smoothed horizontal anchor with the car's rigid vertical position.
    pub anchor: Vec3,
    /// The car rotation with the smoothed yaw offset applied.
    pub eye_rotation: Quat,
    /// Deterministic shake offset to add to the eye.
    pub shake_offset: Vec3,
}

impl ChaseAnimation {
    /// Drops all smoothing and shake state, aligning with the car exactly.
    pub fn snap(&mut self, position: Vec3, rotation: Quat) {
        let xz = Vec2::new(position.x, position.z);
        self.initialized = true;
        self.position_xz = xz;
        self.last_car_xz = xz;
        self.yaw = yaw_from_rotation(rotation);
        self.shake_amplitude = 0.0;
        self.shake_phase = 0.0;
    }

    /// Clears any pending shake without touching the follow smoothing.
    pub fn clear_shake(&mut self) {
        self.shake_amplitude = 0.0;
        self.shake_phase = 0.0;
    }

    /// Feeds one physics-step velocity change into the shake accumulator.
    ///
    /// Vertical and horizontal impulses are thresholded separately, scaled,
    /// rate-limited per step and clamped to the maximum amplitude.
    pub fn push_impulse(&mut self, delta_velocity: Vec3, tuning: &ChaseTuning) {
        if tuning.animation_strength <= 0.0 {
            return;
        }
        let vertical_excess =
            (delta_velocity.y.abs() - tuning.shake_threshold_vertical.max(0.0)).max(0.0);
        let horizontal_excess = (Vec2::new(delta_velocity.x, delta_velocity.z).length()
            - tuning.shake_threshold_horizontal.max(0.0))
        .max(0.0);
        let impulse = ((vertical_excess + horizontal_excess) * tuning.shake_impulse_scale.max(0.0))
            .min(tuning.shake_max_impulse.max(0.0));
        if impulse <= 0.0 {
            return;
        }
        self.shake_amplitude =
            (self.shake_amplitude + impulse).min(tuning.shake_amplitude.max(0.0));
    }

    /// Advances smoothing and shake decay, returning the camera pose.
    ///
    /// `dt <= 0`, an uninitialised state or a large car jump snap to the car.
    pub fn update(
        &mut self,
        position: Vec3,
        rotation: Quat,
        dt: f32,
        tuning: &ChaseTuning,
    ) -> ChasePose {
        let car_xz = Vec2::new(position.x, position.z);
        let target_yaw = yaw_from_rotation(rotation);
        let teleport = self.last_car_xz.distance(car_xz) > tuning.snap_distance.max(0.0);
        if !self.initialized || dt <= 0.0 || teleport {
            self.snap(position, rotation);
            return self.pose(position, rotation, tuning);
        }

        let position_alpha = smoothing_alpha(dt, tuning.position_lag);
        let yaw_alpha = smoothing_alpha(dt, tuning.yaw_lag);

        self.position_xz = self.position_xz.lerp(car_xz, position_alpha);
        self.position_xz = clamp_horizontal_offset(
            car_xz,
            self.position_xz,
            tuning.max_position_offset.max(0.0),
        );

        let yaw_step = wrap_angle(target_yaw - self.yaw) * yaw_alpha;
        self.yaw = wrap_angle(self.yaw + yaw_step);
        self.yaw = clamp_yaw_offset(target_yaw, self.yaw, tuning.max_yaw_offset.max(0.0));

        self.last_car_xz = car_xz;

        // Shake decays in real time and only the live path advances it.
        let decay = (-tuning.shake_decay.max(0.0) * dt).exp();
        self.shake_amplitude *= decay;
        if self.shake_amplitude < 1e-5 {
            self.shake_amplitude = 0.0;
        }
        self.shake_phase =
            (self.shake_phase + dt * tuning.shake_frequency.max(0.0) * TAU).rem_euclid(TAU);

        self.pose(position, rotation, tuning)
    }

    /// Builds a camera pose from the current state without advancing it.
    pub fn pose(&self, position: Vec3, rotation: Quat, tuning: &ChaseTuning) -> ChasePose {
        let strength = tuning.animation_strength.clamp(0.0, 1.0);
        let car_xz = Vec2::new(position.x, position.z);
        let car_yaw = yaw_from_rotation(rotation);

        let mut horizontal_offset = (self.position_xz - car_xz) * strength;
        let max_position = tuning.max_position_offset.max(0.0);
        let offset_len = horizontal_offset.length();
        if offset_len > max_position && offset_len > 0.0 {
            horizontal_offset *= max_position / offset_len;
        }

        let max_yaw = tuning.max_yaw_offset.max(0.0);
        let yaw_offset = (wrap_angle(self.yaw - car_yaw) * strength).clamp(-max_yaw, max_yaw);

        let anchor = Vec3::new(
            car_xz.x + horizontal_offset.x,
            position.y,
            car_xz.y + horizontal_offset.y,
        );
        let eye_rotation = Quat::from_rotation_y(yaw_offset) * rotation;
        let shake_offset = self.shake_offset(tuning) * strength;

        ChasePose {
            anchor,
            eye_rotation,
            shake_offset,
        }
    }

    fn shake_offset(&self, tuning: &ChaseTuning) -> Vec3 {
        let amplitude = self.shake_amplitude.min(tuning.shake_amplitude.max(0.0));
        if amplitude <= 0.0 {
            return Vec3::ZERO;
        }
        let phase = self.shake_phase;
        // Integer harmonic multipliers keep each axis exactly periodic in
        // TAU, so the waveform stays continuous when `shake_phase` wraps.
        // Distinct phases per axis keep the shake from looking like a single
        // sine wave, while staying deterministic.
        Vec3::new(
            phase.sin() * amplitude * 0.6,
            (phase * 2.0 + 1.3).sin() * amplitude,
            (phase * 3.0 + 2.1).sin() * amplitude * 0.4,
        )
    }
}

/// Framerate-independent exponential smoothing factor.
fn smoothing_alpha(dt: f32, lag: f32) -> f32 {
    if lag <= 1e-4 {
        1.0
    } else {
        1.0 - (-dt.max(0.0) / lag).exp()
    }
}

/// Wraps an angle to `(-pi, pi]`.
fn wrap_angle(angle: f32) -> f32 {
    let wrapped = (angle + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
    if wrapped <= -std::f32::consts::PI {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// Extracts the car heading around the Y axis.
fn yaw_from_rotation(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::NEG_Z;
    (-forward.x).atan2(-forward.z)
}

fn clamp_horizontal_offset(target: Vec2, value: Vec2, max: f32) -> Vec2 {
    let offset = value - target;
    let length = offset.length();
    if length > max && length > 0.0 {
        target + offset * (max / length)
    } else {
        value
    }
}

fn clamp_yaw_offset(target_yaw: f32, yaw: f32, max: f32) -> f32 {
    wrap_angle(target_yaw + wrap_angle(yaw - target_yaw).clamp(-max, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> ChaseTuning {
        ChaseTuning::default()
    }

    #[test]
    fn smoothing_is_framerate_independent() {
        let mut tuning = tuning();
        tuning.max_position_offset = 100.0;
        tuning.max_yaw_offset = 10.0;
        tuning.snap_distance = 1_000.0;

        let start = Vec3::new(1.0, 0.0, 2.0);
        let target = Vec3::new(4.5, 0.0, -3.0);
        let start_rotation = Quat::IDENTITY;
        let target_rotation = Quat::from_rotation_y(0.9);

        let dt = 0.05;

        let mut once = ChaseAnimation::default();
        once.snap(start, start_rotation);
        once.update(target, target_rotation, dt, &tuning);

        let mut twice = ChaseAnimation::default();
        twice.snap(start, start_rotation);
        twice.update(target, target_rotation, dt * 0.5, &tuning);
        twice.update(target, target_rotation, dt * 0.5, &tuning);

        assert!(
            (once.position_xz - twice.position_xz).length() < 1e-5,
            "position smoothing differs: {:?} vs {:?}",
            once.position_xz,
            twice.position_xz
        );
        assert!(
            (wrap_angle(once.yaw - twice.yaw)).abs() < 1e-5,
            "yaw smoothing differs: {} vs {}",
            once.yaw,
            twice.yaw
        );
    }

    #[test]
    fn snaps_on_first_frame_dt_zero_and_teleport() {
        let tuning = tuning();
        let start = Vec3::new(0.0, 0.0, 0.0);
        let moved = Vec3::new(3.0, 0.0, 0.0);

        let mut animation = ChaseAnimation::default();
        let pose = animation.update(moved, Quat::IDENTITY, 0.016, &tuning);
        assert!((pose.anchor - moved).length() < 1e-6);

        // Build up a lag, then a zero-dt update snaps back to the car.
        animation.update(Vec3::new(8.0, 0.0, 0.0), Quat::IDENTITY, 0.016, &tuning);
        let snapped = animation.update(start, Quat::IDENTITY, 0.0, &tuning);
        assert!((snapped.anchor - start).length() < 1e-6);

        // A large jump also snaps.
        let far = Vec3::new(500.0, 0.0, 0.0);
        let jumped = animation.update(far, Quat::IDENTITY, 0.016, &tuning);
        assert!((jumped.anchor - far).length() < 1e-6);
    }

    #[test]
    fn clamps_position_and_yaw_offsets() {
        let mut tuning = tuning();
        tuning.max_position_offset = 2.0;
        tuning.max_yaw_offset = 0.17;
        tuning.snap_distance = 1_000.0;

        let mut animation = ChaseAnimation::default();
        animation.snap(Vec3::ZERO, Quat::IDENTITY);
        // One large step toward a distant sideways/rotated target.
        animation.update(
            Vec3::new(50.0, 0.0, 0.0),
            Quat::from_rotation_y(1.4),
            0.05,
            &tuning,
        );

        let car = Vec3::new(50.0, 0.0, 0.0);
        let horizontal = Vec2::new(
            animation.position_xz.x - car.x,
            animation.position_xz.y - car.z,
        );
        assert!(
            horizontal.length() <= tuning.max_position_offset + 1e-4,
            "position offset {} exceeds clamp",
            horizontal.length()
        );

        let car_yaw = yaw_from_rotation(Quat::from_rotation_y(1.4));
        let yaw_offset = wrap_angle(animation.yaw - car_yaw).abs();
        assert!(
            yaw_offset <= tuning.max_yaw_offset + 1e-4,
            "yaw offset {} exceeds clamp",
            yaw_offset
        );
    }

    #[test]
    fn shake_is_bounded_and_decays() {
        let mut tuning = tuning();
        tuning.shake_amplitude = 0.05;
        tuning.shake_max_impulse = 0.02;
        tuning.shake_impulse_scale = 0.1;
        tuning.shake_threshold_vertical = 1.0;
        tuning.shake_threshold_horizontal = 1.0;

        let mut animation = ChaseAnimation::default();
        animation.snap(Vec3::ZERO, Quat::IDENTITY);

        for _ in 0..100 {
            animation.push_impulse(Vec3::new(0.0, 40.0, 0.0), &tuning);
        }
        assert!(animation.shake_amplitude <= tuning.shake_amplitude + 1e-6);

        let position = Vec3::ZERO;
        let rotation = Quat::IDENTITY;
        let mut previous = animation.shake_amplitude;
        for _ in 0..60 {
            let pose = animation.update(position, rotation, 0.016, &tuning);
            assert!(pose.shake_offset.length() <= tuning.shake_amplitude * 1.5);
            assert!(animation.shake_amplitude <= previous + 1e-6);
            previous = animation.shake_amplitude;
        }
        for _ in 0..240 {
            animation.update(position, rotation, 0.016, &tuning);
        }
        assert!(animation.shake_amplitude < 1e-3, "shake did not decay");

        // Steady state produces no new impulses.
        animation.clear_shake();
        for _ in 0..100 {
            animation.push_impulse(Vec3::new(0.01, -0.01, 0.02), &tuning);
        }
        assert_eq!(animation.shake_amplitude, 0.0);
    }

    #[test]
    fn shake_offset_is_continuous_across_phase_wrap() {
        let mut tuning = tuning();
        tuning.shake_amplitude = 1.0;

        let mut animation = ChaseAnimation {
            shake_amplitude: 1.0,
            ..ChaseAnimation::default()
        };

        let epsilon = 1e-4;

        // Samples just before and just after the phase wraps from TAU to 0
        // must be close; non-integer harmonics would jump here.
        animation.shake_phase = TAU - epsilon;
        let before = animation.shake_offset(&tuning);
        animation.shake_phase = 0.0;
        let after = animation.shake_offset(&tuning);
        assert!(
            (before - after).length() < 1e-2,
            "shake jumps across wrap: {:?} vs {:?}",
            before,
            after
        );

        // The same holds when straddling the wrap from either side.
        animation.shake_phase = TAU - epsilon;
        let before = animation.shake_offset(&tuning);
        animation.shake_phase = epsilon;
        let after = animation.shake_offset(&tuning);
        assert!(
            (before - after).length() < 1e-2,
            "shake jumps across wrap: {:?} vs {:?}",
            before,
            after
        );
    }

    #[test]
    fn zero_strength_is_rigid() {
        let mut tuning = tuning();
        tuning.animation_strength = 0.0;
        tuning.max_position_offset = 2.0;
        tuning.max_yaw_offset = 0.17;
        tuning.snap_distance = 1_000.0;

        let mut animation = ChaseAnimation::default();
        animation.snap(Vec3::ZERO, Quat::IDENTITY);
        animation.push_impulse(Vec3::new(0.0, 20.0, 0.0), &tuning);

        let position = Vec3::new(9.0, 1.0, -4.0);
        let rotation = Quat::from_rotation_y(0.8);
        let pose = animation.update(position, rotation, 0.033, &tuning);

        assert!((pose.anchor - position).length() < 1e-6);
        let expected = Quat::from_rotation_y(0.0) * rotation;
        assert!(
            (pose.eye_rotation - expected).length() < 1e-6,
            "rigid rotation changed: {:?} vs {:?}",
            pose.eye_rotation,
            expected
        );
        assert!(pose.shake_offset.length() < 1e-6);
    }
}
