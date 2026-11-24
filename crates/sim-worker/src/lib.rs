use glam::{Quat, Vec3};
use sim_core::{
    resolve_car_contact_with_slots, AiDriver, AiPace, Controls, PhysicsTuning, RaceTracker,
    TrackCollider, Vehicle, FIXED_DT,
};
use sim_types::snapshot_state as state;
use sim_types::{CarTuning, RaceData, SpawnPoint, SNAPSHOT_STATE_FLOATS};
use wasm_bindgen::prelude::*;

const MAX_AI: u32 = 3;
/// At 500 Hz, one contact event per unordered pair per 100 ms.
const CONTACT_COOLDOWN_TICKS: u64 = 50;

#[derive(Debug, Clone, Copy)]
struct ContactEvent {
    a: u32,
    b: u32,
    point: [f32; 3],
    normal: [f32; 3],
    speed: f32,
    tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RacePhase {
    Free,
    Held,
    Running,
}

struct AiCar {
    vehicle: Vehicle,
    tracker: RaceTracker,
    driver: AiDriver,
    last_throttle: f32,
    last_brake: f32,
}

impl AiCar {
    fn new(
        physics: sim_types::CarPhysics,
        collider: TrackCollider,
        race: &RaceData,
        index: usize,
        driver: AiDriver,
    ) -> Option<Self> {
        let grid_spawn = race.grid.get(index + 1)?;
        let tracker = RaceTracker::new(race);
        let spawn = oriented_spawn(&tracker, grid_spawn);
        let vehicle = Vehicle::new(physics, collider, &spawn);
        let mut car = Self {
            vehicle,
            tracker,
            driver,
            last_throttle: 0.0,
            last_brake: 0.0,
        };
        car.sync_tracker();
        Some(car)
    }

    fn place_on_grid(&mut self, base: &SpawnPoint) {
        let spawn = oriented_spawn(&self.tracker, base);
        self.vehicle.set_spawn(&spawn);
        self.driver.reset();
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        self.tracker.reset();
        self.sync_tracker();
    }

    fn arm_grid_start(&mut self) {
        self.tracker.arm_grid_start();
        self.sync_tracker();
    }

    fn step(&mut self, phase: RacePhase, race_laps: u32) {
        self.sync_tracker();
        let controls = if phase == RacePhase::Held {
            Controls::default()
        } else {
            let driving = self.driver.controls(&self.vehicle.state, &self.tracker);
            race_controls(phase, self.tracker.state().lap, race_laps, driving)
        };
        self.last_throttle = controls.throttle;
        self.last_brake = controls.brake;
        self.vehicle.step(controls);
    }

    fn settle_step(&mut self) {
        self.vehicle.step(Controls::default());
        self.sync_tracker();
    }

    fn sync_tracker(&mut self) {
        self.tracker
            .update(self.vehicle.state.position, self.vehicle.state.tick);
    }

