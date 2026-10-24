mod camera;
mod garage;
mod renderer;
mod scene;

use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::scene::{CarRig, GarageBay, Scene};
use glam::{Mat4, Quat, Vec3};
use sim_core::{Controls, TrackCollider, Vehicle, FIXED_DT};
use sim_types::CarTuning;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct TrackViewer {
    renderer: renderer::Renderer,
    debug_info: String,
    initial_camera: Camera,
    camera: Camera,
}

#[wasm_bindgen]
impl TrackViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        pack_bytes: js_sys::Uint8Array,
    ) -> Result<TrackViewer, JsValue> {
        let bytes = pack_bytes.to_vec();
        let pack = sim_asset::Pack::from_bytes(&bytes).map_err(js_error)?;
        if pack.kind() != sim_types::AssetKind::Track {
            return Err(JsValue::from_str("expected a track pack"));
        }
        let visual = pack.track_visual_owned().map_err(js_error)?;
        let metadata = pack.track_metadata_owned().map_err(js_error)?;
        let collision = pack.track_collision_owned().map_err(js_error)?;
        let manifest = pack.manifest().clone();
        drop(pack);

        let scene = Scene::build(&visual, &metadata, &collision, &manifest)
            .map_err(|error| JsValue::from_str(&error))?;
        let debug_info = scene.debug_info.clone();
        let camera = scene.camera;
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut renderer = renderer::Renderer::new(canvas, width, height, scene).await?;
        renderer.set_lines_visible(false);
        Ok(TrackViewer {
            renderer,
            debug_info,
            initial_camera: camera,
            camera,
        })
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.renderer.resize(width, height);
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        self.renderer.render()
    }

    pub fn debug_info(&self) -> String {
        self.debug_info.clone()
    }

    pub fn camera_info(&self) -> String {
        serde_json::json!({
            "eye": self.camera.eye,
            "target": self.camera.target,
            "fovY": self.camera.fov_y,
        })
        .to_string()
    }

    pub fn set_camera(
        &mut self,
        eye_x: f32,
        eye_y: f32,
        eye_z: f32,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) {
        self.camera.eye = [eye_x, eye_y, eye_z];
        self.camera.target = [target_x, target_y, target_z];
        self.renderer.set_camera(self.camera);
    }

    pub fn reset_camera(&mut self) {
        self.camera = self.initial_camera;
        self.renderer.set_camera(self.camera);
    }
}

#[wasm_bindgen]
pub struct GarageViewer {
    renderer: renderer::Renderer,
    debug_info: String,
    bays: Vec<GarageBay>,
    selected_bay: usize,
    steer: f32,
    spin: f32,
    initial_camera: Camera,
    camera: Camera,
}

