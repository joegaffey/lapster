//! Deterministic layout-following AI controller.
//!
//! The controller is a pure function of the current [`VehicleState`], the
//! [`RaceTracker`] (already updated with the car position), and its own shift
//! cooldown. It follows the race layout centreline, never the visual racing
//! line, and never selects reverse.

use glam::Vec3;

use crate::race::RaceTracker;
use crate::vehicle::{Controls, VehicleState, FIXED_DT};

/// Conservative maximum target speed on a straight, in metres per second.
const MAX_TARGET_SPEED: f32 = 32.0;
/// Conservative minimum target speed through the tightest corner.
const MIN_TARGET_SPEED: f32 = 6.0;
/// Assumed lateral acceleration budget used to derive corner speed.
const LATERAL_ACCELERATION: f32 = 6.0;

/// Selectable difficulty preset for the layout-following AI controller.
///
/// Pace only scales the controller's target speed and lateral acceleration
/// budget. It never changes the vehicle model, collider, setup, or assists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiPace {
    /// Slower target speed and cornering budget.
    Chill,
    /// Default reference pace.
    #[default]
    Club,
    /// Faster target speed and cornering budget.
    Fast,
}

impl AiPace {
    /// Maps a worker pace index to a preset. Unknown indices fall back to
    /// [`AiPace::Club`].
    pub fn from_index(index: u32) -> AiPace {
        match index {
            0 => AiPace::Chill,
            2 => AiPace::Fast,
            _ => AiPace::Club,
        }
    }

    /// Multiplier applied to the controller's target-speed limits.
    pub fn target_speed_scale(self) -> f32 {
        match self {
            AiPace::Chill => 0.85,
            AiPace::Club => 1.0,
            AiPace::Fast => 1.12,
        }
    }

    /// Multiplier applied to the controller's lateral acceleration budget.
    pub fn lateral_acceleration_scale(self) -> f32 {
        match self {
            AiPace::Chill => 0.75,
            AiPace::Club => 1.0,
            AiPace::Fast => 1.22,
        }
    }
}

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
/// Default maximum steering command change in normalized units per second.
const DEFAULT_STEERING_RATE: f32 = 3.0;
/// Largest accepted steering rate in normalized units per second.
const MAX_STEERING_RATE: f32 = 20.0;
/// Speed above the target at which the controller starts braking.
///
/// The enter margin is deliberately wide so a small overspeed coasts instead of
/// touching the brake, which used to start a throttle/brake limit cycle.
const BRAKE_ENTER_MARGIN: f32 = 2.0;
/// Speed below the target at which the controller releases the brake.
///
/// Together with [`BRAKE_ENTER_MARGIN`] this gives a 3 m/s hysteresis band.
const BRAKE_RELEASE_MARGIN: f32 = 1.0;
/// Speed band below the target where Drive mode coasts instead of applying a
/// tiny throttle, preventing a low-amplitude throttle/brake oscillation.
const THROTTLE_DEADBAND: f32 = 0.5;
/// Fraction of the remaining target-speed error removed per physics tick.
const TARGET_SPEED_BLEND: f32 = 0.02;
/// Maximum throttle command change in normalized units per second.
const THROTTLE_RATE: f32 = 8.0;
/// Maximum brake command change in normalized units per second.
const BRAKE_RATE: f32 = 12.0;
/// Largest throttle and brake value allowed to overlap.
const CONTROL_OVERLAP_LIMIT: f32 = 0.1;

/// Throttle/brake mode with hysteresis around the target speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DriveMode {
    /// Accelerate or coast toward the target speed.
    #[default]
    Drive,
    /// Brake down toward the target speed.
    Brake,
}

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
    pace: AiPace,
    steering_rate: f32,
    last_steering: f32,
    mode: DriveMode,
    last_throttle: f32,
    last_brake: f32,
    smoothed_target: Option<f32>,
    controls_initialized: bool,
}