    fn reset(&mut self) {
        self.vehicle.reset();
        self.tracker.reset();
        self.driver.reset();
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        self.sync_tracker();
    }
}

#[wasm_bindgen]
pub struct Simulation {
    vehicle: Vehicle,
    last_throttle: f32,
    last_brake: f32,
    ai_cars: Vec<AiCar>,
    race: Option<RaceData>,
    race_phase: RacePhase,
    race_laps: u32,
    contacts: Vec<ContactEvent>,
    contact_cooldowns: Vec<(u32, u32, u64)>,
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new(
        track_pack_bytes: &[u8],
        car_pack_bytes: &[u8],
        ai_count: u32,
        ai_pace: u32,
    ) -> Result<Simulation, JsValue> {
        let track_bytes = track_pack_bytes.to_vec();
        let track_pack = sim_asset::Pack::from_bytes(&track_bytes).map_err(js_error)?;
        if track_pack.kind() != sim_types::AssetKind::Track {
            return Err(JsValue::from_str("expected a track pack"));
        }
        let track_collision = track_pack.track_collision_owned().map_err(js_error)?;
        let track_metadata = track_pack.track_metadata_owned().map_err(js_error)?;
        let race = track_pack.track_race_owned().map_err(js_error)?;
        drop(track_pack);

        let car_bytes = car_pack_bytes.to_vec();
        let car_pack = sim_asset::Pack::from_bytes(&car_bytes).map_err(js_error)?;
        if car_pack.kind() != sim_types::AssetKind::Car {
            return Err(JsValue::from_str("expected a car pack"));
        }
        let car_physics = car_pack.car_physics_owned().map_err(js_error)?;
        drop(car_pack);

        let mut spawn = track_metadata
            .spawns
            .first()
            .cloned()
            .ok_or_else(|| JsValue::from_str("track has no spawn point"))?;
        let spawn_rotation = spawn_rotation(&spawn);
        spawn.rotation_xyzw =
            (Quat::from_rotation_y(std::f32::consts::PI) * spawn_rotation).to_array();
        let collider = TrackCollider::from_collision(&track_collision, &track_metadata);
        let vehicle = Vehicle::new(car_physics.clone(), collider.clone(), &spawn);

        let redline = car_physics.engine.redline_rpm;
        let idle = car_physics.engine.idle_rpm;
        let max_gear = car_physics.drivetrain.gear_ratios.len() as i32;
        let upshift_rpm = idle + (redline - idle) * 0.85;
        let downshift_rpm = idle + (redline - idle) * 0.3;

        let requested = ai_count.min(MAX_AI);
        let available = race
            .as_ref()
            .map(|race| race.grid.len().saturating_sub(1) as u32)
            .unwrap_or(0);
        let ai_total = requested.min(available);
        let pace = AiPace::from_index(ai_pace);

        let mut ai_cars = Vec::with_capacity(ai_total as usize);
        if let Some(race) = race.as_ref() {
            for index in 0..ai_total as usize {
                let driver = AiDriver::new(upshift_rpm, downshift_rpm, max_gear, pace);
                if let Some(car) =
                    AiCar::new(car_physics.clone(), collider.clone(), race, index, driver)
                {
                    ai_cars.push(car);
                } else {
                    break;
                }
            }
        }

        let mut simulation = Simulation {
            vehicle,
            last_throttle: 0.0,
            last_brake: 0.0,
            ai_cars,
            race,
            race_phase: RacePhase::Free,
            race_laps: 0,
            contacts: Vec::new(),
            contact_cooldowns: Vec::new(),
        };
        simulation.settle();
        Ok(simulation)
    }

    pub fn step(
        &mut self,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
        clutch: f32,
        handbrake: f32,
    ) {
        let controls = if self.race_phase == RacePhase::Held {
            Controls::default()
        } else {
            Controls {
                steering,
                throttle,
                brake,
                clutch,
                gear,
                handbrake,
            }
        };
        self.last_throttle = controls.throttle;
        self.last_brake = controls.brake;
        self.vehicle.step(controls);
        let phase = self.race_phase;
        let race_laps = self.race_laps;
        for ai in &mut self.ai_cars {
            ai.step(phase, race_laps);
        }
        self.resolve_contacts();
    }

    pub fn simulate(
        &mut self,
        seconds: f32,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
    ) -> f64 {
        let steps = ((seconds.clamp(0.0, 120.0) / FIXED_DT) as u32).min(60_000);
        let controls = if self.race_phase == RacePhase::Held {
            Controls::default()
        } else {
            Controls {
                steering,
                throttle,
                brake,
                clutch: 0.0,
                gear,
                handbrake: 0.0,
            }
        };
        self.last_throttle = controls.throttle;
        self.last_brake = controls.brake;
        let phase = self.race_phase;
        let race_laps = self.race_laps;
        for _ in 0..steps {
            self.vehicle.step(controls);
            for ai in &mut self.ai_cars {
                ai.step(phase, race_laps);
            }
            self.resolve_contacts();
        }
        self.vehicle.state.tick as f64
    }

    pub fn reset(&mut self) {
        self.vehicle.reset();
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        for ai in &mut self.ai_cars {
            ai.reset();
        }
        self.settle();
        self.clear_contacts();
    }

    pub fn place_on_grid(&mut self, slot: usize) {
        self.vehicle.place_on_grid(slot);
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        for ai in &mut self.ai_cars {
            ai.reset();
        }
        self.settle();
        self.clear_contacts();
    }

    /// Places the player and AI on the race grid and holds them before the
    /// start.
    ///
    /// The player uses `race.grid[slot]`; AI fill the lowest grid slots in
    /// order, skipping the player's slot. Both headings come from the nearest
    /// layout tangent until track-ed grid rotations are fixed (TD-0021). The
    /// lateral offset is clamped toward the centreline by
    /// [`RaceTracker::grid_spawn`]. Grid starts are armed after settling, so
    /// the first forward start/finish crossing begins lap 1 without
    /// incrementing.
    pub fn place_on_race_grid(&mut self, slot: u32) {
        let Some(race) = self.race.clone() else {
            return;
        };
        let player_slot = slot as usize;
        let Some(player_grid) = race.grid.get(player_slot).cloned() else {
            return;
        };
        let layout_tracker = RaceTracker::new(&race);
        let player_spawn = oriented_spawn(&layout_tracker, &player_grid);
        self.vehicle.set_spawn(&player_spawn);
        self.last_throttle = 0.0;
        self.last_brake = 0.0;

        let mut next_grid = 0usize;
        for ai in &mut self.ai_cars {
            while next_grid == player_slot {
                next_grid += 1;
            }
            if let Some(grid) = race.grid.get(next_grid) {
                ai.place_on_grid(grid);
            } else {
                ai.reset();
            }
            next_grid += 1;
        }

        self.race_phase = RacePhase::Held;
        self.race_laps = 0;
        self.settle();
        for ai in &mut self.ai_cars {
            ai.arm_grid_start();
        }
        self.clear_contacts();
    }