#[wasm_bindgen]
impl GarageViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        track_pack_bytes: js_sys::Uint8Array,
        car_pack_bytes: js_sys::Uint8Array,
    ) -> Result<GarageViewer, JsValue> {
        let track_bytes = track_pack_bytes.to_vec();
        let track_pack = sim_asset::Pack::from_bytes(&track_bytes).map_err(js_error)?;
        if track_pack.kind() != sim_types::AssetKind::Track {
            return Err(JsValue::from_str(
                "expected a track pack for the first argument",
            ));
        }
        let track_visual = track_pack.track_visual_owned().map_err(js_error)?;
        let track_metadata = track_pack.track_metadata_owned().map_err(js_error)?;
        let track_collision = track_pack.track_collision_owned().map_err(js_error)?;
        let track_manifest = track_pack.manifest().clone();
        drop(track_pack);

        let car_bytes = car_pack_bytes.to_vec();
        let car_pack = sim_asset::Pack::from_bytes(&car_bytes).map_err(js_error)?;
        if car_pack.kind() != sim_types::AssetKind::Car {
            return Err(JsValue::from_str(
                "expected a car pack for the second argument",
            ));
        }
        let car_physics = car_pack.car_physics_owned().map_err(js_error)?;
        let car_visual = car_pack.car_visual_owned().map_err(js_error)?;
        let car_manifest = car_pack.manifest().clone();
        drop(car_pack);

        let mut scene = Scene::build_garage(
            &track_visual,
            &track_metadata,
            &track_collision,
            &track_manifest,
            &car_physics,
            &car_visual,
            &car_manifest,
            10,
            0,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let bays = std::mem::take(&mut scene.garage_bays);
        let debug_info = scene.debug_info.clone();
        let camera = bays.first().map(|bay| bay.camera).unwrap_or(scene.camera);
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut renderer = renderer::Renderer::new(canvas, width, height, scene).await?;
        renderer.set_lines_visible(false);
        Ok(GarageViewer {
            renderer,
            debug_info,
            bays,
            selected_bay: 0,
            steer: 0.0,
            spin: 0.0,
            initial_camera: camera,
            camera,
        })
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.renderer.resize(width, height);
    }

    pub fn set_wheel_angles(&mut self, steer: f32, spin: f32) {
        self.steer = steer;
        self.spin = spin;
        self.apply_wheel_angles();
    }

    fn apply_wheel_angles(&mut self) {
        let Some(bay) = self.bays.get(self.selected_bay) else {
            return;
        };
        for wheel in &bay.wheels {
            let spin_rotation = glam::Mat4::from_rotation_x(-self.spin);
            let rotation = if wheel.is_front {
                glam::Mat4::from_rotation_y(self.steer) * spin_rotation
            } else {
                spin_rotation
            };
            self.renderer
                .set_mesh_model(wheel.mesh_index, wheel.base_model * rotation);
        }
    }

    pub fn select_bay(&mut self, index: usize) -> String {
        if self.bays.is_empty() {
            return self.camera_info();
        }
        self.selected_bay = index % self.bays.len();
        let bay = &self.bays[self.selected_bay];
        self.camera = bay.camera;
        self.initial_camera = bay.camera;
        self.renderer.set_camera(self.camera);
        self.apply_wheel_angles();
        let _ = self.renderer.render();
        self.camera_info()
    }

    pub fn bay_info(&self) -> String {
        let bay = self.bays.get(self.selected_bay);
        serde_json::json!({
            "selected": self.selected_bay,
            "count": self.bays.len(),
            "name": bay.map(|bay| bay.name.clone()).unwrap_or_default(),
            "color": bay.map(|bay| bay.color),
            "colorName": bay.map(|bay| bay.color_name).unwrap_or_default(),
        })
        .to_string()
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        self.renderer.render()
    }

    pub fn debug_info(&self) -> String {
        self.debug_info.clone()
    }

    pub fn camera_info(&self) -> String {
        serde_json::json!({
            "eye": self.camera.eye,
            "target": self.camera.target,
            "fovY": self.camera.fov_y,
        })
        .to_string()
    }

    pub fn set_camera(
        &mut self,
        eye_x: f32,
        eye_y: f32,
        eye_z: f32,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) {
        self.camera.eye = [eye_x, eye_y, eye_z];
        self.camera.target = [target_x, target_y, target_z];
        self.renderer.set_camera(self.camera);
    }

    pub fn reset_camera(&mut self) {
        self.camera = self.initial_camera;
        self.renderer.set_camera(self.camera);
    }

    pub fn steer_angle(&self) -> f32 {
        self.steer
    }

    pub fn spin_angle(&self) -> f32 {
        self.spin
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

const COCKPIT_GLASS_ALPHA: f32 = 0.32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    Chase,
    Bonnet,
    Cockpit,
}

impl CameraMode {
    fn as_str(self) -> &'static str {
        match self {
            CameraMode::Chase => "chase",
            CameraMode::Bonnet => "bonnet",
            CameraMode::Cockpit => "cockpit",
        }
    }
}

#[wasm_bindgen]
pub struct DriveViewer {
    renderer: renderer::Renderer,
    vehicle: Vehicle,
    previous_state: sim_core::VehicleState,
    rig: CarRig,
    info: String,
    accumulator: f32,
    last_time: Option<f64>,
    step_times: Vec<f32>,
    track_id: String,
    car_id: String,
    car_bay: usize,
    camera_mode: CameraMode,
}