impl Default for AiDriver {
    fn default() -> Self {
        Self::new(
            DEFAULT_UPSHIFT_RPM,
            DEFAULT_DOWNSHIFT_RPM,
            DEFAULT_MAX_GEAR,
            AiPace::default(),
        )
    }
}

impl AiDriver {
    /// Builds a driver with car-specific shift thresholds and a pace preset.
    ///
    /// Non-finite inputs fall back to conservative defaults. `max_gear` is
    /// clamped to at least first gear.
    pub fn new(upshift_rpm: f32, downshift_rpm: f32, max_gear: i32, pace: AiPace) -> Self {
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
            pace,
            steering_rate: DEFAULT_STEERING_RATE,
            last_steering: 0.0,
            mode: DriveMode::Drive,
            last_throttle: 0.0,
            last_brake: 0.0,
            smoothed_target: None,
            controls_initialized: false,
        }
    }

    /// Sets the maximum steering command change in normalized units per second.
    ///
    /// Non-finite values fall back to the default rate. The rate is clamped to
    /// `0..=20`, where `0` disables the limit.
    pub fn set_steering_rate(&mut self, rate: f32) {
        self.steering_rate = sanitize_steering_rate(rate);
    }

    /// Returns a copy of the driver with the given steering rate.
    ///
    /// See [`AiDriver::set_steering_rate`] for sanitization and clamping.
    pub fn with_steering_rate(mut self, rate: f32) -> Self {
        self.set_steering_rate(rate);
        self
    }

    /// Clears the shift cooldown, stored steering command, drive mode, and
    /// throttle/brake state, for example after a car reset.
    pub fn reset(&mut self) {
        self.shift_cooldown = 0;
        self.last_steering = 0.0;
        self.mode = DriveMode::Drive;
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        self.smoothed_target = None;
        self.controls_initialized = false;
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
        let mut desired_steering = 0.0;
        if forward.length_squared() > 0.5 && right.length_squared() > 0.5 {
            let forward_component = offset.dot(forward);
            let right_component = offset.dot(right);
            let desired_angle = -right_component.atan2(forward_component);
            desired_steering = (desired_angle / STEERING_MAX_ANGLE).clamp(-1.0, 1.0);
        }
        controls.steering = self.rate_limit_steering(desired_steering);

        let raw_target = target_speed(tracker, distance, lookahead, self.pace);
        let target = self.smooth_target(raw_target);
        self.update_mode(speed, target);

        let (desired_throttle, desired_brake) = self.desired_controls(speed, target);
        self.apply_controls(desired_throttle, desired_brake);
        controls.throttle = self.last_throttle;
        controls.brake = self.last_brake;

        self.shift_gears(state, &mut controls);
        controls
    }

    fn smooth_target(&mut self, raw_target: f32) -> f32 {
        let target = match self.smoothed_target {
            Some(previous) => previous + (raw_target - previous) * TARGET_SPEED_BLEND,
            None => raw_target,
        };
        self.smoothed_target = Some(target);
        target
    }

    fn update_mode(&mut self, speed: f32, target: f32) {
        match self.mode {
            DriveMode::Drive => {
                if speed > target + BRAKE_ENTER_MARGIN {
                    self.mode = DriveMode::Brake;
                }
            }
            DriveMode::Brake => {
                if speed < target - BRAKE_RELEASE_MARGIN {
                    self.mode = DriveMode::Drive;
                }
            }
        }
    }

    fn desired_controls(&self, speed: f32, target: f32) -> (f32, f32) {
        match self.mode {
            DriveMode::Drive => {
                // Coast inside a small deadband around the target instead of
                // dosing in a tiny throttle that would trigger a brake shortly
                // after. Throttle only resumes below `target - deadband`.
                let throttle = ((target - THROTTLE_DEADBAND - speed) / 6.0).clamp(0.0, 1.0);
                (throttle, 0.0)
            }
            DriveMode::Brake => {
                let brake = ((speed - target) / 6.0).clamp(0.15, 1.0);
                (0.0, brake)
            }
        }
    }

    fn apply_controls(&mut self, desired_throttle: f32, desired_brake: f32) {
        if !self.controls_initialized {
            // The first command after reset is applied directly so the car
            // does not hesitate at launch.
            self.last_throttle = desired_throttle;
            self.last_brake = desired_brake;
            self.controls_initialized = true;
        } else {
            let throttle_delta = THROTTLE_RATE * FIXED_DT;
            let brake_delta = BRAKE_RATE * FIXED_DT;
            self.last_throttle +=
                (desired_throttle - self.last_throttle).clamp(-throttle_delta, throttle_delta);
            self.last_brake += (desired_brake - self.last_brake).clamp(-brake_delta, brake_delta);
        }

        // Never command throttle and brake together above a small threshold.
        if self.last_throttle > CONTROL_OVERLAP_LIMIT && self.last_brake > CONTROL_OVERLAP_LIMIT {
            if self.mode == DriveMode::Brake {
                self.last_throttle = 0.0;
            } else {
                self.last_brake = 0.0;
            }
        }

        self.last_throttle = self.last_throttle.clamp(0.0, 1.0);
        self.last_brake = self.last_brake.clamp(0.0, 1.0);
    }

    fn rate_limit_steering(&mut self, desired: f32) -> f32 {
        let desired = desired.clamp(-1.0, 1.0);
        if self.steering_rate <= 0.0 {
            self.last_steering = desired;
            return desired;
        }
        let max_delta = self.steering_rate * FIXED_DT;
        let delta = (desired - self.last_steering).clamp(-max_delta, max_delta);
        self.last_steering = (self.last_steering + delta).clamp(-1.0, 1.0);
        self.last_steering
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

fn sanitize_steering_rate(rate: f32) -> f32 {
    if rate.is_finite() {
        rate.clamp(0.0, MAX_STEERING_RATE)
    } else {
        DEFAULT_STEERING_RATE
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
fn target_speed(tracker: &RaceTracker, distance: f32, lookahead: f32, pace: AiPace) -> f32 {
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
    let speed_scale = pace.target_speed_scale();
    let max_target_speed = MAX_TARGET_SPEED * speed_scale;
    let min_target_speed = MIN_TARGET_SPEED * speed_scale;
    let lateral_acceleration = LATERAL_ACCELERATION * pace.lateral_acceleration_scale();
    let target = if curvature <= 1e-4 {
        max_target_speed
    } else {
        (lateral_acceleration / curvature).sqrt()
    };
    target.clamp(min_target_speed, max_target_speed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::tests::{
        collider as vehicle_collider, physics as vehicle_physics, spawn as vehicle_spawn,
    };
    use crate::vehicle::{Vehicle, WheelState};
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

        let mut controls = driver.controls(&state, &tracker);
        for _ in 0..60 {
            if controls.steering.abs() > 0.05 {
                break;
            }
            controls = driver.controls(&state, &tracker);
        }

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

        let mut controls = driver.controls(&state, &tracker);
        for _ in 0..60 {
            if controls.steering.abs() > 0.05 {
                break;
            }
            controls = driver.controls(&state, &tracker);
        }

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

    #[test]
    fn pace_from_index_maps_known_indices_and_falls_back() {
        assert_eq!(AiPace::from_index(0), AiPace::Chill);
        assert_eq!(AiPace::from_index(1), AiPace::Club);
        assert_eq!(AiPace::from_index(2), AiPace::Fast);
        assert_eq!(AiPace::from_index(3), AiPace::Club);
        assert_eq!(AiPace::from_index(u32::MAX), AiPace::Club);
        assert_eq!(AiPace::default(), AiPace::Club);
    }

    #[test]
    fn pace_changes_target_speed_monotonically_on_the_same_layout() {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let straight = straight_tracker(position);
        let straight_distance = straight.state().distance_m;
        let chill = target_speed(&straight, straight_distance, 8.0, AiPace::Chill);
        let club = target_speed(&straight, straight_distance, 8.0, AiPace::Club);
        let fast = target_speed(&straight, straight_distance, 8.0, AiPace::Fast);
        assert!(
            chill < club && club < fast,
            "straight {chill} {club} {fast}"
        );

        let corner: Vec<[f32; 3]> = (0..64)
            .map(|index| {
                let angle = index as f32 / 64.0 * std::f32::consts::TAU;
                [40.0 * angle.cos(), 0.0, 40.0 * angle.sin()]
            })
            .collect();
        let curved = tracker(corner, true, Vec3::new(40.0, 0.0, 0.0));
        let chill = target_speed(&curved, 0.0, 8.0, AiPace::Chill);
        let club = target_speed(&curved, 0.0, 8.0, AiPace::Club);
        let fast = target_speed(&curved, 0.0, 8.0, AiPace::Fast);
        assert!(chill < club && club < fast, "corner {chill} {club} {fast}");
    }

    #[test]
    fn all_paces_keep_controls_finite() {
        let degenerate = RaceTracker::new(&race_data(Vec::new(), true));
        let position = Vec3::new(0.0, 0.0, 0.0);
        let good_state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        let bad_state = vehicle_state(
            Vec3::new(f32::NAN, 0.0, 0.0),
            Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
            Vec3::new(f32::INFINITY, 0.0, 0.0),
            f32::NAN,
            -1,
        );

        for pace in [AiPace::Chill, AiPace::Club, AiPace::Fast] {
            let mut driver = AiDriver::new(
                DEFAULT_UPSHIFT_RPM,
                DEFAULT_DOWNSHIFT_RPM,
                DEFAULT_MAX_GEAR,
                pace,
            );
            assert_finite(&driver.controls(&bad_state, &degenerate));
            assert_finite(&driver.controls(&good_state, &degenerate));
        }
    }

    fn corner_state_and_tracker() -> (VehicleState, RaceTracker) {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -10.0], [10.0, 0.0, -10.0]],
            false,
            position,
        );
        let state = vehicle_state(position, Quat::IDENTITY, Vec3::ZERO, 900.0, 1);
        (state, tracker)
    }

    #[test]
    fn steering_rate_limits_a_large_step_and_keeps_ramping() {
        let (state, tracker) = corner_state_and_tracker();
        let max_step = DEFAULT_STEERING_RATE * FIXED_DT;
        let mut driver = AiDriver::default();

        let first = driver.controls(&state, &tracker).steering;
        assert!(first.abs() > 0.0);
        assert!(
            first.abs() <= max_step + 1e-6,
            "first tick {} exceeded step {}",
            first,
            max_step
        );

        let second = driver.controls(&state, &tracker).steering;
        assert!(
            second.abs() > first.abs(),
            "expected the second tick to ramp, got {first} then {second}"
        );
        assert!(
            second.abs() <= 2.0 * max_step + 1e-6,
            "second tick {} exceeded two steps",
            second
        );

        let desired = AiDriver::default()
            .with_steering_rate(0.0)
            .controls(&state, &tracker)
            .steering;
        assert!(desired.abs() > 2.0 * max_step);
        assert_eq!(first.signum(), desired.signum());
    }

    #[test]
    fn zero_steering_rate_applies_desired_steering_immediately() {
        let (state, tracker) = corner_state_and_tracker();
        let desired = AiDriver::default()
            .with_steering_rate(0.0)
            .controls(&state, &tracker)
            .steering;

        assert!(
            desired.abs() > 0.9,
            "expected a large steering command, got {desired}"
        );
    }

    #[test]
    fn reset_clears_stored_steering() {
        let (state, tracker) = corner_state_and_tracker();
        let mut driver = AiDriver::default().with_steering_rate(20.0);
        let step = 20.0 * FIXED_DT;

        let first = driver.controls(&state, &tracker).steering;
        assert!((first.abs() - step).abs() < 1e-6, "first tick {first}");

        driver.reset();
        let after_reset = driver.controls(&state, &tracker).steering;
        assert!(
            (after_reset.abs() - first.abs()).abs() < 1e-6,
            "reset should restart the ramp: {first} then {after_reset}"
        );
    }

    fn straight_speed_state(speed: f32) -> (VehicleState, RaceTracker) {
        let position = Vec3::new(0.0, 0.0, -5.0);
        let tracker = straight_tracker(position);
        let state = vehicle_state(
            position,
            Quat::IDENTITY,
            Vec3::new(0.0, 0.0, -speed),
            3_000.0,
            3,
        );
        (state, tracker)
    }

    #[test]
    fn steady_target_does_not_chatter_between_throttle_and_brake() {
        let target = MAX_TARGET_SPEED * AiPace::Club.target_speed_scale();
        let (state, tracker) = straight_speed_state(target + BRAKE_ENTER_MARGIN + 0.1);
        let mut driver = AiDriver::default();
        let mut transitions = 0;
        let mut previous = driver.mode;

        for _ in 0..200 {
            let controls = driver.controls(&state, &tracker);
            if driver.mode != previous {
                transitions += 1;
                previous = driver.mode;
            }
            assert!(
                controls.throttle <= CONTROL_OVERLAP_LIMIT
                    || controls.brake <= CONTROL_OVERLAP_LIMIT,
                "throttle {} and brake {} applied together",
                controls.throttle,
                controls.brake
            );
            assert_finite(&controls);
        }

        assert_eq!(driver.mode, DriveMode::Brake);
        assert!(transitions <= 2, "mode chattered {transitions} times");
    }

    /// Runs the AI controller against the real vehicle on a long straight.
    ///
    /// The old controller cycled throttle and brake around the target speed,
    /// producing a visible forward/back surge even with a single AI. This test
    /// drives a genuine [`Vehicle`] at 500 Hz and measures the settled ripple,
    /// so it fails on a limit cycle instead of only checking fixed states.
    #[test]
    fn closed_loop_vehicle_settles_without_speed_ripple_or_chatter() {
        // Long straight along -Z, matching the identity spawn facing.
        let mut tracker = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -1000.0]],
            false,
            Vec3::ZERO,
        );
        let mut vehicle = Vehicle::new(vehicle_physics(), vehicle_collider(), &vehicle_spawn());
        let mut driver = AiDriver::default();

        // The proportional throttle has a long tail, so give it 14 s to
        // accelerate through the gears and settle, then measure the last 6 s.
        const TOTAL_TICKS: usize = 10_000;
        const WARMUP_TICKS: usize = 7_000;
        const SETTLED_SPEED_FLOOR: f32 = 20.0;
        const MAX_RIPPLE: f32 = 0.5;
        const MAX_MODE_TRANSITIONS: u32 = 2;

        let mut min_speed = f32::INFINITY;
        let mut max_speed = f32::NEG_INFINITY;
        let mut max_brake = 0.0f32;
        let mut mode_transitions = 0u32;
        let mut previous_mode = driver.mode;

        for tick in 0..TOTAL_TICKS {
            tracker.update(vehicle.state.position, tick as u64);
            let controls = driver.controls(&vehicle.state, &tracker);
            let speed = Vec3::new(vehicle.state.velocity.x, 0.0, vehicle.state.velocity.z).length();

            if tick >= WARMUP_TICKS {
                min_speed = min_speed.min(speed);
                max_speed = max_speed.max(speed);
                max_brake = max_brake.max(controls.brake);
                if driver.mode != previous_mode {
                    mode_transitions += 1;
                }
            }

            previous_mode = driver.mode;
            vehicle.step(controls);
        }

        let ripple = max_speed - min_speed;
        assert!(
            max_speed > SETTLED_SPEED_FLOOR,
            "closed-loop AI never accelerated: max speed {max_speed:.2} m/s"
        );
        assert!(
            ripple <= MAX_RIPPLE,
            "closed-loop AI speed ripple {ripple:.3} m/s (min {min_speed:.2}, max {max_speed:.2})"
        );
        assert!(
            mode_transitions <= MAX_MODE_TRANSITIONS,
            "closed-loop AI chattered between throttle and brake {mode_transitions} times"
        );
        assert!(
            max_brake <= CONTROL_OVERLAP_LIMIT,
            "closed-loop AI braked at cruise: max brake {max_brake:.3}"
        );
    }

    /// A small overspeed over the target must coast instead of braking.
    ///
    /// The old `+0.8 m/s` brake-enter margin braked inside this band and could
    /// start a throttle/brake cycle. The widened margin keeps the car in Drive
    /// so it coasts back toward the target.
    #[test]
    fn closed_loop_small_overspeed_coasts_instead_of_braking() {
        let mut tracker = tracker(
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, -1000.0]],
            false,
            Vec3::ZERO,
        );
        let mut vehicle = Vehicle::new(vehicle_physics(), vehicle_collider(), &vehicle_spawn());
        let mut driver = AiDriver::default();
        let target = MAX_TARGET_SPEED * AiPace::Club.target_speed_scale();

        // Accelerate from rest so the tyres, engine, and gear are consistent
        // before the overspeed nudge.
        for tick in 0..3_000u64 {
            tracker.update(vehicle.state.position, tick);
            let controls = driver.controls(&vehicle.state, &tracker);
            vehicle.step(controls);
        }

        // Emulate a small crest: 1 m/s above the target, inside the old brake
        // entry margin but comfortably inside the new coast band.
        let velocity = Vec3::new(vehicle.state.velocity.x, 0.0, vehicle.state.velocity.z);
        let speed = velocity.length();
        let overspeed = target + 1.0;
        let scale = overspeed / speed.max(1e-3);
        vehicle.state.velocity.x *= scale;
        vehicle.state.velocity.z *= scale;
        let wheel_delta = (overspeed - speed) / 0.33;
        for wheel in &mut vehicle.state.wheels {
            wheel.angular_velocity += wheel_delta;
        }

        let mut max_brake = 0.0f32;
        let mut mode_transitions = 0u32;
        let mut previous_mode = driver.mode;
        for tick in 3_001..4_500u64 {
            tracker.update(vehicle.state.position, tick);
            let controls = driver.controls(&vehicle.state, &tracker);
            max_brake = max_brake.max(controls.brake);
            if driver.mode != previous_mode {
                mode_transitions += 1;
            }
            previous_mode = driver.mode;
            vehicle.step(controls);
        }

        assert!(
            max_brake <= CONTROL_OVERLAP_LIMIT,
            "a {:.1} m/s overspeed braked at {:.3}",
            overspeed - target,
            max_brake
        );
        assert_eq!(
            mode_transitions, 0,
            "a small overspeed changed the drive mode"
        );
    }

    #[test]
    fn throttle_and_brake_never_overlap_or_go_non_finite() {
        let target = MAX_TARGET_SPEED * AiPace::Club.target_speed_scale();
        let speeds = [
            0.0,
            target - 3.0,
            target - 0.5,
            target,
            target + 0.5,
            target + BRAKE_ENTER_MARGIN + 0.1,
            target + 3.0,
            target - 1.0,
        ];
        let mut driver = AiDriver::default();

        for _ in 0..8 {
            for speed in speeds {
                let (state, tracker) = straight_speed_state(speed);
                let controls = driver.controls(&state, &tracker);
                assert!(
                    controls.throttle <= CONTROL_OVERLAP_LIMIT
                        || controls.brake <= CONTROL_OVERLAP_LIMIT,
                    "speed {speed}: throttle {} and brake {} applied together",
                    controls.throttle,
                    controls.brake
                );
                assert_finite(&controls);
            }
        }
    }

    #[test]
    fn controls_are_rate_limited_after_the_first_call() {
        let target = MAX_TARGET_SPEED * AiPace::Club.target_speed_scale();
        let max_throttle_step = THROTTLE_RATE * FIXED_DT;
        let max_brake_step = BRAKE_RATE * FIXED_DT;

        let mut driver = AiDriver::default();
        let (state, tracker) = straight_speed_state(0.0);
        let first = driver.controls(&state, &tracker);
        assert!(
            (first.throttle - 1.0).abs() < 1e-6,
            "first throttle {} was not applied directly",
            first.throttle
        );

        let (state, tracker) = straight_speed_state(target);
        let second = driver.controls(&state, &tracker);
        assert!(second.throttle < first.throttle);
        assert!(
            (first.throttle - second.throttle) <= max_throttle_step + 1e-6,
            "throttle step {} exceeded {}",
            first.throttle - second.throttle,
            max_throttle_step
        );

        let mut driver = AiDriver::default();
        let (state, tracker) = straight_speed_state(target + 6.0);
        let first = driver.controls(&state, &tracker);
        assert!(
            first.brake > 0.9,
            "first brake {} was not direct",
            first.brake
        );

        let (state, tracker) = straight_speed_state(target + 3.0);
        let second = driver.controls(&state, &tracker);
        assert!(second.brake < first.brake);
        assert!(
            (first.brake - second.brake) <= max_brake_step + 1e-6,
            "brake step {} exceeded {}",
            first.brake - second.brake,
            max_brake_step
        );
    }

    #[test]
    fn reset_clears_drive_state_and_restarts_direct_controls() {
        let target = MAX_TARGET_SPEED * AiPace::Club.target_speed_scale();
        let mut driver = AiDriver::default();
        let (state, tracker) = straight_speed_state(target + 6.0);
        let braking = driver.controls(&state, &tracker);
        assert_eq!(driver.mode, DriveMode::Brake);
        assert!(braking.brake > 0.9);
        assert!(driver.smoothed_target.is_some());

        driver.reset();
        assert_eq!(driver.mode, DriveMode::Drive);
        assert_eq!(driver.last_throttle, 0.0);
        assert_eq!(driver.last_brake, 0.0);
        assert!(driver.smoothed_target.is_none());

        let (state, tracker) = straight_speed_state(0.0);
        let launched = driver.controls(&state, &tracker);
        assert!(
            (launched.throttle - 1.0).abs() < 1e-6,
            "launch throttle {} was not applied directly",
            launched.throttle
        );
        assert!(launched.brake < 0.01);
    }

    #[test]
    fn steering_rate_sanitizes_and_clamps_inputs() {
        let mut driver = AiDriver::default();
        assert_eq!(driver.steering_rate, DEFAULT_STEERING_RATE);

        driver.set_steering_rate(f32::NAN);
        assert_eq!(driver.steering_rate, DEFAULT_STEERING_RATE);
        driver.set_steering_rate(f32::INFINITY);
        assert_eq!(driver.steering_rate, DEFAULT_STEERING_RATE);
        driver.set_steering_rate(-5.0);
        assert_eq!(driver.steering_rate, 0.0);
        driver.set_steering_rate(100.0);
        assert_eq!(driver.steering_rate, MAX_STEERING_RATE);

        let driver = AiDriver::default().with_steering_rate(7.5);
        assert_eq!(driver.steering_rate, 7.5);
    }

    fn assert_finite(controls: &Controls) {
        assert!(controls.steering.is_finite());
        assert!(controls.throttle.is_finite());
        assert!(controls.brake.is_finite());
        assert!(controls.clutch.is_finite());
        assert!(controls.handbrake.is_finite());
    }
}
