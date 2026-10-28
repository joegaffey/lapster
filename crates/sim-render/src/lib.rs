#![recursion_limit = "256"]

mod camera;
mod garage;
mod particles;
mod renderer;
mod scene;
mod tuning;

use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::particles::ParticleSystem;
use crate::renderer::{CarShadowMode, EnvironmentShadowMode};
use crate::scene::{CarRig, CockpitNeedleSource, GarageBay, Scene};
use crate::tuning::{GpuTuning, TuningConfig};
use glam::{Mat4, Quat, Vec3};
use sim_core::{Controls, PhysicsTuning, TrackCollider, Vehicle, FIXED_DT};
use sim_types::snapshot_state as snapshot;
use sim_types::{CarTuning, SNAPSHOT_STATE_FLOATS};

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
        let shadow_focus = camera.target;
        let (collision_center, region_half) = track_region(&collision, &metadata);
        let region_center = metadata
            .spawns
            .first()
            .map(|spawn| spawn.position)
            .unwrap_or(collision_center);
        let collider = TrackCollider::from_collision(&collision, &metadata);
        let ground_y = collider
            .raycast(
                Vec3::new(region_center[0], region_center[1] + 100.0, region_center[2]),
                Vec3::NEG_Y,
                300.0,
            )
            .map(|hit| hit.point.y)
            .unwrap_or(region_center[1]);
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut renderer =
            renderer::Renderer::new(canvas, width, height, scene, &GpuTuning::default()).await?;
        renderer.set_lines_visible(false);
        renderer.set_baked_region(collision_center, region_half, ground_y);
        renderer.set_shadow_focus(shadow_focus, TRACK_SHADOW_EXTENT);
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
        camera_json(&self.camera)
    }

    pub fn set_environment_shadow_mode(&mut self, mode: &str) {
        self.renderer
            .set_environment_shadow_mode(environment_shadow_mode(mode));
    }

    pub fn environment_shadow_mode(&self) -> String {
        environment_shadow_mode_name(self.renderer.environment_shadow_mode()).to_string()
    }

    pub fn set_car_shadow_mode(&mut self, mode: &str) {
        self.renderer
            .set_car_shadow_mode(car_shadow_mode(mode), false);
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

    pub fn car_shadow_mode(&self) -> String {
        car_shadow_mode_name(self.renderer.car_shadow_mode()).to_string()
    }

    pub fn set_time_of_day(&mut self, hours: f32) -> f64 {
        self.renderer.set_time_of_day(hours)
    }

    pub fn time_of_day(&self) -> f32 {
        self.renderer.time_of_day()
    }

    pub fn baked_resolution(&self) -> u32 {
        self.renderer.baked_resolution()
    }

    pub fn last_bake_ms(&self) -> f64 {
        self.renderer.last_bake_ms()
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
        self.renderer
            .set_shadow_focus([target_x, target_y, target_z], TRACK_SHADOW_EXTENT);
    }

    pub fn reset_camera(&mut self) {
        self.camera = self.initial_camera;
        self.renderer.set_camera(self.camera);
        self.renderer
            .set_shadow_focus(self.camera.target, TRACK_SHADOW_EXTENT);
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
    car_shadow_range: Option<std::ops::Range<usize>>,
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
            true,
            false,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let bays = std::mem::take(&mut scene.garage_bays);
        let car_shadow_range = scene.car_mesh_range.clone();
        let debug_info = scene.debug_info.clone();
        let camera = bays.first().map(|bay| bay.camera).unwrap_or(scene.camera);
        let shadow_focus = camera.target;
        let collider = TrackCollider::from_collision(&track_collision, &track_metadata);
        let spawn = track_metadata.spawns.first().cloned();
        let center = spawn
            .as_ref()
            .map(|spawn| [spawn.position[0], spawn.position[1], spawn.position[2]])
            .unwrap_or(shadow_focus);
        let ground_y = collider
            .raycast(
                Vec3::new(center[0], center[1] + 100.0, center[2]),
                Vec3::NEG_Y,
                300.0,
            )
            .map(|hit| hit.point.y)
            .unwrap_or(center[1]);
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for bay in &bays {
            let point = Vec3::from_array(bay.decal_center);
            min = min.min(point);
            max = max.max(point);
        }
        let showroom_center = if min.is_finite() && max.is_finite() {
            (min + max) * 0.5
        } else {
            Vec3::new(center[0], ground_y, center[2])
        };
        let showroom_half = ((max.x - min.x).max(max.z - min.z) * 0.5 + 20.0).clamp(60.0, 140.0);
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut renderer =
            renderer::Renderer::new(canvas, width, height, scene, &GpuTuning::default()).await?;
        renderer.set_lines_visible(false);
        renderer.set_baked_region(showroom_center.to_array(), showroom_half, ground_y);
        renderer.set_shadow_focus(shadow_focus, GARAGE_SHADOW_EXTENT);
        let mut viewer = GarageViewer {
            renderer,
            debug_info,
            bays,
            selected_bay: 0,
            steer: 0.0,
            spin: 0.0,
            initial_camera: camera,
            camera,
            car_shadow_range,
        };
        viewer.update_decals();
        Ok(viewer)
    }

    fn update_decals(&mut self) {
        let light = Vec3::from_array(self.renderer.sun_direction());
        for bay in &self.bays {
            let Some(index) = bay.decal_mesh_index else {
                continue;
            };
            let height = (bay.decal_ground_y - bay.decal_center[1]).abs() + 0.7;
            self.renderer.set_shadow_decal(
                index,
                bay.decal_center,
                [0.0, 1.0, 0.0],
                bay.decal_yaw,
                bay.decal_width,
                bay.decal_length,
                light.to_array(),
                height,
            );
        }
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
        let camera = self.bays[self.selected_bay].camera;
        self.initial_camera = camera;
        self.renderer
            .set_shadow_focus(camera.target, GARAGE_SHADOW_EXTENT);
        self.apply_wheel_angles();
        camera_json(&camera)
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
        camera_json(&self.camera)
    }

    pub fn set_environment_shadow_mode(&mut self, mode: &str) {
        self.renderer
            .set_environment_shadow_mode(environment_shadow_mode(mode));
    }

    pub fn environment_shadow_mode(&self) -> String {
        environment_shadow_mode_name(self.renderer.environment_shadow_mode()).to_string()
    }

    pub fn set_car_shadow_mode(&mut self, mode: &str) {
        let mode = car_shadow_mode(mode);
        self.renderer.set_car_shadow_mode(mode, false);
        if let Some(range) = self.car_shadow_range.clone() {
            self.renderer
                .set_static_shadow_range(range, mode != CarShadowMode::Off);
        }
        self.update_decals();
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

    pub fn car_shadow_mode(&self) -> String {
        car_shadow_mode_name(self.renderer.car_shadow_mode()).to_string()
    }

    pub fn set_time_of_day(&mut self, hours: f32) -> f64 {
        let bake_ms = self.renderer.set_time_of_day(hours);
        self.update_decals();
        bake_ms
    }

    pub fn time_of_day(&self) -> f32 {
        self.renderer.time_of_day()
    }

    pub fn baked_resolution(&self) -> u32 {
        self.renderer.baked_resolution()
    }

    pub fn last_bake_ms(&self) -> f64 {
        self.renderer.last_bake_ms()
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
        self.renderer
            .set_shadow_focus([target_x, target_y, target_z], GARAGE_SHADOW_EXTENT);
    }

    pub fn reset_camera(&mut self) {
        self.camera = self.initial_camera;
        self.renderer.set_camera(self.camera);
        self.renderer
            .set_shadow_focus(self.camera.target, GARAGE_SHADOW_EXTENT);
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

fn merge_json(target: &mut serde_json::Value, patch: serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(target), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                match target.get_mut(&key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        target.insert(key, value);
                    }
                }
            }
        }
        (target, patch) => *target = patch,
    }
}

