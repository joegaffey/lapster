use glam::{Quat, Vec3};
use sim_core::{Controls, PhysicsTuning, TrackCollider, Vehicle, FIXED_DT};
use sim_types::snapshot_state as state;
use sim_types::{CarTuning, SNAPSHOT_STATE_FLOATS};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Simulation {
    vehicle: Vehicle,
    last_throttle: f32,
    last_brake: f32,
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new(track_pack_bytes: &[u8], car_pack_bytes: &[u8]) -> Result<Simulation, JsValue> {
        let track_bytes = track_pack_bytes.to_vec();
        let track_pack = sim_asset::Pack::from_bytes(&track_bytes).map_err(js_error)?;
        if track_pack.kind() != sim_types::AssetKind::Track {
            return Err(JsValue::from_str("expected a track pack"));
        }
        let track_collision = track_pack.track_collision_owned().map_err(js_error)?;
        let track_metadata = track_pack.track_metadata_owned().map_err(js_error)?;
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
        let spawn_rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let spawn_rotation = if spawn_rotation.length_squared() > 1e-8 {
            spawn_rotation.normalize()
        } else {
            Quat::IDENTITY
        };
        spawn.rotation_xyzw =
            (Quat::from_rotation_y(std::f32::consts::PI) * spawn_rotation).to_array();
        let collider = TrackCollider::from_collision(&track_collision, &track_metadata);
        let vehicle = Vehicle::new(car_physics, collider, &spawn);
        let mut simulation = Simulation {
            vehicle,
            last_throttle: 0.0,
            last_brake: 0.0,
        };
        simulation.settle();
        Ok(simulation)
    }

    pub fn step(&mut self, throttle: f32, brake: f32, steering: f32, gear: i32, clutch: f32) {
        self.last_throttle = throttle;
        self.last_brake = brake;
        self.vehicle.step(Controls {
            steering,
            throttle,
            brake,
            clutch,
            gear,
        });
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
        self.last_throttle = throttle;
        self.last_brake = brake;
        let controls = Controls {
            steering,
            throttle,
            brake,
            clutch: 0.0,
            gear,
        };
        for _ in 0..steps {
            self.vehicle.step(controls);
        }
        self.vehicle.state.tick as f64
    }

    pub fn reset(&mut self) {
        self.vehicle.reset();
        self.last_throttle = 0.0;
        self.last_brake = 0.0;
        self.settle();
    }

    fn settle(&mut self) {
        for _ in 0..500 {
            self.vehicle.step(Controls::default());
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
        self.reset();
        Ok(self.tick())
    }

    pub fn apply_setup(&mut self, json: &str) -> Result<String, JsValue> {
        let tuning: CarTuning = serde_json::from_str(json)
            .map_err(|error| JsValue::from_str(&format!("invalid setup: {error}")))?;
        self.vehicle.apply_setup(&tuning);
        Ok(self.setup())
    }

    pub fn reset_setup(&mut self) -> String {
        self.vehicle.reset_setup();
        self.setup()
    }

    pub fn set_traction_control(&mut self, enabled: bool) {
        self.vehicle.set_traction_control(enabled);
    }

    pub fn set_traction_strength(&mut self, strength: f32) {
        self.vehicle.set_traction_strength(strength);
    }

    pub fn set_abs_control(&mut self, enabled: bool) {
        self.vehicle.set_abs_control(enabled);
    }

    pub fn set_stability_control(&mut self, enabled: bool) {
        self.vehicle.set_stability_control(enabled);
    }

    pub fn set_stability_strength(&mut self, strength: f32) {
        self.vehicle.set_stability_strength(strength);
    }

    pub fn set_steering_sensitivity(&mut self, sensitivity: f32) {
        self.vehicle.set_steering_sensitivity(sensitivity);
    }

    pub fn snapshot(&self) -> Vec<f32> {
        let mut out = vec![0.0f32; SNAPSHOT_STATE_FLOATS];
        let state_data = &self.vehicle.state;
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
        out[state::THROTTLE] = self.last_throttle;
        out[state::BRAKE] = self.last_brake;
        out[state::GEAR] = state_data.gear as f32;
        out[state::ABS_ACTIVE] = if self.vehicle.abs_active() { 1.0 } else { 0.0 };
        out[state::ESC_ACTIVITY] = self.vehicle.esc_activity();
        out[state::TICK] = state_data.tick as f32;
        let mut surface_ids = 0u32;
        for (index, wheel) in state_data.wheels.iter().enumerate() {
            surface_ids |= ((wheel.surface_id as u32) & 0x3f) << (index * 6);
        }
        out[state::SURFACE_IDS] = surface_ids as f32;
        out
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