    /// Starts the race and stores the lap limit.
    pub fn start_race(&mut self, laps: u32) {
        self.race_laps = laps.max(1);
        self.race_phase = RacePhase::Running;
        self.clear_contacts();
    }

    /// Drains pending car-to-car contact events as a JSON array.
    ///
    /// The array is empty (`[]`) when no contact occurred since the last drain.
    pub fn drain_contacts(&mut self) -> String {
        let contacts = std::mem::take(&mut self.contacts);
        if contacts.is_empty() {
            return String::from("[]");
        }
        let events = contacts
            .iter()
            .map(|contact| {
                serde_json::json!({
                    "a": contact.a,
                    "b": contact.b,
                    "point": contact.point,
                    "normal": contact.normal,
                    "speed": contact.speed,
                    "tick": contact.tick,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&events).unwrap_or_else(|_| String::from("[]"))
    }

    /// Returns one AI car's [`sim_types::RaceState`] as JSON.
    ///
    /// An invalid index returns `{"available":false}`. Otherwise the object
    /// includes `available: true` and every `RaceState` field.
    pub fn ai_race_state(&self, index: u32) -> String {
        race_state_json(self.ai_cars.get(index as usize).map(|ai| &ai.tracker))
    }

    pub fn ai_count(&self) -> u32 {
        self.ai_cars.len() as u32
    }

    pub fn ai_snapshot(&self, index: u32) -> Vec<f32> {
        match self.ai_cars.get(index as usize) {
            Some(ai) => snapshot_for(&ai.vehicle, ai.last_throttle, ai.last_brake),
            None => vec![0.0f32; SNAPSHOT_STATE_FLOATS],
        }
    }

    fn clear_contacts(&mut self) {
        self.contacts.clear();
        self.contact_cooldowns.clear();
    }

    /// Resolves car-to-car contact for the player and every AI in slot order.
    ///
    /// Player slot `0` is paired with each AI slot `index + 1`, then AI pairs
    /// are resolved in ascending order. The loop is `O(n^2)` and does not
    /// hard-code the AI count. Events are rate limited per unordered pair.
    fn resolve_contacts(&mut self) {
        if self.ai_cars.is_empty() {
            return;
        }
        let mut found = Vec::new();
        {
            let (player, ai_cars) = (&mut self.vehicle, &mut self.ai_cars);
            for (index, ai) in ai_cars.iter_mut().enumerate() {
                if let Some(contact) =
                    resolve_car_contact_with_slots(player, &mut ai.vehicle, 0, index as u32 + 1)
                {
                    found.push(contact);
                }
            }
            for first in 0..ai_cars.len() {
                for second in (first + 1)..ai_cars.len() {
                    let (left, right) = ai_cars.split_at_mut(second);
                    if let Some(contact) = resolve_car_contact_with_slots(
                        &mut left[first].vehicle,
                        &mut right[0].vehicle,
                        first as u32 + 1,
                        second as u32 + 1,
                    ) {
                        found.push(contact);
                    }
                }
            }
        }

        for contact in found {
            let a = contact.a.min(contact.b);
            let b = contact.a.max(contact.b);
            let tick = contact.tick;
            let last = self
                .contact_cooldowns
                .iter()
                .find(|(ca, cb, _)| *ca == a && *cb == b)
                .map(|(_, _, last)| *last);
            if let Some(last) = last {
                if tick.saturating_sub(last) < CONTACT_COOLDOWN_TICKS {
                    continue;
                }
            }
            self.contacts.push(ContactEvent {
                a,
                b,
                point: contact.point.to_array(),
                normal: contact.normal.to_array(),
                speed: contact.relative_speed,
                tick,
            });
            if let Some(entry) = self
                .contact_cooldowns
                .iter_mut()
                .find(|(ca, cb, _)| *ca == a && *cb == b)
            {
                entry.2 = tick;
            } else {
                self.contact_cooldowns.push((a, b, tick));
            }
        }
    }

    fn settle(&mut self) {
        for _ in 0..500 {
            self.vehicle.step(Controls::default());
            for ai in &mut self.ai_cars {
                ai.settle_step();
            }
        }
    }

    pub fn tick(&self) -> f64 {
        self.vehicle.state.tick as f64
    }

    pub fn setup(&self) -> String {
        serde_json::to_string(&self.vehicle.setup()).unwrap_or_default()
    }

    pub fn default_setup(&self) -> String {
        serde_json::to_string(&self.vehicle.default_setup()).unwrap_or_default()
    }

    pub fn apply_physics_tuning(&mut self, json: &str) -> Result<f64, JsValue> {
        let tuning: PhysicsTuning = serde_json::from_str(json)
            .map_err(|error| JsValue::from_str(&format!("invalid physics tuning: {error}")))?;
        self.vehicle.set_physics_tuning(&tuning);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_physics_tuning(&tuning);
        }
        self.reset();
        Ok(self.tick())
    }

    pub fn apply_setup(&mut self, json: &str) -> Result<String, JsValue> {
        let tuning: CarTuning = serde_json::from_str(json)
            .map_err(|error| JsValue::from_str(&format!("invalid setup: {error}")))?;
        self.vehicle.apply_setup(&tuning);
        for ai in &mut self.ai_cars {
            ai.vehicle.apply_setup(&tuning);
        }
        Ok(self.setup())
    }

    pub fn reset_setup(&mut self) -> String {
        self.vehicle.reset_setup();
        for ai in &mut self.ai_cars {
            ai.vehicle.reset_setup();
        }
        self.setup()
    }

    pub fn set_traction_control(&mut self, enabled: bool) {
        self.vehicle.set_traction_control(enabled);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_traction_control(enabled);
        }
    }

    pub fn set_traction_strength(&mut self, strength: f32) {
        self.vehicle.set_traction_strength(strength);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_traction_strength(strength);
        }
    }

