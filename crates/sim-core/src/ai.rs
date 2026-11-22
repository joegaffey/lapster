//! Deterministic layout-following AI controller.
//!
//! The controller is a pure function of the current [`VehicleState`], the
//! [`RaceTracker`] (already updated with the car position), and its own shift
//! cooldown. It follows the race layout centreline, never the visual racing
//! line, and never selects reverse.

use glam::Vec3;

use crate::race::RaceTracker;
use crate::vehicle::{Controls, VehicleState};

/// Conservative maximum target speed on a straight, in metres per second.
const MAX_TARGET_SPEED: f32 = 32.0;
/// Conservative minimum target speed through the tightest corner.
const MIN_TARGET_SPEED: f32 = 6.0;
/// Assumed lateral acceleration budget used to derive corner speed.
const LATERAL_ACCELERATION: f32 = 6.0;
/// Steering angle that maps to full lock in the controller.
const STEERING_MAX_ANGLE: f32 = 0.55;
/// Physics ticks to wait between gear changes.
const SHIFT_COOLDOWN_TICKS: u32 = 40;
/// Default upshift threshold when no engine data is supplied.
const DEFAULT_UPSHIFT_RPM: f32 = 6_500.0;
/// Default downshift threshold when no engine data is supplied.
const DEFAULT_DOWNSHIFT_RPM: f32 = 2_800.0;
/// Default highest forward gear when no drivetrain data is supplied.
const DEFAULT_MAX_GEAR: i32 = 6;

/// A deterministic AI driver that follows a race layout centreline.
///
/// Call [`AiDriver::controls`] once per physics tick after updating the
/// tracker with the car's current position. Shift decisions use a tick-based
/// cooldown, so the result is independent of wall-clock time.
#[derive(Debug, Clone)]
pub struct AiDriver {
    shift_cooldown: u32,
    upshift_rpm: f32,
    downshift_rpm: f32,
    max_gear: i32,
}

impl Default for AiDriver {
    fn default() -> Self {
        Self::new(DEFAULT_UPSHIFT_RPM, DEFAULT_DOWNSHIFT_RPM, DEFAULT_MAX_GEAR)
    }
}

impl AiDriver {
    /// Builds a driver with car-specific shift thresholds.
    ///
    /// Non-finite inputs fall back to conservative defaults. `max_gear` is
    /// clamped to at least first gear.
    pub fn new(upshift_rpm: f32, downshift_rpm: f32, max_gear: i32) -> Self {
        let upshift_rpm = if upshift_rpm.is_finite() && upshift_rpm > 0.0 {
            upshift_rpm
        } else {
            DEFAULT_UPSHIFT_RPM
        };
        let downshift_rpm = if downshift_rpm.is_finite() && downshift_rpm >= 0.0 {
            downshift_rpm.min(upshift_rpm)
        } else {
            DEFAULT_DOWNSHIFT_RPM
        };
        Self {
            shift_cooldown: 0,
            upshift_rpm,
            downshift_rpm,
            max_gear: max_gear.max(1),
        }
    }

    /// Clears the shift cooldown, for example after a car reset.
    pub fn reset(&mut self) {
        self.shift_cooldown = 0;
    }

    /// Returns deterministic controls for the next physics tick.
    pub fn controls(&mut self, state: &VehicleState, tracker: &RaceTracker) -> Controls {
        let gear = state.gear.clamp(1, self.max_gear);
        let mut controls = Controls {
            gear,
            ..Controls::default()
        };
        if !state_is_finite(state) {
            return controls;
        }

        self.shift_cooldown = self.shift_cooldown.saturating_sub(1);

        let velocity = Vec3::new(state.velocity.x, 0.0, state.velocity.z);
        let speed = velocity.length();
        if tracker.total_length() <= 1e-3 {
            self.shift_gears(state, &mut controls);
            return controls;
        }

        let distance = tracker.state().distance_m;
        let lookahead = (8.0 + speed * 0.35).clamp(8.0, 24.0);
        let Some(sample) = tracker.sample(distance + lookahead) else {
            self.shift_gears(state, &mut controls);
            return controls;
        };

        let forward = horizontal_direction(state.rotation * Vec3::NEG_Z);
        let right = horizontal_direction(state.rotation * Vec3::X);
        let offset = Vec3::new(
            sample.point.x - state.position.x,
            0.0,
            sample.point.z - state.position.z,
        );
        if forward.length_squared() > 0.5 && right.length_squared() > 0.5 {
            let forward_component = offset.dot(forward);
            let right_component = offset.dot(right);
            let desired_angle = -right_component.atan2(forward_component);
            controls.steering = (desired_angle / STEERING_MAX_ANGLE).clamp(-1.0, 1.0);
        }

        let target_speed = target_speed(tracker, distance, lookahead);
        if speed < target_speed - 0.5 {
            controls.throttle = ((target_speed - speed) / 6.0).clamp(0.25, 1.0);
        } else if speed > target_speed + 1.0 {
            controls.brake = ((speed - target_speed) / 6.0).clamp(0.15, 1.0);
        } else {
            controls.throttle = 0.3;
        }

        self.shift_gears(state, &mut controls);
        controls
    }