fn track_region(
    collision: &sim_types::TrackCollision,
    metadata: &sim_types::TrackMetadata,
) -> ([f32; 3], f32) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    for (triangle, surface_id) in collision.surface_ids.iter().enumerate() {
        let grass = metadata
            .surfaces
            .get(*surface_id as usize)
            .is_some_and(|surface| surface.name.to_ascii_uppercase().contains("GRASS"));
        if grass {
            continue;
        }
        let start = triangle * 3;
        let Some(indices) = collision.indices.get(start..start + 3) else {
            break;
        };
        for index in indices {
            let Some(vertex) = collision.vertices.get(*index as usize) else {
                continue;
            };
            let point = Vec3::from_array(*vertex);
            min = min.min(point);
            max = max.max(point);
            any = true;
        }
    }
    if !any || !min.is_finite() || !max.is_finite() {
        for vertex in &collision.vertices {
            let point = Vec3::from_array(*vertex);
            min = min.min(point);
            max = max.max(point);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return ([0.0, 0.0, 0.0], 80.0);
    }
    let center = (min + max) * 0.5;
    let span = max - min;
    let half = (span.x.max(span.z) * 0.5 + 20.0).max(60.0);
    ([center.x, center.y, center.z], half)
}

pub(crate) fn shadow_decal_yaw(forward: Vec3) -> f32 {
    (-forward.x).atan2(-forward.z)
}