    pub fn set_abs_control(&mut self, enabled: bool) {
        self.vehicle.set_abs_control(enabled);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_abs_control(enabled);
        }
    }

    pub fn set_stability_control(&mut self, enabled: bool) {
        self.vehicle.set_stability_control(enabled);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_stability_control(enabled);
        }
    }

    pub fn set_stability_strength(&mut self, strength: f32) {
        self.vehicle.set_stability_strength(strength);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_stability_strength(strength);
        }
    }

    pub fn set_steering_sensitivity(&mut self, sensitivity: f32) {
        self.vehicle.set_steering_sensitivity(sensitivity);
        for ai in &mut self.ai_cars {
            ai.vehicle.set_steering_sensitivity(sensitivity);
        }
    }

    pub fn snapshot(&self) -> Vec<f32> {
        snapshot_for(&self.vehicle, self.last_throttle, self.last_brake)
    }
}

fn race_controls(phase: RacePhase, lap: u32, race_laps: u32, driving: Controls) -> Controls {
    match phase {
        RacePhase::Free => driving,
        RacePhase::Held => Controls::default(),
        RacePhase::Running if lap > race_laps => Controls {
            brake: 1.0,
            gear: driving.gear.max(1),
            ..Controls::default()
        },
        RacePhase::Running => driving,
    }
}

fn oriented_spawn(tracker: &RaceTracker, base: &SpawnPoint) -> SpawnPoint {
    let mut spawn = base.clone();
    match tracker.grid_spawn(base) {
        Some((position, tangent)) => {
            spawn.position = position.to_array();
            let yaw = (-tangent.x).atan2(-tangent.z);
            spawn.rotation_xyzw = Quat::from_rotation_y(yaw).to_array();
        }
        None => {
            let rotation = spawn_rotation(&spawn);
            spawn.rotation_xyzw =
                (Quat::from_rotation_y(std::f32::consts::PI) * rotation).to_array();
        }
    }
    spawn
}

fn race_state_json(tracker: Option<&RaceTracker>) -> String {
    match tracker {
        None => serde_json::json!({ "available": false }).to_string(),
        Some(tracker) => {
            let state = tracker.state();
            serde_json::json!({
                "available": true,
                "lap": state.lap,
                "sector": state.sector,
                "progress": state.progress,
                "distance_m": state.distance_m,
                "wrong_way": state.wrong_way,
                "started": state.started,
                "current_lap_time": state.current_lap_time,
                "last_lap_time": state.last_lap_time,
                "best_lap_time": state.best_lap_time,
            })
            .to_string()
        }
    }
}

fn spawn_rotation(spawn: &SpawnPoint) -> Quat {
    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    );
    if rotation.length_squared() > 1e-8 {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    }
}