    fn shift_gears(&mut self, state: &VehicleState, controls: &mut Controls) {
        if self.shift_cooldown > 0 {
            return;
        }
        if state.engine_rpm >= self.upshift_rpm && controls.gear < self.max_gear {
            controls.gear += 1;
            self.shift_cooldown = SHIFT_COOLDOWN_TICKS;
        } else if state.engine_rpm <= self.downshift_rpm && controls.gear > 1 {
            controls.gear -= 1;
            self.shift_cooldown = SHIFT_COOLDOWN_TICKS;
        }
    }
}

fn state_is_finite(state: &VehicleState) -> bool {
    state.position.is_finite()
        && state.rotation.is_finite()
        && state.velocity.is_finite()
        && state.angular_velocity.is_finite()
        && state.engine_rpm.is_finite()
}

fn horizontal_direction(vector: Vec3) -> Vec3 {
    let horizontal = Vec3::new(vector.x, 0.0, vector.z);
    if horizontal.length_squared() > 1e-6 {
        horizontal.normalize()
    } else {
        Vec3::ZERO
    }
}

/// Conservative target speed from the maximum layout curvature ahead.
fn target_speed(tracker: &RaceTracker, distance: f32, lookahead: f32) -> f32 {
    let mut curvature = 0.0f32;
    let mut probe = 4.0;
    let end = lookahead + 12.0;
    while probe <= end {
        if let Some(sample) = tracker.sample(distance + probe) {
            if sample.curvature.is_finite() {
                curvature = curvature.max(sample.curvature);
            }
        }
        probe += 4.0;
    }
    let target = if curvature <= 1e-4 {
        MAX_TARGET_SPEED
    } else {
        (LATERAL_ACCELERATION / curvature).sqrt()
    };
    target.clamp(MIN_TARGET_SPEED, MAX_TARGET_SPEED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::WheelState;
    use glam::Quat;
    use sim_types::{
        Layout, LayoutDirection, RaceData, SectorSpec, SpawnPoint, StartFinish, RACE_DATA_VERSION,
    };

    fn race_data(points: Vec<[f32; 3]>, closed: bool) -> RaceData {
        RaceData {
            version: RACE_DATA_VERSION,
            layout: Layout {
                layout_id: String::from("test"),
                closed,
                direction: LayoutDirection::Forward,
                points,
                half_width: 5.0,
            },
            start_finish: StartFinish {
                section_id: String::from("test"),
                fraction: 0.0,
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            grid: vec![SpawnPoint {
                name: String::from("grid_0"),
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            }],
            sectors: vec![SectorSpec {
                name: String::from("sector_1"),
                start_fraction: 0.0,
                end_fraction: 1.0,
            }],
        }
    }

    fn tracker(points: Vec<[f32; 3]>, closed: bool, position: Vec3) -> RaceTracker {
        let mut tracker = RaceTracker::new(&race_data(points, closed));
        tracker.update(position, 0);
        tracker
    }

    fn vehicle_state(
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        engine_rpm: f32,
        gear: i32,
    ) -> VehicleState {
        VehicleState {
            position,
            rotation,
            velocity,
            angular_velocity: Vec3::ZERO,
            wheels: [WheelState::default(); 4],
            engine_rpm,
            gear,
            steering_rack_torque: 0.0,
            tick: 0,
        }
    }

    fn straight_tracker(position: Vec3) -> RaceTracker {
        tracker(vec![[0.0, 0.0, 0.0], [0.0, 0.0, -100.0]], false, position)
    }

    #[test]
    fn straight_produces_throttle_without_steering() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = straight_tracker(position);
        let state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        let mut driver = AiDriver::default();

        let controls = driver.controls(&state, &tracker);

        assert!(controls.throttle > 0.5);
        assert!(controls.brake < 0.01);
        assert!(controls.steering.abs() < 0.05);
        assert_eq!(controls.gear, 1);
    }

    #[test]
    fn steers_toward_a_right_corner() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -10.0], [10.0, 0.0, -10.0]],
            false,
            position,
        );
        let state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        let mut driver = AiDriver::default();

        let controls = driver.controls(&state, &tracker);

        assert!(
            controls.steering < -0.05,
            "expected a right turn, got {}",
            controls.steering
        );
    }

    #[test]
    fn steers_toward_a_left_corner() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -10.0], [-10.0, 0.0, -10.0]],
            false,
            position,
        );
        let state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        let mut driver = AiDriver::default();

        let controls = driver.controls(&state, &tracker);

        assert!(
            controls.steering > 0.05,
            "expected a left turn, got {}",
            controls.steering
        );
    }

    #[test]
    fn target_speed_drops_for_a_tighter_corner() {
        let position = Vec3::new(0.0, 0.0, -1.0);
        let straight = straight_tracker(position);
        let corner = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -5.0], [5.0, 0.0, -5.0]],
            false,
            position,
        );
        let state = vehicle_state(
            position,
            Quat::IDENTITY,
            Vec3::new(0.0, 0.0, -20.0),
            900.0,
            1,
        );

        let straight_controls = AiDriver::default().controls(&state, &straight);
        let corner_controls = AiDriver::default().controls(&state, &corner);

        assert!(straight_controls.throttle > 0.5);
        assert!(straight_controls.brake < 0.01);
        assert!(corner_controls.brake > 0.5);
        assert!(corner_controls.throttle < 0.01);
    }

    #[test]
    fn gears_upshift_and_respect_cooldown() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = straight_tracker(position);
        let mut state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 7_000.0, 1);
        let mut driver = AiDriver::default();

        let first = driver.controls(&state, &tracker);
        assert_eq!(first.gear, 2);

        state.gear = first.gear;
        let cooldown = driver.controls(&state, &tracker);
        assert_eq!(cooldown.gear, 2);

        let mut gear = cooldown.gear;
        for _ in 0..60 {
            state.gear = gear;
            gear = driver.controls(&state, &tracker).gear;
        }
        assert!(gear >= 3, "expected another upshift, got {gear}");
    }

    #[test]
    fn gears_downshift_without_selecting_reverse() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = straight_tracker(position);
        let mut driver = AiDriver::default();

        let state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 2_000.0, 3);
        assert_eq!(driver.controls(&state, &tracker).gear, 2);

        let mut driver = AiDriver::default();
        let reverse = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, -1);
        assert!(driver.controls(&reverse, &tracker).gear >= 1);
    }

    #[test]
    fn degenerate_layout_and_non_finite_state_return_finite_controls() {
        let degenerate = RaceTracker::new(&race_data(Vec::new(), true));
        let position = Vec3::new(0.0, 0.0, 0.0);
        let mut driver = AiDriver::default();

        let bad_state = vehicle_state(
            Vec3::new(f32::NAN, 0.0, 0.0),
            Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
            Vec3::new(f32::INFINITY, 0.0, 0.0),
            f32::NAN,
            -1,
        );
        let bad_controls = driver.controls(&bad_state, &degenerate);
        assert_finite(&bad_controls);

        let good_state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        let controls = driver.controls(&good_state, &degenerate);
        assert_finite(&controls);
        assert!(controls.gear >= 1);
    }

    fn assert_finite(controls: &Controls) {
        assert!(controls.steering.is_finite());
        assert!(controls.throttle.is_finite());
        assert!(controls.brake.is_finite());
        assert!(controls.clutch.is_finite());
        assert!(controls.handbrake.is_finite());
    }
}