fn environment_shadow_mode(mode: &str) -> EnvironmentShadowMode {
    if mode.eq_ignore_ascii_case("hybrid") {
        EnvironmentShadowMode::Hybrid
    } else if mode.eq_ignore_ascii_case("baked") {
        EnvironmentShadowMode::Baked
    } else if mode.eq_ignore_ascii_case("off") {
        EnvironmentShadowMode::Off
    } else {
        EnvironmentShadowMode::High
    }
}

fn environment_shadow_mode_name(mode: EnvironmentShadowMode) -> &'static str {
    match mode {
        EnvironmentShadowMode::High => "high",
        EnvironmentShadowMode::Hybrid => "hybrid",
        EnvironmentShadowMode::Baked => "baked",
        EnvironmentShadowMode::Off => "off",
    }
}

fn car_shadow_mode(mode: &str) -> CarShadowMode {
    if mode.eq_ignore_ascii_case("silhouette") {
        CarShadowMode::Silhouette
    } else if mode.eq_ignore_ascii_case("off") {
        CarShadowMode::Off
    } else {
        CarShadowMode::Dynamic
    }
}

fn car_shadow_mode_name(mode: CarShadowMode) -> &'static str {
    match mode {
        CarShadowMode::Dynamic => "dynamic",
        CarShadowMode::Silhouette => "silhouette",
        CarShadowMode::Off => "off",
    }
}

fn camera_json(camera: &Camera) -> String {
    serde_json::json!({
        "eye": camera.eye,
        "target": camera.target,
        "fovY": camera.fov_y,
    })
    .to_string()
}

const GARAGE_SHADOW_EXTENT: f32 = 80.0;
const TRACK_SHADOW_EXTENT: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    Chase,
    Bonnet,
    Cockpit,
    Isometric,
}