#[wasm_bindgen]
impl DriveViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        track_pack_bytes: js_sys::Uint8Array,
        car_pack_bytes: js_sys::Uint8Array,
        car_bay: Option<usize>,
    ) -> Result<DriveViewer, JsValue> {
        let track_bytes = track_pack_bytes.to_vec();
        let track_pack = sim_asset::Pack::from_bytes(&track_bytes).map_err(js_error)?;
        if track_pack.kind() != sim_types::AssetKind::Track {
            return Err(JsValue::from_str(
                "expected a track pack for the first argument",
            ));
        }
        let track_visual = track_pack.track_visual_owned().map_err(js_error)?;
        let track_metadata = track_pack.track_metadata_owned().map_err(js_error)?;
        let track_collision = track_pack.track_collision_owned().map_err(js_error)?;
        let track_manifest = track_pack.manifest().clone();
        drop(track_pack);

        let car_bytes = car_pack_bytes.to_vec();
        let car_pack = sim_asset::Pack::from_bytes(&car_bytes).map_err(js_error)?;
        if car_pack.kind() != sim_types::AssetKind::Car {
            return Err(JsValue::from_str(
                "expected a car pack for the second argument",
            ));
        }
        let car_physics = car_pack.car_physics_owned().map_err(js_error)?;
        let car_visual = car_pack.car_visual_owned().map_err(js_error)?;
        let car_manifest = car_pack.manifest().clone();
        drop(car_pack);

        let car_bay = car_bay.unwrap_or(0) % garage::BAY_COLOR_NAMES.len();
        let mut scene = Scene::build_garage(
            &track_visual,
            &track_metadata,
            &track_collision,
            &track_manifest,
            &car_physics,
            &car_visual,
            &car_manifest,
            1,
            car_bay,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let rig = scene
            .car_rig
            .take()
            .ok_or_else(|| JsValue::from_str("garage scene has no car rig"))?;
        let info = scene.debug_info.clone();
        let collider = TrackCollider::from_collision(&track_collision, &track_metadata);
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
        let mut vehicle = Vehicle::new(car_physics, collider, &spawn);
        for _ in 0..500 {
            vehicle.step(Controls::default());
        }
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let renderer = renderer::Renderer::new(canvas, width, height, scene).await?;

        let previous_state = vehicle.state;
        let mut drive = DriveViewer {
            renderer,
            vehicle,
            previous_state,
            rig,
            info,
            accumulator: 0.0,
            last_time: None,
            step_times: Vec::with_capacity(256),
            track_id: track_manifest.id,
            car_id: car_manifest.id,
            car_bay,
            camera_mode: CameraMode::Chase,
        };
        drive.write_initial_pose()?;
        Ok(drive)
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.renderer.resize(width, height);
    }

    pub fn reset(&mut self) {
        self.vehicle.reset();
        for _ in 0..500 {
            self.vehicle.step(Controls::default());
        }
        self.previous_state = self.vehicle.state;
        self.accumulator = 0.0;
        self.last_time = None;
        self.step_times.clear();
        self.update_visuals();
        self.update_camera();
        let _ = self.renderer.render();
    }

    pub fn update(
        &mut self,
        time_seconds: f64,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
    ) -> Result<(), JsValue> {
        let dt = match self.last_time {
            Some(last) => ((time_seconds - last).max(0.0) as f32).min(0.05),
            None => 0.0,
        };
        self.last_time = Some(time_seconds);
        self.accumulator += dt;

        let controls = Controls {
            steering,
            throttle,
            brake,
            clutch: 0.0,
            gear,
        };

        let mut steps = 0;
        while self.accumulator >= FIXED_DT && steps < 25 {
            let start = performance_now();
            self.previous_state = self.vehicle.state;
            self.vehicle.step(controls);
            let elapsed = (performance_now() - start) as f32;
            if self.step_times.len() >= 250 {
                self.step_times.remove(0);
            }
            self.step_times.push(elapsed);
            self.accumulator -= FIXED_DT;
            steps += 1;
        }
        if steps == 25 && self.accumulator > FIXED_DT {
            self.accumulator = 0.0;
        }

        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn simulate(
        &mut self,
        seconds: f32,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
    ) -> u32 {
        let steps = ((seconds.clamp(0.0, 120.0) / FIXED_DT) as u32).min(60_000);
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
        self.previous_state = self.vehicle.state;
        self.update_visuals();
        self.update_camera();
        let _ = self.renderer.render();
        self.vehicle.state.tick as u32
    }

    pub fn telemetry(&self) -> String {
        let state = &self.vehicle.state;
        let speed = state.velocity.length();
        let mut times = self.step_times.clone();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let p50 = percentile(&times, 0.50);
        let p99 = percentile(&times, 0.99);
        let labels = ["FL", "FR", "RL", "RR"];
        let wheels = state
            .wheels
            .iter()
            .enumerate()
            .map(|(index, wheel)| {
                serde_json::json!({
                    "name": labels[index],
                    "contact": wheel.contact,
                    "load": wheel.normal_load,
                    "compression": wheel.suspension_compression,
                    "slipRatio": wheel.slip_ratio,
                    "slipAngle": wheel.slip_angle,
                    "surfaceFriction": wheel.surface_friction,
                    "steerAngle": wheel.steer_angle,
                    "spinAngle": wheel.spin_angle,
                    "angularVelocity": wheel.angular_velocity,
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "track": self.track_id,
            "car": self.car_id,
            "carBay": self.car_bay,
            "carColor": garage::BAY_COLOR_NAMES[self.car_bay],
            "tick": state.tick,
            "speedKph": speed * 3.6,
            "speedMps": speed,
            "rpm": state.engine_rpm,
            "gear": state.gear,
            "position": [state.position.x, state.position.y, state.position.z],
            "velocity": [state.velocity.x, state.velocity.y, state.velocity.z],
            "steeringRackTorque": state.steering_rack_torque,
            "tractionControl": self.vehicle.traction_control(),
            "tractionStrength": self.vehicle.traction_strength(),
            "absControl": self.vehicle.abs_control(),
            "absActive": self.vehicle.abs_active(),
            "stabilityControl": self.vehicle.stability_control(),
            "stabilityStrength": self.vehicle.stability_strength(),
            "steeringSensitivity": self.vehicle.steering_sensitivity(),
            "escActivity": self.vehicle.esc_activity(),
            "debugLinesVisible": self.renderer.lines_visible(),
            "msaaSamples": self.renderer.msaa_samples(),
            "rendererBackend": self.renderer.backend(),
            "rendererFormat": self.renderer.surface_format(),
            "cameraMode": self.camera_mode.as_str(),
            "contacts": state.wheels.iter().filter(|wheel| wheel.contact).count(),
            "stepP50Ms": p50,
            "stepP99Ms": p99,
            "wheels": wheels,
        })
        .to_string()
    }

    pub fn info(&self) -> String {
        self.info.clone()
    }

    pub fn set_camera_mode(&mut self, mode: &str) {
        self.camera_mode = if mode.eq_ignore_ascii_case("bonnet") {
            CameraMode::Bonnet
        } else if mode.eq_ignore_ascii_case("cockpit") {
            CameraMode::Cockpit
        } else {
            CameraMode::Chase
        };
        if let Some(index) = self.rig.glass_mesh_index {
            let alpha = if self.camera_mode == CameraMode::Cockpit {
                COCKPIT_GLASS_ALPHA
            } else {
                1.0
            };
            self.renderer.set_mesh_alpha(index, alpha);
        }
        self.update_camera();
    }

    pub fn set_traction_control(&mut self, enabled: bool) {
        self.vehicle.set_traction_control(enabled);
    }

    pub fn set_stability_control(&mut self, enabled: bool) {
        self.vehicle.set_stability_control(enabled);
    }

    pub fn set_traction_strength(&mut self, strength: f32) {
        self.vehicle.set_traction_strength(strength);
    }

    pub fn set_abs_control(&mut self, enabled: bool) {
        self.vehicle.set_abs_control(enabled);
    }

    pub fn set_stability_strength(&mut self, strength: f32) {
        self.vehicle.set_stability_strength(strength);
    }

    pub fn set_steering_sensitivity(&mut self, sensitivity: f32) {
        self.vehicle.set_steering_sensitivity(sensitivity);
    }

    pub fn set_debug_lines_visible(&mut self, visible: bool) {
        self.renderer.set_lines_visible(visible);
    }

    pub fn set_msaa_samples(&mut self, requested: u32) -> Result<u32, JsValue> {
        let actual = self.renderer.set_msaa_samples(requested);
        self.renderer.render()?;
        Ok(actual)
    }

    pub fn msaa_samples(&self) -> u32 {
        self.renderer.msaa_samples()
    }

    pub fn car_setup(&self) -> String {
        serde_json::to_string(&self.vehicle.setup()).unwrap_or_else(|_| String::from("{}"))
    }

    pub fn default_car_setup(&self) -> String {
        serde_json::to_string(&self.vehicle.default_setup()).unwrap_or_else(|_| String::from("{}"))
    }

    pub fn apply_car_setup(&mut self, json: &str) -> Result<(), JsValue> {
        let tuning: CarTuning = serde_json::from_str(json).map_err(js_error)?;
        self.vehicle.apply_setup(&tuning);
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn reset_car_setup(&mut self) -> Result<(), JsValue> {
        self.vehicle.reset_setup();
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn set_body_visible(&mut self, visible: bool) {
        self.renderer
            .set_mesh_visible(self.rig.body_mesh_index, visible);
        if let Some(index) = self.rig.glass_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
    }

    pub fn camera_mode(&self) -> String {
        self.camera_mode.as_str().to_string()
    }

    fn write_initial_pose(&mut self) -> Result<(), JsValue> {
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    fn update_visuals(&mut self) {
        let state = &self.vehicle.state;
        let (position, rotation) = self.visual_pose();
        let car_world = Mat4::from_rotation_translation(rotation, position);
        self.renderer
            .set_mesh_model(self.rig.body_mesh_index, car_world * self.rig.body_local);
        if let Some(index) = self.rig.glass_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rig.body_local);
        }
        for (index, wheel) in self.rig.wheels.iter().enumerate() {
            let wheel_state = state.wheels.get(index).copied().unwrap_or_default();
            let spin = Mat4::from_rotation_x(-wheel_state.spin_angle);
            let rotation = if wheel.is_front {
                Mat4::from_rotation_y(wheel_state.steer_angle) * spin
            } else {
                spin
            };
            self.renderer
                .set_mesh_model(wheel.mesh_index, car_world * wheel.local_model * rotation);
        }
        if let Some(index) = self.rig.steering_mesh_index {
            let steer = state.wheels[0].steer_angle;
            let model = car_world * self.rig.steering_local * Mat4::from_rotation_x(-steer * 3.0);
            self.renderer.set_mesh_model(index, model);
        }
    }

    fn update_camera(&mut self) {
        let (position, rotation) = self.visual_pose();
        if self.camera_mode == CameraMode::Bonnet {
            let eye = position + rotation * Vec3::new(-0.35, 1.25, -0.1);
            let target = position + rotation * Vec3::new(-0.35, 1.05, -20.0);
            self.renderer.set_camera(Camera {
                eye: eye.to_array(),
                target: target.to_array(),
                fov_y: 1.0,
            });
            return;
        }
        if self.camera_mode == CameraMode::Cockpit {
            let eye = position + rotation * Vec3::new(-0.35, 1.10, 0.25);
            let target = position + rotation * Vec3::new(-0.35, 1.00, -20.0);
            self.renderer.set_camera(Camera {
                eye: eye.to_array(),
                target: target.to_array(),
                fov_y: 1.0,
            });
            return;
        }

        let aspect = self.renderer.aspect();
        let landscape = ((aspect - 0.75) / (2.0 - 0.75)).clamp(0.0, 1.0);
        let back = 7.2 + (4.8 - 7.2) * landscape;
        let height = 2.7 + (2.0 - 2.7) * landscape;
        let look_ahead = -5.0 + 2.5 * landscape;
        let eye = position + rotation * Vec3::new(0.0, height, back);
        let target = position + rotation * Vec3::new(0.0, 0.65, look_ahead);
        self.renderer.set_camera(Camera {
            eye: [eye.x, eye.y.max(0.8), eye.z],
            target: [target.x, target.y, target.z],
            fov_y: 0.95,
        });
    }

    fn visual_pose(&self) -> (Vec3, Quat) {
        let alpha = if self.last_time.is_some() {
            (self.accumulator / FIXED_DT).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let position = self
            .previous_state
            .position
            .lerp(self.vehicle.state.position, alpha);
        let rotation = self
            .previous_state
            .rotation
            .slerp(self.vehicle.state.rotation, alpha)
            .normalize();
        (position, rotation)
    }
}

fn percentile(values: &[f32], fraction: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f32 * fraction).round() as usize;
    values[index.min(values.len() - 1)]
}

fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}