fn snapshot_for(vehicle: &Vehicle, last_throttle: f32, last_brake: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; SNAPSHOT_STATE_FLOATS];
    let state_data = &vehicle.state;
    out[state::POSITION..state::POSITION + 3].copy_from_slice(&state_data.position.to_array());
    out[state::ROTATION..state::ROTATION + 4].copy_from_slice(&[
        state_data.rotation.x,
        state_data.rotation.y,
        state_data.rotation.z,
        state_data.rotation.w,
    ]);
    out[state::VELOCITY..state::VELOCITY + 3].copy_from_slice(&state_data.velocity.to_array());
    out[state::ANGULAR_VELOCITY..state::ANGULAR_VELOCITY + 3]
        .copy_from_slice(&state_data.angular_velocity.to_array());

    let mut contact = Vec3::ZERO;
    let mut normal = Vec3::ZERO;
    let mut contact_count = 0.0f32;
    for (index, wheel) in state_data.wheels.iter().enumerate() {
        out[state::WHEEL_ANGULAR_SPEEDS + index] = wheel.angular_velocity;
        out[state::SUSPENSION_TRAVEL + index] = wheel.suspension_compression;
        out[state::SLIP_RATIO + index] = wheel.slip_ratio;
        out[state::SLIP_ANGLE + index] = wheel.slip_angle;
        out[state::TIRE_LOAD + index] = wheel.normal_load;
        out[state::CONTACT_FLAGS + index] = if wheel.contact { 1.0 } else { 0.0 };
        out[state::SURFACE_FRICTION + index] = wheel.surface_friction;
        out[state::WHEEL_STEER_ANGLE + index] = wheel.steer_angle;
        out[state::WHEEL_SPIN_ANGLE + index] = wheel.spin_angle;
        if wheel.contact {
            contact += wheel.contact_point;
            normal += wheel.contact_normal;
            contact_count += 1.0;
        }
    }
    if contact_count > 0.0 {
        contact /= contact_count;
        normal = normal.normalize_or(Vec3::Y);
    } else {
        contact = state_data.position;
        normal = Vec3::Y;
    }
    out[state::CONTACT_CENTER..state::CONTACT_CENTER + 3].copy_from_slice(&contact.to_array());
    out[state::CONTACT_NORMAL..state::CONTACT_NORMAL + 3].copy_from_slice(&normal.to_array());
    out[state::ENGINE_RPM] = state_data.engine_rpm;
    out[state::RACK_TORQUE] = state_data.steering_rack_torque;
    out[state::THROTTLE] = last_throttle;
    out[state::BRAKE] = last_brake;
    out[state::GEAR] = state_data.gear as f32;
    out[state::ABS_ACTIVE] = if vehicle.abs_active() { 1.0 } else { 0.0 };
    out[state::ESC_ACTIVITY] = vehicle.esc_activity();
    out[state::TICK] = state_data.tick as f32;
    let mut surface_ids = 0u32;
    for (index, wheel) in state_data.wheels.iter().enumerate() {
        surface_ids |= ((wheel.surface_id as u32) & 0x3f) << (index * 6);
    }
    out[state::SURFACE_IDS] = surface_ids as f32;
    out
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_types::{
        Aabb, AeroSpec, BrakeSpec, DriveLayout, DrivetrainSpec, EngineSpec, Layout,
        LayoutDirection, PacejkaCoefficients, SectorSpec, StartFinish, SteeringSpec, SurfaceSpec,
        SuspensionSpec, TireSpec, TorquePoint, TrackCollision, TrackMetadata, WheelPhysics,
        RACE_DATA_VERSION,
    };

    fn physics() -> sim_types::CarPhysics {
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
        sim_types::CarPhysics {
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

    fn grid(name: &str, position: [f32; 3]) -> SpawnPoint {
        SpawnPoint {
            name: String::from(name),
            position,
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn race() -> RaceData {
        RaceData {
            version: RACE_DATA_VERSION,
            layout: Layout {
                layout_id: String::from("test"),
                closed: true,
                direction: LayoutDirection::Forward,
                points: vec![
                    [0.0, 0.0, 0.0],
                    [10.0, 0.0, 0.0],
                    [10.0, 0.0, 10.0],
                    [0.0, 0.0, 10.0],
                ],
                half_width: 5.0,
            },
            start_finish: StartFinish {
                section_id: String::from("test"),
                fraction: 0.0,
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            grid: vec![
                grid("grid_0", [0.0, 0.0, 2.0]),
                grid("grid_1", [1.0, 0.0, 3.0]),
                grid("grid_2", [2.0, 0.0, 4.0]),
            ],
            sectors: vec![
                SectorSpec {
                    name: String::from("sector_1"),
                    start_fraction: 0.0,
                    end_fraction: 0.5,
                },
                SectorSpec {
                    name: String::from("sector_2"),
                    start_fraction: 0.5,
                    end_fraction: 1.0,
                },
            ],
        }
    }

    fn grid_race() -> RaceData {
        let mut race = race();
        race.layout.points = vec![[0.0, 0.0, 0.0], [200.0, 0.0, 0.0]];
        race.layout.closed = false;
        race.layout.half_width = 10.0;
        race.grid = vec![
            grid("grid_0", [10.0, 0.0, 3.0]),
            grid("grid_1", [18.0, 0.0, 2.0]),
            grid("grid_2", [26.0, 0.0, 1.0]),
            grid("grid_3", [34.0, 0.0, 0.0]),
        ];
        race
    }

    fn square_point(progress: f32) -> Vec3 {
        let distance = progress * 40.0;
        if distance <= 10.0 {
            Vec3::new(distance, 0.0, 0.0)
        } else if distance <= 20.0 {
            Vec3::new(10.0, 0.0, distance - 10.0)
        } else if distance <= 30.0 {
            Vec3::new(10.0 - (distance - 20.0), 0.0, 10.0)
        } else {
            Vec3::new(0.0, 0.0, 10.0 - (distance - 30.0))
        }
    }

    fn simulation() -> Simulation {
        simulation_with_ai(1)
    }

    fn simulation_with_ai(ai_count: usize) -> Simulation {
        let mut race = race();
        while race.grid.len() < ai_count + 1 {
            let index = race.grid.len();
            race.grid.push(grid(
                &format!("grid_{index}"),
                [index as f32, 0.0, 2.0 + index as f32],
            ));
        }
        simulation_from_race(race, ai_count)
    }

    fn simulation_from_race(race: RaceData, ai_count: usize) -> Simulation {
        let physics = physics();
        let collider = collider();
        let vehicle = Vehicle::new(physics.clone(), collider.clone(), &race.grid[0]);
        let mut ai_cars = Vec::new();
        for index in 0..ai_count {
            let driver = AiDriver::default();
            if let Some(ai) = AiCar::new(physics.clone(), collider.clone(), &race, index, driver) {
                ai_cars.push(ai);
            }
        }
        Simulation {
            vehicle,
            last_throttle: 0.0,
            last_brake: 0.0,
            ai_cars,
            race: Some(race),
            race_phase: RacePhase::Free,
            race_laps: 0,
            contacts: Vec::new(),
            contact_cooldowns: Vec::new(),
        }
    }

    fn place_pairs(simulation: &mut Simulation, pairs: &[(usize, Vec3)]) {
        for (slot, position) in pairs {
            if *slot == 0 {
                simulation.vehicle.state.position = *position;
                simulation.vehicle.state.velocity = Vec3::ZERO;
                simulation.vehicle.state.angular_velocity = Vec3::ZERO;
            } else if let Some(ai) = simulation.ai_cars.get_mut(slot - 1) {
                ai.vehicle.state.position = *position;
                ai.vehicle.state.velocity = Vec3::ZERO;
                ai.vehicle.state.angular_velocity = Vec3::ZERO;
            }
        }
    }

    fn contact_events(simulation: &mut Simulation) -> Vec<serde_json::Value> {
        serde_json::from_str(&simulation.drain_contacts()).expect("contact JSON")
    }

    fn force_player_ai_contact(simulation: &mut Simulation) {
        place_pairs(
            simulation,
            &[
                (0, Vec3::new(0.0, 0.25, 0.0)),
                (1, Vec3::new(0.0, 0.25, 3.4)),
            ],
        );
        simulation.vehicle.state.tick += 1;
        if let Some(ai) = simulation.ai_cars.first_mut() {
            ai.vehicle.state.tick = simulation.vehicle.state.tick;
        }
        simulation.resolve_contacts();
    }

    #[test]
    fn ai_race_state_serializes_all_contract_fields() {
        let simulation = simulation();
        let value: serde_json::Value =
            serde_json::from_str(&simulation.ai_race_state(0)).expect("parse ai race state");

        assert_eq!(value["available"], serde_json::json!(true));
        let mut keys = value
            .as_object()
            .expect("race state object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "available",
                "best_lap_time",
                "current_lap_time",
                "distance_m",
                "lap",
                "last_lap_time",
                "progress",
                "sector",
                "started",
                "wrong_way"
            ]
        );
    }

    #[test]
    fn ai_race_state_invalid_index_is_unavailable() {
        let simulation = simulation();
        let value: serde_json::Value =
            serde_json::from_str(&simulation.ai_race_state(7)).expect("parse ai race state");

        assert_eq!(value, serde_json::json!({ "available": false }));
    }

    #[test]
    fn place_on_race_grid_holds_player_and_ai_until_start() {
        let mut simulation = simulation();
        simulation.place_on_race_grid(0);
        assert_eq!(simulation.race_phase, RacePhase::Held);

        let player_grid = simulation.race.as_ref().expect("race").grid[0].position;
        assert!((simulation.vehicle.state.position.x - player_grid[0]).abs() < 0.2);
        assert!((simulation.vehicle.state.position.z - player_grid[2]).abs() < 0.2);

        simulation.step(1.0, 0.0, 0.0, 1, 0.0, 0.0);
        assert_eq!(simulation.last_throttle, 0.0);
        assert_eq!(simulation.ai_cars[0].last_throttle, 0.0);
        assert!(
            simulation.ai_cars[0].vehicle.state.velocity.length() < 0.1,
            "ai moved while held"
        );

        simulation.start_race(3);
        assert_eq!(simulation.race_phase, RacePhase::Running);
        assert_eq!(simulation.race_laps, 3);
        simulation.step(1.0, 0.0, 0.0, 1, 0.0, 0.0);
        assert_eq!(simulation.last_throttle, 1.0);
        assert!(simulation.ai_cars[0].last_throttle > 0.0);
    }

    fn assert_grid_x(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.5,
            "expected x {expected}, got {actual}"
        );
    }

    #[test]
    fn player_slot_three_places_player_last_and_ai_ahead() {
        let mut simulation = simulation_from_race(grid_race(), 3);
        simulation.place_on_race_grid(3);
        assert_eq!(simulation.race_phase, RacePhase::Held);

        assert_grid_x(simulation.vehicle.state.position.x, 34.0);
        for (ai, expected) in simulation.ai_cars.iter().zip([10.0, 18.0, 26.0]) {
            assert_grid_x(ai.vehicle.state.position.x, expected);
        }
    }

    #[test]
    fn player_slot_zero_keeps_ai_on_later_grids() {
        let mut simulation = simulation_from_race(grid_race(), 3);
        simulation.place_on_race_grid(0);

        assert_grid_x(simulation.vehicle.state.position.x, 10.0);
        for (ai, expected) in simulation.ai_cars.iter().zip([18.0, 26.0, 34.0]) {
            assert_grid_x(ai.vehicle.state.position.x, expected);
        }
    }

    #[test]
    fn ai_brakes_after_the_lap_limit() {
        let mut simulation = simulation();
        simulation.start_race(2);

        let ai = &mut simulation.ai_cars[0];
        ai.tracker.arm_grid_start();
        ai.tracker.update(square_point(0.95), 0);
        ai.tracker.update(square_point(0.05), 1);
        ai.tracker.update(square_point(0.50), 2);
        ai.tracker.update(square_point(0.90), 3);
        ai.tracker.update(square_point(0.05), 4);
        ai.tracker.update(square_point(0.50), 5);
        ai.tracker.update(square_point(0.90), 6);
        ai.tracker.update(square_point(0.05), 7);
        ai.vehicle.state.position = square_point(0.05);

        ai.step(RacePhase::Running, 2);

        assert_eq!(ai.tracker.state().lap, 3);
        assert_eq!(ai.last_throttle, 0.0);
        assert!(ai.last_brake > 0.9);
    }

    #[test]
    fn free_and_running_below_limit_keep_driver_controls() {
        let driving = Controls {
            throttle: 0.7,
            steering: 0.3,
            gear: 2,
            ..Controls::default()
        };
        assert_eq!(race_controls(RacePhase::Free, 10, 0, driving), driving);

        let running = race_controls(RacePhase::Running, 2, 3, driving);
        assert_eq!(running.throttle, driving.throttle);
        assert_eq!(running.steering, driving.steering);

        let held = race_controls(RacePhase::Held, 1, 3, driving);
        assert_eq!(held.throttle, 0.0);
        assert_eq!(held.brake, 0.0);
    }

    #[test]
    fn player_to_ai_contact_emits_bounded_event() {
        let mut simulation = simulation_with_ai(1);
        place_pairs(
            &mut simulation,
            &[
                (0, Vec3::new(0.0, 0.25, 0.0)),
                (1, Vec3::new(0.0, 0.25, 3.4)),
            ],
        );
        simulation.vehicle.state.velocity = Vec3::new(0.0, 0.0, 4.0);
        simulation.ai_cars[0].vehicle.state.velocity = Vec3::new(0.0, 0.0, -4.0);

        simulation.resolve_contacts();
        let events = contact_events(&mut simulation);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["a"], 0);
        assert_eq!(events[0]["b"], 1);
        assert!(events[0]["speed"].as_f64().unwrap_or(0.0) > 1.0);
        assert_eq!(events[0]["point"].as_array().map(Vec::len), Some(3));
        assert_eq!(events[0]["normal"].as_array().map(Vec::len), Some(3));
        assert_eq!(simulation.drain_contacts(), "[]");
    }

    #[test]
    fn ai_to_ai_contact_emits_bounded_event() {
        let mut simulation = simulation_with_ai(2);
        place_pairs(
            &mut simulation,
            &[
                (0, Vec3::new(100.0, 0.25, 100.0)),
                (1, Vec3::new(0.0, 0.25, 0.0)),
                (2, Vec3::new(0.0, 0.25, 3.4)),
            ],
        );
        simulation.ai_cars[0].vehicle.state.velocity = Vec3::new(0.0, 0.0, 4.0);
        simulation.ai_cars[1].vehicle.state.velocity = Vec3::new(0.0, 0.0, -4.0);

        simulation.resolve_contacts();
        let events = contact_events(&mut simulation);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["a"], 1);
        assert_eq!(events[0]["b"], 2);
        assert!(events[0]["speed"].as_f64().unwrap_or(0.0) > 1.0);
    }

    #[test]
    fn contact_events_respect_per_pair_cooldown() {
        let mut simulation = simulation_with_ai(1);
        let mut emitted = 0;
        for tick in 0..=120u64 {
            place_pairs(
                &mut simulation,
                &[
                    (0, Vec3::new(0.0, 0.25, 0.0)),
                    (1, Vec3::new(0.0, 0.25, 3.4)),
                ],
            );
            simulation.vehicle.state.tick = tick;
            simulation.ai_cars[0].vehicle.state.tick = tick;
            simulation.resolve_contacts();
            let events = contact_events(&mut simulation);
            assert!(events.len() <= 1, "more than one event in a tick");
            emitted += events.len();
        }
        assert_eq!(emitted, 3, "expected one event per 50-tick window");
    }

    #[test]
    fn ai_count_zero_emits_no_contact_events() {
        let mut simulation = simulation_with_ai(0);
        simulation.vehicle.state.position = Vec3::new(0.0, 0.25, 0.0);
        simulation.vehicle.state.velocity = Vec3::new(0.0, 0.0, 4.0);

        simulation.resolve_contacts();

        assert!(simulation.contacts.is_empty());
        assert_eq!(simulation.drain_contacts(), "[]");
    }

    #[test]
    fn lifecycle_actions_clear_contact_events_and_cooldowns() {
        let mut simulation = simulation_with_ai(1);

        force_player_ai_contact(&mut simulation);
        assert_eq!(simulation.contacts.len(), 1);
        simulation.reset();
        assert!(simulation.contacts.is_empty());
        assert!(simulation.contact_cooldowns.is_empty());
        assert_eq!(simulation.drain_contacts(), "[]");

        force_player_ai_contact(&mut simulation);
        assert_eq!(simulation.contacts.len(), 1);
        simulation.place_on_grid(0);
        assert!(simulation.contacts.is_empty());
        assert!(simulation.contact_cooldowns.is_empty());

        force_player_ai_contact(&mut simulation);
        assert_eq!(simulation.contacts.len(), 1);
        simulation.place_on_race_grid(0);
        assert!(simulation.contacts.is_empty());
        assert!(simulation.contact_cooldowns.is_empty());

        force_player_ai_contact(&mut simulation);
        assert_eq!(simulation.contacts.len(), 1);
        simulation.start_race(3);
        assert!(simulation.contacts.is_empty());
        assert!(simulation.contact_cooldowns.is_empty());
        assert_eq!(simulation.drain_contacts(), "[]");
    }

    #[test]
    fn simulate_path_emits_contact_events() {
        let mut simulation = simulation_with_ai(1);
        place_pairs(
            &mut simulation,
            &[
                (0, Vec3::new(0.0, 0.25, 0.0)),
                (1, Vec3::new(0.0, 0.25, 3.4)),
            ],
        );
        simulation.vehicle.state.velocity = Vec3::new(0.0, 0.0, 6.0);
        simulation.ai_cars[0].vehicle.state.velocity = Vec3::new(0.0, 0.0, -6.0);

        simulation.simulate(0.02, 0.0, 0.0, 0.0, 1);
        let events = contact_events(&mut simulation);

        assert!(!events.is_empty(), "simulate did not emit a contact");
        assert!(events.iter().all(|event| event["tick"].as_u64().is_some()));
    }
}