impl CameraMode {
    fn as_str(self) -> &'static str {
        match self {
            CameraMode::Chase => "chase",
            CameraMode::Bonnet => "bonnet",
            CameraMode::Cockpit => "cockpit",
            CameraMode::Isometric => "isometric",
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
    brake_lights_on: bool,
    reverse_lights_on: bool,
    surface_names: Vec<String>,
    particles: ParticleSystem,
    last_particle_time: Option<f64>,
    tuning: TuningConfig,
    track_shadow_center: Vec3,
    track_shadow_half: f32,
}

#[wasm_bindgen]
impl DriveViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        track_pack_bytes: js_sys::Uint8Array,
        car_pack_bytes: js_sys::Uint8Array,
        car_bay: Option<usize>,
        tuning_json: Option<String>,
    ) -> Result<DriveViewer, JsValue> {
        let mut tuning = tuning_json
            .and_then(|json| serde_json::from_str::<TuningConfig>(&json).ok())
            .unwrap_or_default();
        tuning.sanitize();
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
        let surface_names = track_metadata
            .surfaces
            .iter()
            .map(|surface| surface.name.clone())
            .collect::<Vec<_>>();

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
            false,
            true,
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
        vehicle.set_physics_tuning(&tuning.physics);
        for _ in 0..500 {
            vehicle.step(Controls::default());
        }
        let (track_shadow_center, track_shadow_half) =
            track_region(&track_collision, &track_metadata);
        let region_half = track_shadow_half;
        let baked_half_extent = if tuning.gpu.baked_half_extent > 0.0 {
            tuning
                .gpu
                .baked_half_extent
                .clamp(10.0, region_half.max(10.0))
        } else {
            region_half
        };
        let region_center = vehicle.state.position.to_array();
        let ground_y = vehicle
            .state
            .wheels
            .iter()
            .filter(|wheel| wheel.contact)
            .map(|wheel| wheel.contact_point.y)
            .fold(f32::INFINITY, f32::min);
        let ground_y = if ground_y.is_finite() {
            ground_y
        } else {
            region_center[1]
        };
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut renderer =
            renderer::Renderer::new(canvas, width, height, scene, &tuning.gpu).await?;
        renderer.set_baked_region(track_shadow_center, baked_half_extent, ground_y);

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
            brake_lights_on: false,
            reverse_lights_on: false,
            surface_names,
            particles: ParticleSystem::new(),
            last_particle_time: None,
            tuning,
            track_shadow_center: Vec3::from_array(track_shadow_center),
            track_shadow_half,
        };
        drive.renderer.set_shadow_tuning(&drive.tuning.shadows);
        drive
            .renderer
            .set_particle_render_tuning(&drive.tuning.particles.render);
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
        self.last_particle_time = None;
        self.step_times.clear();
        self.particles.clear();
        self.set_brake_lights(0.0);
        self.set_reverse_lights(false);
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
        self.set_brake_lights(brake);
        self.set_reverse_lights(gear == -1);

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
        self.set_brake_lights(brake);
        self.set_reverse_lights(gear == -1);
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

    pub fn render_remote(&mut self, frame: &[f32]) -> Result<(), JsValue> {
        if frame.len() < SNAPSHOT_STATE_FLOATS {
            return Err(JsValue::from_str("remote snapshot is too short"));
        }
        let position = Vec3::new(
            frame[snapshot::POSITION],
            frame[snapshot::POSITION + 1],
            frame[snapshot::POSITION + 2],
        );
        let rotation = Quat::from_xyzw(
            frame[snapshot::ROTATION],
            frame[snapshot::ROTATION + 1],
            frame[snapshot::ROTATION + 2],
            frame[snapshot::ROTATION + 3],
        );
        let rotation = if rotation.length_squared() > 1e-8 {
            rotation.normalize()
        } else {
            Quat::IDENTITY
        };
        let contact_center = Vec3::new(
            frame[snapshot::CONTACT_CENTER],
            frame[snapshot::CONTACT_CENTER + 1],
            frame[snapshot::CONTACT_CENTER + 2],
        );
        let contact_normal = Vec3::new(
            frame[snapshot::CONTACT_NORMAL],
            frame[snapshot::CONTACT_NORMAL + 1],
            frame[snapshot::CONTACT_NORMAL + 2],
        )
        .normalize_or(Vec3::Y);
        let mut remote = self.vehicle.state;
        remote.position = position;
        remote.rotation = rotation;
        remote.velocity = Vec3::new(
            frame[snapshot::VELOCITY],
            frame[snapshot::VELOCITY + 1],
            frame[snapshot::VELOCITY + 2],
        );
        remote.angular_velocity = Vec3::new(
            frame[snapshot::ANGULAR_VELOCITY],
            frame[snapshot::ANGULAR_VELOCITY + 1],
            frame[snapshot::ANGULAR_VELOCITY + 2],
        );
        remote.engine_rpm = frame[snapshot::ENGINE_RPM];
        remote.gear = frame[snapshot::GEAR].round() as i32;
        remote.steering_rack_torque = frame[snapshot::RACK_TORQUE];
        remote.tick = frame[snapshot::TICK].max(0.0) as u64;
        let surface_ids = frame[snapshot::SURFACE_IDS].max(0.0) as u32;
        for (index, wheel) in remote.wheels.iter_mut().enumerate() {
            wheel.surface_id = ((surface_ids >> (index * 6)) & 0x3f) as u16;
            wheel.angular_velocity = frame[snapshot::WHEEL_ANGULAR_SPEEDS + index];
            wheel.suspension_compression = frame[snapshot::SUSPENSION_TRAVEL + index];
            wheel.slip_ratio = frame[snapshot::SLIP_RATIO + index];
            wheel.slip_angle = frame[snapshot::SLIP_ANGLE + index];
            wheel.normal_load = frame[snapshot::TIRE_LOAD + index];
            wheel.contact = frame[snapshot::CONTACT_FLAGS + index] > 0.5;
            wheel.surface_friction = frame[snapshot::SURFACE_FRICTION + index];
            wheel.steer_angle = frame[snapshot::WHEEL_STEER_ANGLE + index];
            wheel.spin_angle = frame[snapshot::WHEEL_SPIN_ANGLE + index];
            wheel.contact_point = contact_center;
            wheel.contact_normal = contact_normal;
        }
        self.vehicle.state = remote;
        self.previous_state = remote;
        self.last_time = None;
        self.accumulator = 0.0;
        self.set_brake_lights(frame[snapshot::BRAKE]);
        self.set_reverse_lights(remote.gear == -1);
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
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
                    "surfaceId": wheel.surface_id,
                    "surface": self.surface_names.get(wheel.surface_id as usize).cloned().unwrap_or_default(),
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
            "shadowMapSize": self.renderer.shadow_map_size(),
            "carShadowMapSize": self.renderer.car_shadow_map_size(),
            "shadowFocus": self.renderer.shadow_focus(),
            "shadowsEnabled": self.renderer.shadows_enabled(),
            "environmentShadowMode": environment_shadow_mode_name(self.renderer.environment_shadow_mode()),
            "carShadowMode": car_shadow_mode_name(self.renderer.effective_car_shadow_mode()),
            "brakeLightsOn": self.brake_lights_on,
            "reverseLightsOn": self.reverse_lights_on,
            "timeOfDay": self.renderer.time_of_day(),
            "sunElevation": self.renderer.sun_elevation(),
            "bakedShadowResolution": self.renderer.baked_resolution(),
            "bakedRegionHalfExtent": self.renderer.baked_region_half_extent(),
            "shadowBakeMs": self.renderer.last_bake_ms(),
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
            "particles": self.particles.count(),
            "stepP50Ms": p50,
            "stepP99Ms": p99,
            "wheels": wheels,
        })
        .to_string()
    }

    pub fn info(&self) -> String {
        self.info.clone()
    }

    pub fn set_environment_shadow_mode(&mut self, mode: &str) {
        self.renderer
            .set_environment_shadow_mode(environment_shadow_mode(mode));
    }

    pub fn environment_shadow_mode(&self) -> String {
        environment_shadow_mode_name(self.renderer.environment_shadow_mode()).to_string()
    }

    pub fn set_car_shadow_mode(&mut self, mode: &str) {
        self.renderer
            .set_car_shadow_mode(car_shadow_mode(mode), true);
    }

    pub fn car_shadow_mode(&self) -> String {
        car_shadow_mode_name(self.renderer.car_shadow_mode()).to_string()
    }

    pub fn effective_car_shadow_mode(&self) -> String {
        car_shadow_mode_name(self.renderer.effective_car_shadow_mode()).to_string()
    }

    pub fn set_time_of_day(&mut self, hours: f32) -> f64 {
        let bake_ms = self.renderer.set_time_of_day(hours);
        self.update_visuals();
        self.update_camera();
        bake_ms
    }

    pub fn time_of_day(&self) -> f32 {
        self.renderer.time_of_day()
    }

    pub fn baked_resolution(&self) -> u32 {
        self.renderer.baked_resolution()
    }

    pub fn last_bake_ms(&self) -> f64 {
        self.renderer.last_bake_ms()
    }

    pub fn set_camera_mode(&mut self, mode: &str) {
        self.camera_mode = if mode.eq_ignore_ascii_case("bonnet") {
            CameraMode::Bonnet
        } else if mode.eq_ignore_ascii_case("cockpit") {
            CameraMode::Cockpit
        } else if mode.eq_ignore_ascii_case("isometric") || mode.eq_ignore_ascii_case("top") {
            CameraMode::Isometric
        } else {
            CameraMode::Chase
        };
        self.apply_cockpit_glass_alpha();
        self.update_dashboard_visibility();
        self.update_camera();
    }

    fn update_dashboard_visibility(&mut self) {
        let visible = self.camera_mode == CameraMode::Cockpit;
        if let Some(index) = self.rig.dashboard_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        for needle in &self.rig.cockpit_needles {
            self.renderer.set_mesh_visible(needle.mesh_index, visible);
        }
    }

    fn apply_cockpit_glass_alpha(&mut self) {
        if let Some(index) = self.rig.glass_mesh_index {
            let alpha = if self.camera_mode == CameraMode::Cockpit {
                self.tuning.camera.cockpit_glass_alpha
            } else {
                1.0
            };
            self.renderer.set_mesh_alpha(index, alpha);
        }
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

    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.renderer.set_shadows_enabled(enabled);
    }

    pub fn shadows_enabled(&self) -> bool {
        self.renderer.shadows_enabled()
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
        if let Some(index) = self.rig.brake_light_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        if let Some(index) = self.rig.reverse_light_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
    }

    fn set_brake_lights(&mut self, brake: f32) {
        let on = brake > 0.05;
        if on == self.brake_lights_on {
            return;
        }
        self.brake_lights_on = on;
        if let Some(index) = self.rig.brake_light_mesh_index {
            self.renderer
                .set_mesh_emissive(index, if on { [5.0, 0.12, 0.02] } else { [0.0; 3] });
        }
    }

    fn set_reverse_lights(&mut self, on: bool) {
        if on == self.reverse_lights_on {
            return;
        }
        self.reverse_lights_on = on;
        if let Some(index) = self.rig.reverse_light_mesh_index {
            self.renderer
                .set_mesh_emissive(index, if on { [3.5, 3.5, 3.5] } else { [0.0; 3] });
        }
    }

    pub fn camera_mode(&self) -> String {
        self.camera_mode.as_str().to_string()
    }

    pub fn tuning(&self) -> String {
        let values = serde_json::to_value(&self.tuning).unwrap_or(serde_json::Value::Null);
        let schema = serde_json::from_str(&TuningConfig::schema_json())
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        serde_json::json!({ "values": values, "schema": schema }).to_string()
    }

    pub fn apply_tuning(&mut self, json: &str) -> Result<(), JsValue> {
        let patch: serde_json::Value = serde_json::from_str(json).map_err(js_error)?;
        let mut values = serde_json::to_value(&self.tuning).map_err(js_error)?;
        merge_json(&mut values, patch);
        let mut tuning: TuningConfig = serde_json::from_value(values).map_err(js_error)?;
        tuning.sanitize();
        self.tuning = tuning;
        self.renderer.set_shadow_tuning(&self.tuning.shadows);
        self.renderer
            .set_particle_render_tuning(&self.tuning.particles.render);
        self.apply_cockpit_glass_alpha();
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn apply_physics_tuning(&mut self, json: &str) -> Result<(), JsValue> {
        let tuning: PhysicsTuning = serde_json::from_str(json).map_err(js_error)?;
        self.vehicle.set_physics_tuning(&tuning);
        self.vehicle.reset();
        for _ in 0..500 {
            self.vehicle.step(Controls::default());
        }
        self.previous_state = self.vehicle.state;
        self.accumulator = 0.0;
        self.last_time = None;
        self.last_particle_time = None;
        self.particles.clear();
        self.set_brake_lights(0.0);
        self.set_reverse_lights(false);
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn reset_tuning(&mut self) -> Result<(), JsValue> {
        self.tuning = TuningConfig::default();
        self.renderer.set_shadow_tuning(&self.tuning.shadows);
        self.renderer
            .set_particle_render_tuning(&self.tuning.particles.render);
        self.apply_cockpit_glass_alpha();
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
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
        let full_track_dynamic = self.tuning.shadows.full_track
            && self.renderer.environment_shadow_mode() == EnvironmentShadowMode::High;
        let (focus, extent) = if full_track_dynamic {
            (self.track_shadow_center, self.track_shadow_half)
        } else {
            let focus = if self.renderer.car_shadow_separate() {
                Vec3::from_array(
                    self.renderer
                        .car_shadow_focus((position + Vec3::Y * 0.5).to_array()),
                )
            } else {
                position + rotation * Vec3::NEG_Z * 8.0 + Vec3::Y * 0.5
            };
            (focus, self.tuning.shadows.drive_extent)
        };
        self.renderer.set_shadow_focus(focus.to_array(), extent);
        self.renderer
            .set_mesh_model(self.rig.body_mesh_index, car_world * self.rig.body_local);
        if let Some(index) = self.rig.dashboard_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rig.dashboard_local);
        }
        let speed_kph = state.velocity.length() * 3.6;
        let gear_fraction = ((state.gear.max(1) - 1) as f32 / 5.0).clamp(0.0, 1.0);
        for needle in &self.rig.cockpit_needles {
            let fraction = match needle.source {
                CockpitNeedleSource::Speed => (speed_kph / 240.0).clamp(0.0, 1.0),
                CockpitNeedleSource::Rpm => (state.engine_rpm / 8_000.0).clamp(0.0, 1.0),
                CockpitNeedleSource::Gear => gear_fraction,
            };
            let angle =
                -std::f32::consts::FRAC_PI_4 * 3.0 + fraction * std::f32::consts::FRAC_PI_4 * 6.0;
            let model = car_world
                * self.rig.dashboard_local
                * Mat4::from_translation(needle.center)
                * Mat4::from_rotation_z(angle);
            self.renderer.set_mesh_model(needle.mesh_index, model);
        }
        if let Some(index) = self.rig.glass_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rig.body_local);
        }
        if let Some(index) = self.rig.brake_light_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rig.body_local);
        }
        if let Some(index) = self.rig.reverse_light_mesh_index {
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
        if let Some(index) = self.rig.decal_mesh_index {
            let mut contact_count = 0.0f32;
            let mut contact = Vec3::ZERO;
            let mut normal = Vec3::ZERO;
            for wheel in &state.wheels {
                if wheel.contact {
                    contact += wheel.contact_point;
                    normal += wheel.contact_normal;
                    contact_count += 1.0;
                }
            }
            let base = if contact_count > 0.0 {
                contact / contact_count
            } else {
                position
            };
            let normal = if normal.length_squared() > 0.001 {
                normal.normalize()
            } else {
                Vec3::Y
            };
            let light = Vec3::from_array(self.renderer.sun_direction());
            let forward = rotation * Vec3::NEG_Z;
            let yaw = shadow_decal_yaw(forward);
            self.renderer.set_shadow_decal(
                index,
                base.to_array(),
                normal.to_array(),
                yaw,
                self.rig.decal_width,
                self.rig.decal_length,
                light.to_array(),
                0.7,
            );
        }

        let now = performance_now();
        let particle_dt = self
            .last_particle_time
            .map(|last| ((now - last) / 1000.0) as f32)
            .unwrap_or(0.0);
        self.last_particle_time = Some(now);
        self.particles.update(
            particle_dt,
            car_world,
            state,
            &self.rig,
            &self.surface_names,
            &self.tuning.particles,
        );
        self.renderer.set_particles(self.particles.instances());
    }

    fn update_camera(&mut self) {
        let (position, rotation) = self.visual_pose();
        if self.camera_mode == CameraMode::Bonnet {
            let shot = &self.tuning.camera.bonnet;
            let eye = position + rotation * Vec3::from_array(shot.eye);
            let target = position + rotation * Vec3::from_array(shot.target);
            self.renderer.set_camera(Camera {
                eye: eye.to_array(),
                target: target.to_array(),
                fov_y: shot.fov_y,
            });
            return;
        }
        if self.camera_mode == CameraMode::Cockpit {
            let shot = &self.tuning.camera.cockpit;
            let eye = position + rotation * Vec3::from_array(shot.eye);
            let target = position + rotation * Vec3::from_array(shot.target);
            self.renderer.set_camera(Camera {
                eye: eye.to_array(),
                target: target.to_array(),
                fov_y: shot.fov_y,
            });
            return;
        }
        if self.camera_mode == CameraMode::Isometric {
            let tuning = &self.tuning.camera.isometric;
            let focus = position + Vec3::Y * tuning.focus_height;
            let aspect = self.renderer.aspect();
            let fit = (1.0 / aspect.max(tuning.fit_aspect_floor))
                .powf(tuning.fit_power)
                .clamp(tuning.fit_min, tuning.fit_max);
            let offset = Vec3::from_array(tuning.offset) * fit;
            self.renderer.set_camera(Camera {
                eye: (focus + offset).to_array(),
                target: focus.to_array(),
                fov_y: tuning.fov_y,
            });
            return;
        }

        let tuning = &self.tuning.camera.chase;
        let aspect = self.renderer.aspect();
        let range = (tuning.aspect_far - tuning.aspect_near).max(0.0001);
        let landscape = ((aspect - tuning.aspect_near) / range).clamp(0.0, 1.0);
        let back = tuning.back_near + (tuning.back_far - tuning.back_near) * landscape;
        let height = tuning.height_near + (tuning.height_far - tuning.height_near) * landscape;
        let look_ahead = tuning.look_near + (tuning.look_far - tuning.look_near) * landscape;
        let eye = position + rotation * Vec3::new(0.0, height, back);
        let target = position + rotation * Vec3::new(0.0, tuning.target_height, look_ahead);
        self.renderer.set_camera(Camera {
            eye: [eye.x, eye.y.max(tuning.min_eye_y), eye.z],
            target: [target.x, target.y, target.z],
            fov_y: tuning.fov_y,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn shadow_decal_yaw_matches_car_heading() {
        assert!(shadow_decal_yaw(Vec3::NEG_Z).abs() < 1e-6);
        assert!((shadow_decal_yaw(Vec3::NEG_X) - FRAC_PI_2).abs() < 1e-6);
        assert!((shadow_decal_yaw(Vec3::X) + FRAC_PI_2).abs() < 1e-6);
        assert!((shadow_decal_yaw(Vec3::Z).abs() - std::f32::consts::PI).abs() < 1e-6);

        for angle in [-2.5, -1.0, 0.0, 0.7, 2.8] {
            let car = Quat::from_rotation_y(angle);
            let forward = car * Vec3::NEG_Z;
            let decal = Quat::from_rotation_y(shadow_decal_yaw(forward));
            assert!((decal * Vec3::NEG_Z - forward).length() < 1e-5);
            assert!((decal * Vec3::X - car * Vec3::X).length() < 1e-5);
        }
    }
}
