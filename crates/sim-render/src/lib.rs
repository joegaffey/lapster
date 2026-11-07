#![recursion_limit = "256"]

mod camera;
mod garage;
mod grass;
mod particles;
mod renderer;
mod scene;
mod track_editor;
mod tuning;
mod vegetation;

use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::particles::{ParticleInstance, ParticleSystem};
use crate::renderer::{CarShadowMode, EnvironmentShadowMode};
use crate::scene::{CarRig, CockpitNeedleSource, GarageBay, Scene, SceneTexture};
use crate::tuning::{GpuTuning, ParticleTuning, TuningConfig};
use glam::{Mat4, Quat, Vec3};
use sim_core::{
    Controls, PhysicsTuning, TrackCollider, Vehicle, VehicleState, WheelState, FIXED_DT,
};
use sim_types::snapshot_state as snapshot;
use sim_types::{CarTuning, SNAPSHOT_STATE_FLOATS};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

struct StyleCar {
    transform: Mat4,
    bounds: sim_types::Aabb,
}

struct StyleDrive {
    vehicle: Vehicle,
    rig: CarRig,
    previous_state: VehicleState,
    accumulator: f32,
    last_time: Option<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TreeDetail {
    High,
    Low,
    Cards,
}

impl TreeDetail {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "high" => Some(Self::High),
            "low" => Some(Self::Low),
            "cards" => Some(Self::Cards),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
            Self::Cards => "cards",
        }
    }
}

fn tree_mesh_indices(scene: &Scene) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut cards = Vec::new();
    for (index, mesh) in scene.meshes.iter().enumerate() {
        if !mesh.name.starts_with("tree_") {
            continue;
        }
        if mesh.name.starts_with("tree_cards") {
            cards.push(index);
        } else if mesh.name.ends_with("_high") {
            high.push(index);
        } else if mesh.name.ends_with("_low") {
            low.push(index);
        }
    }
    (high, low, cards)
}

fn set_tree_visibility(
    renderer: &mut renderer::Renderer,
    high: &[usize],
    low: &[usize],
    cards: &[usize],
    detail: TreeDetail,
) {
    for &index in high {
        renderer.set_mesh_visible(index, detail == TreeDetail::High);
    }
    for &index in low {
        renderer.set_mesh_visible(index, detail == TreeDetail::Low);
    }
    for &index in cards {
        renderer.set_mesh_visible(index, detail == TreeDetail::Cards);
    }
}

#[wasm_bindgen]
pub struct TrackViewer {
    renderer: renderer::Renderer,
    canvas: web_sys::HtmlCanvasElement,
    debug_info: String,
    initial_camera: Camera,
    camera: Camera,
    vegetation: Option<scene::VegetationGroups>,
    style_car: Option<StyleCar>,
    style_drive: Option<StyleDrive>,
    editor_control_points: Option<Vec<[f32; 3]>>,
    editor_kerb_spans: Vec<track_editor::KerbSpan>,
    editor_prop_lines: Vec<track_editor::PropLine>,
    tree_high_meshes: Vec<usize>,
    tree_low_meshes: Vec<usize>,
    tree_card_meshes: Vec<usize>,
    tree_detail: TreeDetail,
    track_blend: f32,
}

#[wasm_bindgen]
impl TrackViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        pack_bytes: js_sys::Uint8Array,
    ) -> Result<TrackViewer, JsValue> {
        Self::create_internal(canvas, pack_bytes, false, None).await
    }

    pub async fn create_style(
        canvas: web_sys::HtmlCanvasElement,
        pack_bytes: js_sys::Uint8Array,
    ) -> Result<TrackViewer, JsValue> {
        Self::create_internal(canvas, pack_bytes, true, None).await
    }

    pub async fn create_style_with_car(
        canvas: web_sys::HtmlCanvasElement,
        pack_bytes: js_sys::Uint8Array,
        car_pack_bytes: js_sys::Uint8Array,
    ) -> Result<TrackViewer, JsValue> {
        Self::create_internal(canvas, pack_bytes, true, Some(car_pack_bytes.to_vec())).await
    }

    pub async fn create_editor(canvas: web_sys::HtmlCanvasElement) -> Result<TrackViewer, JsValue> {
        let points = crate::track_editor::default_control_points();
        let track = crate::track_editor::track_from_control_points_with_spans(
            &points,
            Some(&crate::track_editor::default_kerb_spans()),
            None,
        )
        .map_err(js_error)?;
        let scene = Scene::build(
            &track.visual,
            &track.metadata,
            &track.collision,
            &track.manifest,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let mut viewer =
            Self::create_from_scene(canvas, scene, &track.metadata, &track.collision, None).await?;
        viewer.editor_control_points = Some(points);
        viewer.editor_kerb_spans = track.kerb_spans;
        viewer.editor_prop_lines = track.prop_lines;
        Ok(viewer)
    }

    async fn create_internal(
        canvas: web_sys::HtmlCanvasElement,
        pack_bytes: js_sys::Uint8Array,
        style_spike: bool,
        car_bytes: Option<Vec<u8>>,
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

        let car = if let Some(bytes) = car_bytes {
            let car_pack = sim_asset::Pack::from_bytes(&bytes).map_err(js_error)?;
            if car_pack.kind() != sim_types::AssetKind::Car {
                return Err(JsValue::from_str("expected a car pack"));
            }
            let visual = car_pack.car_visual_owned().map_err(js_error)?;
            let physics = car_pack.car_physics_owned().map_err(js_error)?;
            Some((visual, physics))
        } else {
            None
        };

        let scene = match (style_spike, car.as_ref()) {
            (true, Some((car_visual, _))) => Scene::build_style_spike_with_car(
                &visual, &metadata, &collision, &manifest, car_visual,
            ),
            (true, None) => Scene::build_style_spike(&visual, &metadata, &collision, &manifest),
            (false, _) => Scene::build(&visual, &metadata, &collision, &manifest),
        }
        .map_err(|error| JsValue::from_str(&error))?;
        let car_physics = car.map(|(_, physics)| physics);
        Self::create_from_scene(canvas, scene, &metadata, &collision, car_physics).await
    }

    async fn create_from_scene(
        canvas: web_sys::HtmlCanvasElement,
        scene: Scene,
        metadata: &sim_types::TrackMetadata,
        collision: &sim_types::TrackCollision,
        car_physics: Option<sim_types::CarPhysics>,
    ) -> Result<TrackViewer, JsValue> {
        let canvas_handle = canvas.clone();
        let debug_info = scene.debug_info.clone();
        let (tree_high_meshes, tree_low_meshes, tree_card_meshes) = tree_mesh_indices(&scene);
        let vegetation = scene.vegetation.clone();
        let style_car_transform = scene.style_car_transform;
        let style_car = style_car_transform
            .zip(scene.style_car_bounds)
            .map(|(transform, bounds)| StyleCar { transform, bounds });
        let style_rig = scene.car_rig.clone();
        let camera = scene.camera;
        let shadow_focus = camera.target;
        let (collision_center, region_half) = track_region(collision, metadata);
        let region_center = metadata
            .spawns
            .first()
            .map(|spawn| spawn.position)
            .unwrap_or(collision_center);
        let collider = TrackCollider::from_collision(collision, metadata);
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
        set_tree_visibility(
            &mut renderer,
            &tree_high_meshes,
            &tree_low_meshes,
            &tree_card_meshes,
            TreeDetail::High,
        );
        let style_drive = if let (Some(car_physics), Some(transform), Some(rig)) =
            (car_physics, style_car_transform, style_rig.as_ref())
        {
            let (_, rotation, translation) = transform.to_scale_rotation_translation();
            let spawn = sim_types::SpawnPoint {
                name: "style".to_string(),
                position: translation.to_array(),
                rotation_xyzw: rotation.to_array(),
            };
            let mut vehicle = Vehicle::new(car_physics, collider.clone(), &spawn);
            for _ in 0..500 {
                vehicle.step(Controls::default());
            }
            let previous_state = vehicle.state;
            let mut drive = StyleDrive {
                vehicle,
                rig: rig.clone(),
                previous_state,
                accumulator: 0.0,
                last_time: None,
            };
            update_style_car_rig(
                &mut renderer,
                &drive.rig,
                drive.vehicle.state.position,
                drive.vehicle.state.rotation,
                &drive.vehicle.state,
                0.0,
                false,
            );
            drive.previous_state = drive.vehicle.state;
            Some(drive)
        } else {
            None
        };
        Ok(TrackViewer {
            renderer,
            canvas: canvas_handle,
            debug_info,
            initial_camera: camera,
            camera,
            vegetation,
            style_car,
            style_drive,
            editor_control_points: None,
            editor_kerb_spans: Vec::new(),
            editor_prop_lines: Vec::new(),
            tree_high_meshes,
            tree_low_meshes,
            tree_card_meshes,
            tree_detail: TreeDetail::High,
            track_blend: 1.0,
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

    pub fn editor_control_points(&self) -> String {
        let points = self
            .editor_control_points
            .as_ref()
            .map(|points| crate::track_editor::control_points_with_heights(points))
            .unwrap_or_default();
        serde_json::to_string(&points).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn editor_centerline(&self) -> String {
        let points = self
            .editor_control_points
            .as_ref()
            .map(|points| crate::track_editor::centerline_points(points))
            .unwrap_or_default();
        serde_json::to_string(&points).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn editor_kerb_spans(&self) -> String {
        serde_json::to_string(&self.editor_kerb_spans).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn editor_prop_lines(&self) -> String {
        serde_json::to_string(&self.editor_prop_lines).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn set_track_blend_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.track_blend = match mode {
            "off" => 0.0,
            "dither" => 1.0,
            "soft" => 2.0,
            "hard" => 3.0,
            _ => return Err(JsValue::from_str("expected off, dither, soft, or hard")),
        };
        self.renderer.set_track_blend_mode(self.track_blend);
        Ok(())
    }

    pub fn track_blend_mode(&self) -> String {
        match self.renderer.track_blend_mode() {
            mode if mode > 2.5 => "hard".to_string(),
            mode if mode > 1.5 => "soft".to_string(),
            mode if mode > 0.5 => "dither".to_string(),
            _ => "off".to_string(),
        }
    }

    pub fn editor_track_pack(&self) -> Result<Vec<u8>, JsValue> {
        let points = self
            .editor_control_points
            .as_ref()
            .ok_or_else(|| JsValue::from_str("editor control points unavailable"))?;
        let track = crate::track_editor::track_from_control_points_with_spans(
            points,
            Some(&self.editor_kerb_spans),
            Some(&self.editor_prop_lines),
        )
        .map_err(js_error)?;
        crate::track_editor::pack_from_track(&track).map_err(|error| JsValue::from_str(&error))
    }

    pub async fn set_editor_control_points(&mut self, points_json: &str) -> Result<(), JsValue> {
        let points = serde_json::from_str::<Vec<[f32; 3]>>(points_json).map_err(js_error)?;
        let points = crate::track_editor::control_points_with_heights(&points);
        self.rebuild_editor(
            points,
            self.editor_kerb_spans.clone(),
            self.editor_prop_lines.clone(),
        )
        .await
    }

    pub async fn set_editor_kerb_spans(&mut self, spans_json: &str) -> Result<(), JsValue> {
        let spans = serde_json::from_str::<Vec<crate::track_editor::KerbSpan>>(spans_json)
            .map_err(js_error)?;
        let points = self
            .editor_control_points
            .clone()
            .ok_or_else(|| JsValue::from_str("editor control points unavailable"))?;
        self.rebuild_editor(points, spans, self.editor_prop_lines.clone())
            .await
    }

    pub async fn set_editor_prop_lines(&mut self, lines_json: &str) -> Result<(), JsValue> {
        let lines = serde_json::from_str::<Vec<crate::track_editor::PropLine>>(lines_json)
            .map_err(js_error)?;
        let points = self
            .editor_control_points
            .clone()
            .ok_or_else(|| JsValue::from_str("editor control points unavailable"))?;
        self.rebuild_editor(points, self.editor_kerb_spans.clone(), lines)
            .await
    }

    async fn rebuild_editor(
        &mut self,
        points: Vec<[f32; 3]>,
        kerb_spans: Vec<crate::track_editor::KerbSpan>,
        prop_lines: Vec<crate::track_editor::PropLine>,
    ) -> Result<(), JsValue> {
        let track = crate::track_editor::track_from_control_points_with_spans(
            &points,
            Some(&kerb_spans),
            Some(&prop_lines),
        )
        .map_err(js_error)?;
        let scene = Scene::build(
            &track.visual,
            &track.metadata,
            &track.collision,
            &track.manifest,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let debug_info = scene.debug_info.clone();
        let (tree_high_meshes, tree_low_meshes, tree_card_meshes) = tree_mesh_indices(&scene);
        let camera = self.camera;
        let (collision_center, region_half) = track_region(&track.collision, &track.metadata);
        let region_center = track
            .metadata
            .spawns
            .first()
            .map(|spawn| spawn.position)
            .unwrap_or(collision_center);
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let ground_y = collider
            .raycast(
                Vec3::new(region_center[0], region_center[1] + 100.0, region_center[2]),
                Vec3::NEG_Y,
                300.0,
            )
            .map(|hit| hit.point.y)
            .unwrap_or(region_center[1]);
        let width = self.canvas.width().max(1);
        let height = self.canvas.height().max(1);
        let mut renderer = renderer::Renderer::new(
            self.canvas.clone(),
            width,
            height,
            scene,
            &GpuTuning::default(),
        )
        .await?;
        renderer.set_lines_visible(false);
        renderer.set_baked_region(collision_center, region_half, ground_y);
        renderer.set_shadow_focus(camera.target, TRACK_SHADOW_EXTENT);
        renderer.set_grass_center_xz(camera.eye[0], camera.eye[2]);
        renderer.set_track_blend_mode(self.track_blend);
        renderer.set_camera(camera);
        set_tree_visibility(
            &mut renderer,
            &tree_high_meshes,
            &tree_low_meshes,
            &tree_card_meshes,
            self.tree_detail,
        );
        self.renderer = renderer;
        self.debug_info = debug_info;
        self.editor_control_points = Some(points);
        self.editor_kerb_spans = track.kerb_spans;
        self.editor_prop_lines = track.prop_lines;
        self.tree_high_meshes = tree_high_meshes;
        self.tree_low_meshes = tree_low_meshes;
        self.tree_card_meshes = tree_card_meshes;
        Ok(())
    }

    pub fn tree_detail(&self) -> String {
        self.tree_detail.as_str().to_string()
    }

    pub fn set_tree_detail(&mut self, detail: &str) -> Result<(), JsValue> {
        let next = TreeDetail::parse(detail)
            .ok_or_else(|| JsValue::from_str("expected high, low, or cards"))?;
        if next == self.tree_detail {
            return Ok(());
        }
        self.tree_detail = next;
        let tree_high_meshes = std::mem::take(&mut self.tree_high_meshes);
        let tree_low_meshes = std::mem::take(&mut self.tree_low_meshes);
        let tree_card_meshes = std::mem::take(&mut self.tree_card_meshes);
        set_tree_visibility(
            &mut self.renderer,
            &tree_high_meshes,
            &tree_low_meshes,
            &tree_card_meshes,
            next,
        );
        self.tree_high_meshes = tree_high_meshes;
        self.tree_low_meshes = tree_low_meshes;
        self.tree_card_meshes = tree_card_meshes;
        Ok(())
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

    pub fn set_grass_texture_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_texture_enabled(enabled);
    }

    pub fn set_grass_shader_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_shader_enabled(enabled);
    }

    pub fn set_vegetation_tier(&mut self, tier: &str) {
        let Some(groups) = &self.vegetation else {
            return;
        };
        let medium = tier != "low";
        let high = tier == "high";
        for &index in &groups.near {
            self.renderer.set_mesh_visible(index, true);
        }
        for &index in &groups.mid {
            self.renderer.set_mesh_visible(index, medium);
        }
        for &index in &groups.far {
            self.renderer.set_mesh_visible(index, high);
        }
    }

    pub fn style_camera_tuning(&self) -> String {
        serde_json::to_string(&TuningConfig::default().camera).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn set_camera_fov(&mut self, fov_y: f32) {
        self.camera.fov_y = fov_y.clamp(0.1, 3.0);
        self.renderer.set_camera(self.camera);
    }

    pub fn style_car_info(&self) -> String {
        let Some(car) = &self.style_car else {
            return "null".to_string();
        };
        let (position, rotation, speed_kph, rpm, gear) = if let Some(drive) = &self.style_drive {
            let state = &drive.vehicle.state;
            (
                state.position,
                state.rotation,
                state.velocity.length() * 3.6,
                state.engine_rpm,
                state.gear,
            )
        } else {
            let (_, rotation, translation) = car.transform.to_scale_rotation_translation();
            (translation, rotation, 0.0, 0.0, 0)
        };
        let forward = rotation * Vec3::NEG_Z;
        let right = rotation * Vec3::X;
        let up = rotation * Vec3::Y;
        serde_json::json!({
            "position": position.to_array(),
            "forward": forward.to_array(),
            "right": right.to_array(),
            "up": up.to_array(),
            "speedKph": speed_kph,
            "rpm": rpm,
            "gear": gear,
            "bounds": {
                "min": car.bounds.min,
                "max": car.bounds.max,
            },
        })
        .to_string()
    }

    pub fn update_car(
        &mut self,
        time_seconds: f64,
        throttle: f32,
        brake: f32,
        steering: f32,
        handbrake: f32,
        cockpit: bool,
    ) -> Result<(), JsValue> {
        let Some(drive) = &mut self.style_drive else {
            return Ok(());
        };
        let dt = match drive.last_time {
            Some(last) => ((time_seconds - last).max(0.0) as f32).min(0.05),
            None => 0.0,
        };
        drive.last_time = Some(time_seconds);
        drive.accumulator += dt;
        let controls = Controls {
            steering: steering.clamp(-1.0, 1.0),
            throttle: throttle.clamp(0.0, 1.0),
            brake: brake.clamp(0.0, 1.0),
            clutch: 0.0,
            gear: auto_gear(drive.vehicle.state.velocity.length()),
            handbrake: handbrake.clamp(0.0, 1.0),
        };
        let mut steps = 0;
        while drive.accumulator >= FIXED_DT && steps < 25 {
            drive.previous_state = drive.vehicle.state;
            drive.vehicle.step(controls);
            drive.accumulator -= FIXED_DT;
            steps += 1;
        }
        if steps == 25 && drive.accumulator > FIXED_DT {
            drive.accumulator = 0.0;
        }
        let alpha = (drive.accumulator / FIXED_DT).clamp(0.0, 1.0);
        let position = drive
            .previous_state
            .position
            .lerp(drive.vehicle.state.position, alpha);
        let rotation = drive
            .previous_state
            .rotation
            .slerp(drive.vehicle.state.rotation, alpha)
            .normalize();
        update_style_car_rig(
            &mut self.renderer,
            &drive.rig,
            position,
            rotation,
            &drive.vehicle.state,
            brake,
            cockpit,
        );
        self.renderer.set_shadow_focus(
            [position.x, position.y + 0.5, position.z],
            TRACK_SHADOW_EXTENT,
        );
        Ok(())
    }

    pub fn reset_car(&mut self) {
        let Some(drive) = &mut self.style_drive else {
            return;
        };
        drive.vehicle.reset();
        for _ in 0..500 {
            drive.vehicle.step(Controls::default());
        }
        drive.previous_state = drive.vehicle.state;
        drive.accumulator = 0.0;
        drive.last_time = None;
        let position = drive.vehicle.state.position;
        let rotation = drive.vehicle.state.rotation;
        update_style_car_rig(
            &mut self.renderer,
            &drive.rig,
            position,
            rotation,
            &drive.vehicle.state,
            0.0,
            false,
        );
        self.renderer.set_shadow_focus(
            [position.x, position.y + 0.5, position.z],
            TRACK_SHADOW_EXTENT,
        );
    }

    pub fn style_info(&self) -> String {
        serde_json::json!({
            "backend": self.renderer.backend(),
            "surfaceFormat": self.renderer.surface_format(),
            "vegetation": self.vegetation.as_ref().map(|groups| serde_json::json!({
                "trees": groups.trees,
                "bushes": groups.bushes,
                "candidates": groups.candidates,
                "nearMeshes": groups.near.len(),
                "midMeshes": groups.mid.len(),
                "farMeshes": groups.far.len(),
            })),
            "mass": {
                "trees": {
                    "maxLayers": self.renderer.tree_mass_max_layers(),
                    "instances": self.renderer.tree_mass_instance_count(),
                    "activeInstances": self.renderer.tree_mass_active_instances(),
                },
                "bushes": {
                    "maxLayers": self.renderer.bush_mass_max_layers(),
                    "instances": self.renderer.bush_mass_instance_count(),
                    "activeInstances": self.renderer.bush_mass_active_instances(),
                },
            },
        })
        .to_string()
    }

    pub fn set_tree_mass(&mut self, layers: u32) -> u32 {
        self.renderer.set_tree_mass(layers)
    }

    pub fn set_bush_mass(&mut self, layers: u32) -> u32 {
        self.renderer.set_bush_mass(layers)
    }

    pub fn set_vegetation_mass(&mut self, layers: u32) -> u32 {
        self.renderer.set_vegetation_mass(layers)
    }

    pub fn set_grass_density(&mut self, density: f32) {
        self.renderer.set_grass_density(density);
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
        self.renderer.set_grass_center_xz(eye_x, eye_z);
        self.renderer
            .set_shadow_focus([target_x, target_y, target_z], TRACK_SHADOW_EXTENT);
    }

    pub fn reset_camera(&mut self) {
        self.camera = self.initial_camera;
        self.renderer.set_camera(self.camera);
        self.renderer
            .set_grass_center_xz(self.camera.eye[0], self.camera.eye[2]);
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
        car_packs: js_sys::Array,
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

        if car_packs.length() == 0 {
            return Err(JsValue::from_str("expected at least one car pack"));
        }
        let mut car_data = Vec::with_capacity(car_packs.length() as usize);
        for value in car_packs.iter() {
            let car_bytes = js_sys::Uint8Array::new(&value).to_vec();
            let car_pack = sim_asset::Pack::from_bytes(&car_bytes).map_err(js_error)?;
            if car_pack.kind() != sim_types::AssetKind::Car {
                return Err(JsValue::from_str(
                    "expected car packs for the second argument",
                ));
            }
            let car_physics = car_pack.car_physics_owned().map_err(js_error)?;
            let car_visual = car_pack.car_visual_owned().map_err(js_error)?;
            let car_manifest = car_pack.manifest().clone();
            car_data.push((car_physics, car_visual, car_manifest));
        }
        let cars = car_data
            .iter()
            .map(|(physics, visual, manifest)| garage::GarageCar {
                physics,
                visual,
                manifest,
            })
            .collect::<Vec<_>>();

        let mut scene = Scene::build_garage(
            &track_visual,
            &track_metadata,
            &track_collision,
            &track_manifest,
            &cars,
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
            "carId": bay.map(|bay| bay.car_id.clone()).unwrap_or_default(),
            "car": bay.map(|bay| bay.car.clone()),
            "color": bay.map(|bay| bay.color),
            "colorName": bay.map(|bay| bay.color_name).unwrap_or_default(),
        })
        .to_string()
    }

    pub fn bay_list(&self) -> String {
        serde_json::to_string(
            &self
                .bays
                .iter()
                .enumerate()
                .map(|(index, bay)| {
                    serde_json::json!({
                        "index": index,
                        "name": bay.name,
                        "carId": bay.car_id,
                        "color": bay.color,
                        "colorName": bay.color_name,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string())
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

    pub fn set_grass_texture_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_texture_enabled(enabled);
    }

    pub fn set_grass_shader_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_shader_enabled(enabled);
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

fn state_from_frame(frame: &[f32]) -> Result<VehicleState, JsValue> {
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
    let mut wheels = [WheelState::default(); 4];
    let surface_ids = frame[snapshot::SURFACE_IDS].max(0.0) as u32;
    for (index, wheel) in wheels.iter_mut().enumerate() {
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
    Ok(VehicleState {
        position,
        rotation,
        velocity: Vec3::new(
            frame[snapshot::VELOCITY],
            frame[snapshot::VELOCITY + 1],
            frame[snapshot::VELOCITY + 2],
        ),
        angular_velocity: Vec3::new(
            frame[snapshot::ANGULAR_VELOCITY],
            frame[snapshot::ANGULAR_VELOCITY + 1],
            frame[snapshot::ANGULAR_VELOCITY + 2],
        ),
        wheels,
        engine_rpm: frame[snapshot::ENGINE_RPM],
        gear: frame[snapshot::GEAR].round() as i32,
        steering_rack_torque: frame[snapshot::RACK_TORQUE],
        tick: frame[snapshot::TICK].max(0.0) as u64,
    })
}

fn remote_particles_tuning(base: &ParticleTuning) -> ParticleTuning {
    let mut tuning = base.clone();
    tuning.max_active = tuning.max_active.min(160);
    tuning.max_spawn_per_step = tuning.max_spawn_per_step.min(4);
    tuning
}

fn set_rig_cockpit_visible(renderer: &mut renderer::Renderer, rig: &CarRig, visible: bool) {
    if let Some(index) = rig.dashboard_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    if let Some(index) = rig.speedometer_face_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    if let Some(index) = rig.rpm_face_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    for needle in &rig.cockpit_needles {
        renderer.set_mesh_visible(needle.mesh_index, visible);
    }
    for cover in &rig.cockpit_covers {
        renderer.set_mesh_visible(cover.mesh_index, visible);
    }
    for segment in &rig.gear_segments {
        renderer.set_mesh_visible(segment.mesh_index, visible);
    }
    if let Some(index) = rig.steering_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
}

fn set_rig_exterior_visible(renderer: &mut renderer::Renderer, rig: &CarRig, visible: bool) {
    renderer.set_mesh_visible(rig.body_mesh_index, visible);
    for index in &rig.extra_body_meshes {
        renderer.set_mesh_visible(*index, visible);
    }
    if let Some(index) = rig.glass_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    if let Some(index) = rig.brake_light_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    if let Some(index) = rig.reverse_light_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    for wheel in &rig.wheels {
        renderer.set_mesh_visible(wheel.mesh_index, visible);
    }
    if let Some(index) = rig.decal_mesh_index {
        renderer.set_mesh_visible(index, visible);
    }
    if !visible {
        if let Some(index) = rig.dashboard_mesh_index {
            renderer.set_mesh_visible(index, false);
        }
        if let Some(index) = rig.speedometer_face_mesh_index {
            renderer.set_mesh_visible(index, false);
        }
        if let Some(index) = rig.rpm_face_mesh_index {
            renderer.set_mesh_visible(index, false);
        }
        for needle in &rig.cockpit_needles {
            renderer.set_mesh_visible(needle.mesh_index, false);
        }
        for cover in &rig.cockpit_covers {
            renderer.set_mesh_visible(cover.mesh_index, false);
        }
        for segment in &rig.gear_segments {
            renderer.set_mesh_visible(segment.mesh_index, false);
        }
        if let Some(index) = rig.steering_mesh_index {
            renderer.set_mesh_visible(index, false);
        }
    }
}

fn update_remote_rig(
    renderer: &mut renderer::Renderer,
    rig: &CarRig,
    state: &VehicleState,
    brake: f32,
    _cockpit: bool,
) {
    let car_world = Mat4::from_rotation_translation(state.rotation, state.position);
    renderer.set_mesh_model(rig.body_mesh_index, car_world * rig.body_local);
    for index in &rig.extra_body_meshes {
        renderer.set_mesh_model(*index, car_world * rig.body_local);
    }
    if let Some(index) = rig.glass_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
    }
    if let Some(index) = rig.brake_light_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
        renderer.set_mesh_emissive(
            index,
            if brake > 0.05 {
                [5.0, 0.12, 0.02]
            } else {
                [0.0; 3]
            },
        );
    }
    if let Some(index) = rig.reverse_light_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
        renderer.set_mesh_emissive(
            index,
            if state.gear == -1 {
                [3.5, 3.5, 3.5]
            } else {
                [0.0; 3]
            },
        );
    }
    for (index, wheel) in rig.wheels.iter().enumerate() {
        let wheel_state = state.wheels.get(index).copied().unwrap_or_default();
        let spin = Mat4::from_rotation_x(-wheel_state.spin_angle);
        let rotation = if wheel.is_front {
            Mat4::from_rotation_y(wheel_state.steer_angle) * spin
        } else {
            spin
        };
        renderer.set_mesh_model(wheel.mesh_index, car_world * wheel.local_model * rotation);
    }
    if let Some(index) = rig.decal_mesh_index {
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
            state.position
        };
        let normal = if normal.length_squared() > 0.001 {
            normal.normalize()
        } else {
            Vec3::Y
        };
        let light = Vec3::from_array(renderer.sun_direction());
        let yaw = shadow_decal_yaw(state.rotation * Vec3::NEG_Z);
        renderer.set_shadow_decal(
            index,
            base.to_array(),
            normal.to_array(),
            yaw,
            rig.decal_width,
            rig.decal_length,
            light.to_array(),
            0.7,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn update_style_car_rig(
    renderer: &mut renderer::Renderer,
    rig: &CarRig,
    position: Vec3,
    rotation: Quat,
    state: &VehicleState,
    brake: f32,
    cockpit: bool,
) {
    let car_world = Mat4::from_rotation_translation(rotation, position);
    renderer.set_mesh_model(rig.body_mesh_index, car_world * rig.body_local);
    for index in &rig.extra_body_meshes {
        renderer.set_mesh_model(*index, car_world * rig.body_local);
    }
    if let Some(index) = rig.glass_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
    }
    if let Some(index) = rig.dashboard_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.dashboard_local);
    }
    if let Some(index) = rig.speedometer_face_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.dashboard_local);
    }
    if let Some(index) = rig.rpm_face_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.dashboard_local);
    }
    for cover in &rig.cockpit_covers {
        renderer.set_mesh_model(cover.mesh_index, car_world * rig.dashboard_local);
    }
    let speed_kph = state.velocity.length() * 3.6;
    let gear_fraction = ((state.gear.max(1) - 1) as f32 / 5.0).clamp(0.0, 1.0);
    for needle in &rig.cockpit_needles {
        let fraction = match needle.source {
            CockpitNeedleSource::Speed => (speed_kph / 330.0).clamp(0.0, 1.0),
            CockpitNeedleSource::Rpm => (state.engine_rpm / 10_000.0).clamp(0.0, 1.0),
            CockpitNeedleSource::Gear => gear_fraction,
        };
        let angle = 135.0_f32.to_radians() - fraction * 270.0_f32.to_radians();
        let model = car_world
            * rig.dashboard_local
            * Mat4::from_translation(needle.center)
            * Mat4::from_rotation_z(angle);
        renderer.set_mesh_model(needle.mesh_index, model);
    }
    for segment in &rig.gear_segments {
        let model = car_world * rig.dashboard_local * Mat4::from_translation(segment.center);
        renderer.set_mesh_model(segment.mesh_index, model);
        renderer.set_mesh_visible(segment.mesh_index, cockpit && state.gear == segment.gear);
    }
    if let Some(index) = rig.brake_light_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
        renderer.set_mesh_emissive(
            index,
            if brake > 0.05 {
                [5.0, 0.12, 0.02]
            } else {
                [0.0; 3]
            },
        );
    }
    if let Some(index) = rig.reverse_light_mesh_index {
        renderer.set_mesh_model(index, car_world * rig.body_local);
        renderer.set_mesh_emissive(
            index,
            if state.gear == -1 {
                [4.0, 4.0, 4.0]
            } else {
                [0.0; 3]
            },
        );
    }
    for (index, wheel) in rig.wheels.iter().enumerate() {
        let wheel_state = state.wheels.get(index).copied().unwrap_or_default();
        let spin = Mat4::from_rotation_x(-wheel_state.spin_angle);
        let wheel_rotation = if wheel.is_front {
            Mat4::from_rotation_y(wheel_state.steer_angle) * spin
        } else {
            spin
        };
        renderer.set_mesh_model(
            wheel.mesh_index,
            car_world * wheel.local_model * wheel_rotation,
        );
    }
    if let Some(index) = rig.steering_mesh_index {
        let steer = state
            .wheels
            .first()
            .map(|wheel| wheel.steer_angle)
            .unwrap_or(0.0);
        renderer.set_mesh_model(
            index,
            car_world * rig.steering_local * Mat4::from_rotation_x(-steer * 3.0),
        );
    }
}

fn auto_gear(speed_mps: f32) -> i32 {
    let kph = speed_mps * 3.6;
    if kph < 25.0 {
        1
    } else if kph < 50.0 {
        2
    } else if kph < 80.0 {
        3
    } else if kph < 115.0 {
        4
    } else if kph < 160.0 {
        5
    } else {
        6
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
    rigs: Vec<CarRig>,
    local_slot: usize,
    remote_visible: Vec<bool>,
    remote_states: Vec<Option<VehicleState>>,
    info: String,
    accumulator: f32,
    last_time: Option<f64>,
    step_times: Vec<f32>,
    track_id: String,
    car_id: String,
    car_bay: usize,
    color_offset: usize,
    camera_mode: CameraMode,
    brake_lights_on: bool,
    reverse_lights_on: bool,
    surface_names: Vec<String>,
    particles: ParticleSystem,
    remote_particles: Vec<ParticleSystem>,
    particle_instances: Vec<ParticleInstance>,
    remote_particle_tuning: ParticleTuning,
    last_particle_time: Option<f64>,
    tuning: TuningConfig,
    cockpit_dial_offsets: [[f32; 3]; 3],
    track_shadow_center: Vec3,
    track_shadow_half: f32,
    tree_high_meshes: Vec<usize>,
    tree_low_meshes: Vec<usize>,
    tree_card_meshes: Vec<usize>,
    tree_detail: TreeDetail,
}

#[wasm_bindgen]
impl DriveViewer {
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        track_pack_bytes: js_sys::Uint8Array,
        car_pack_bytes: js_sys::Uint8Array,
        car_bay: Option<usize>,
        tuning_json: Option<String>,
        remote_slots: Option<usize>,
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

        if car_visual
            .meshes
            .iter()
            .any(|mesh| mesh.name == "body_interior")
        {
            tuning.camera.cockpit.eye = [-0.4, 1.10, 0.1];
            tuning.camera.cockpit.target = [-0.4, 1.00, -20.3];
            tuning.camera.cockpit.fov_y = 1.0;
        }

        let car_bay = car_bay.unwrap_or(0) % garage::BAY_COLOR_NAMES.len();
        let remote_slots = remote_slots.unwrap_or(0).min(7);
        let cars = [garage::GarageCar {
            physics: &car_physics,
            visual: &car_visual,
            manifest: &car_manifest,
        }];
        let remote_particle_tuning = remote_particles_tuning(&tuning.particles);
        let color_offset = if remote_slots == 0 { car_bay } else { 0 };
        let mut scene = Scene::build_garage(
            &track_visual,
            &track_metadata,
            &track_collision,
            &track_manifest,
            &cars,
            1 + remote_slots,
            color_offset,
            false,
            true,
        )
        .map_err(|error| JsValue::from_str(&error))?;
        let mut rigs = std::mem::take(&mut scene.car_rigs);
        if rigs.len() < 1 + remote_slots {
            return Err(JsValue::from_str("scene did not build enough car rigs"));
        }
        rigs.truncate(1 + remote_slots);
        let (tree_high_meshes, tree_low_meshes, tree_card_meshes) = tree_mesh_indices(&scene);
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
        set_tree_visibility(
            &mut renderer,
            &tree_high_meshes,
            &tree_low_meshes,
            &tree_card_meshes,
            TreeDetail::High,
        );

        let previous_state = vehicle.state;
        let mut drive = DriveViewer {
            renderer,
            vehicle,
            previous_state,
            rigs,
            local_slot: 0,
            remote_visible: vec![false; 1 + remote_slots],
            remote_states: vec![None; 1 + remote_slots],
            info,
            accumulator: 0.0,
            last_time: None,
            step_times: Vec::with_capacity(256),
            track_id: track_manifest.id,
            car_id: car_manifest.id,
            car_bay,
            color_offset,
            camera_mode: CameraMode::Chase,
            brake_lights_on: false,
            reverse_lights_on: false,
            surface_names,
            particles: ParticleSystem::new(),
            remote_particles: (0..=remote_slots).map(|_| ParticleSystem::new()).collect(),
            particle_instances: Vec::with_capacity(crate::particles::MAX_PARTICLES),
            remote_particle_tuning,
            last_particle_time: None,
            tuning,
            cockpit_dial_offsets: [[0.0; 3]; 3],
            track_shadow_center: Vec3::from_array(track_shadow_center),
            track_shadow_half,
            tree_high_meshes,
            tree_low_meshes,
            tree_card_meshes,
            tree_detail: TreeDetail::High,
        };
        drive.renderer.set_shadow_tuning(&drive.tuning.shadows);
        drive
            .renderer
            .set_particle_render_tuning(&drive.tuning.particles.render);
        for slot in 1..=remote_slots {
            let _ = drive.set_remote_visible(slot, false);
        }
        drive.write_initial_pose()?;
        Ok(drive)
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.renderer.resize(width, height);
    }

    pub fn tree_detail(&self) -> String {
        self.tree_detail.as_str().to_string()
    }

    pub fn set_tree_detail(&mut self, detail: &str) -> Result<(), JsValue> {
        let next = TreeDetail::parse(detail)
            .ok_or_else(|| JsValue::from_str("expected high, low, or cards"))?;
        if next == self.tree_detail {
            return Ok(());
        }
        self.tree_detail = next;
        let tree_high_meshes = std::mem::take(&mut self.tree_high_meshes);
        let tree_low_meshes = std::mem::take(&mut self.tree_low_meshes);
        let tree_card_meshes = std::mem::take(&mut self.tree_card_meshes);
        set_tree_visibility(
            &mut self.renderer,
            &tree_high_meshes,
            &tree_low_meshes,
            &tree_card_meshes,
            next,
        );
        self.tree_high_meshes = tree_high_meshes;
        self.tree_low_meshes = tree_low_meshes;
        self.tree_card_meshes = tree_card_meshes;
        Ok(())
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
        self.update_with_clutch(time_seconds, throttle, brake, steering, gear, 0.0, 0.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_with_clutch(
        &mut self,
        time_seconds: f64,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
        clutch: f32,
        handbrake: f32,
    ) -> Result<(), JsValue> {
        let dt = match self.last_time {
            Some(last) => ((time_seconds - last).max(0.0) as f32).min(0.05),
            None => 0.0,
        };
        self.last_time = Some(time_seconds);
        self.accumulator += dt;
        self.set_brake_lights(brake.max(handbrake));
        self.set_reverse_lights(gear == -1);

        let controls = Controls {
            steering,
            throttle,
            brake,
            clutch,
            gear,
            handbrake,
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
        self.simulate_with_clutch(seconds, throttle, brake, steering, gear, 0.0, 0.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn simulate_with_clutch(
        &mut self,
        seconds: f32,
        throttle: f32,
        brake: f32,
        steering: f32,
        gear: i32,
        clutch: f32,
        handbrake: f32,
    ) -> u32 {
        let steps = ((seconds.clamp(0.0, 120.0) / FIXED_DT) as u32).min(60_000);
        self.set_brake_lights(brake.max(handbrake));
        self.set_reverse_lights(gear == -1);
        let controls = Controls {
            steering,
            throttle,
            brake,
            clutch,
            gear,
            handbrake,
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
        let remote = state_from_frame(frame)?;
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

    pub fn set_remote_visible(&mut self, slot: usize, visible: bool) -> Result<(), JsValue> {
        if slot == self.local_slot {
            return Ok(());
        }
        let Some(rig) = self.rigs.get(slot) else {
            return Err(JsValue::from_str("remote slot is out of range"));
        };
        set_rig_exterior_visible(&mut self.renderer, rig, visible);
        self.remote_visible[slot] = visible;
        if !visible {
            self.remote_states[slot] = None;
            self.remote_particles[slot].clear();
        }
        Ok(())
    }

    pub fn set_remote_state(&mut self, slot: usize, frame: &[f32]) -> Result<(), JsValue> {
        if slot == self.local_slot {
            return Ok(());
        }
        if slot >= self.rigs.len() {
            return Err(JsValue::from_str("remote slot is out of range"));
        }
        let remote = state_from_frame(frame)?;
        let brake = frame[snapshot::BRAKE];
        let was_hidden = !self.remote_visible[slot];
        self.remote_visible[slot] = true;
        self.remote_states[slot] = Some(remote);
        if was_hidden {
            set_rig_exterior_visible(&mut self.renderer, &self.rigs[slot], true);
        }
        update_remote_rig(
            &mut self.renderer,
            &self.rigs[slot],
            &remote,
            brake,
            self.camera_mode == CameraMode::Cockpit,
        );
        Ok(())
    }

    pub fn place_on_grid(&mut self, slot: usize) -> Result<(), JsValue> {
        self.vehicle.place_on_grid(slot);
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
        self.renderer.render()
    }

    pub fn set_local_slot(&mut self, slot: usize) -> Result<(), JsValue> {
        if slot >= self.rigs.len() {
            return Err(JsValue::from_str("local slot is out of range"));
        }
        if slot == self.local_slot {
            return Ok(());
        }
        let old = self.local_slot;
        self.local_slot = slot;
        self.remote_visible[old] = false;
        self.remote_states[old] = None;
        self.remote_visible[slot] = true;
        self.remote_states[slot] = None;
        self.remote_particles[old].clear();
        self.remote_particles[slot].clear();
        set_rig_exterior_visible(&mut self.renderer, &self.rigs[old], false);
        set_rig_exterior_visible(&mut self.renderer, &self.rigs[slot], true);
        set_rig_cockpit_visible(&mut self.renderer, &self.rigs[slot], true);
        self.previous_state = self.vehicle.state;
        self.update_visuals();
        self.update_camera();
        self.renderer.render()
    }

    pub fn debug_info(&self) -> String {
        self.info.clone()
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
            "localSlot": self.local_slot,
            "remoteSlots": self.rigs.len().saturating_sub(1),
            "remoteCars": self.remote_states.iter().enumerate().filter_map(|(slot, state)| {
                if slot == self.local_slot { return None; }
                state.map(|state| serde_json::json!({
                    "slot": slot,
                    "tick": state.tick,
                    "gear": state.gear,
                    "position": [state.position.x, state.position.y, state.position.z],
                }))
            }).collect::<Vec<_>>(),
            "carColor": garage::BAY_COLOR_NAMES
                [(self.color_offset + self.local_slot) % garage::BAY_COLOR_NAMES.len()],
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
        if let Some(index) = self.rigs[self.local_slot].dashboard_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        if let Some(index) = self.rigs[self.local_slot].speedometer_face_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        if let Some(index) = self.rigs[self.local_slot].rpm_face_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        for needle in &self.rigs[self.local_slot].cockpit_needles {
            self.renderer.set_mesh_visible(needle.mesh_index, visible);
        }
        for cover in &self.rigs[self.local_slot].cockpit_covers {
            self.renderer.set_mesh_visible(cover.mesh_index, visible);
        }
        for segment in &self.rigs[self.local_slot].gear_segments {
            self.renderer.set_mesh_visible(
                segment.mesh_index,
                visible && self.vehicle.state.gear == segment.gear,
            );
        }
    }

    fn apply_cockpit_glass_alpha(&mut self) {
        if let Some(index) = self.rigs[self.local_slot].glass_mesh_index {
            let alpha = if self.camera_mode == CameraMode::Cockpit {
                self.tuning.camera.cockpit_glass_alpha
            } else {
                1.0
            };
            self.renderer.set_mesh_alpha(index, alpha);
        }
    }

    pub fn set_cockpit_dial_offset(&mut self, source: &str, x: f32, y: f32, z: f32) {
        let index = match source.to_ascii_lowercase().as_str() {
            "speed" => 0,
            "rpm" => 1,
            "gear" => 2,
            _ => return,
        };
        self.cockpit_dial_offsets[index] = [x, y, z];
        self.update_visuals();
    }

    pub fn cockpit_dial_offsets(&self) -> String {
        serde_json::json!({
            "speed": self.cockpit_dial_offsets[0],
            "rpm": self.cockpit_dial_offsets[1],
            "gear": self.cockpit_dial_offsets[2],
        })
        .to_string()
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

    pub fn set_grass_texture_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_texture_enabled(enabled);
    }

    pub fn set_grass_shader_enabled(&mut self, enabled: bool) {
        self.renderer.set_grass_shader_enabled(enabled);
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

    pub fn set_speedometer_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        let index = self.rigs[self.local_slot].speedometer_face_mesh_index;
        self.set_dial_face_texture(index, width, height, rgba)
    }

    pub fn set_rpm_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        let index = self.rigs[self.local_slot].rpm_face_mesh_index;
        self.set_dial_face_texture(index, width, height, rgba)
    }

    fn set_dial_face_texture(
        &mut self,
        index: Option<usize>,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize {
            return Err(JsValue::from_str("invalid dial texture"));
        }
        if let Some(index) = index {
            let texture = SceneTexture {
                width,
                height,
                rgba8: rgba.to_vec(),
            };
            self.renderer.set_mesh_texture(index, &texture);
            self.renderer.render()?;
        }
        Ok(())
    }

    pub fn set_gear_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize {
            return Err(JsValue::from_str("invalid gear texture"));
        }
        let texture = SceneTexture {
            width,
            height,
            rgba8: rgba.to_vec(),
        };
        let indexes: Vec<usize> = self.rigs[self.local_slot]
            .gear_segments
            .iter()
            .map(|segment| segment.mesh_index)
            .collect();
        for index in indexes {
            self.renderer.set_mesh_texture(index, &texture);
        }
        self.renderer.render()
    }

    pub fn set_body_visible(&mut self, visible: bool) {
        self.renderer
            .set_mesh_visible(self.rigs[self.local_slot].body_mesh_index, visible);
        for index in &self.rigs[self.local_slot].extra_body_meshes {
            self.renderer.set_mesh_visible(*index, visible);
        }
        if let Some(index) = self.rigs[self.local_slot].glass_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        if let Some(index) = self.rigs[self.local_slot].brake_light_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
        if let Some(index) = self.rigs[self.local_slot].reverse_light_mesh_index {
            self.renderer.set_mesh_visible(index, visible);
        }
    }

    fn set_brake_lights(&mut self, brake: f32) {
        let on = brake > 0.05;
        if on == self.brake_lights_on {
            return;
        }
        self.brake_lights_on = on;
        if let Some(index) = self.rigs[self.local_slot].brake_light_mesh_index {
            self.renderer
                .set_mesh_emissive(index, if on { [5.0, 0.12, 0.02] } else { [0.0; 3] });
        }
    }

    fn set_reverse_lights(&mut self, on: bool) {
        if on == self.reverse_lights_on {
            return;
        }
        self.reverse_lights_on = on;
        if let Some(index) = self.rigs[self.local_slot].reverse_light_mesh_index {
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
        self.remote_particle_tuning = remote_particles_tuning(&self.tuning.particles);
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
        self.remote_particle_tuning = remote_particles_tuning(&self.tuning.particles);
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
        self.renderer.set_mesh_model(
            self.rigs[self.local_slot].body_mesh_index,
            car_world * self.rigs[self.local_slot].body_local,
        );
        for index in &self.rigs[self.local_slot].extra_body_meshes {
            self.renderer
                .set_mesh_model(*index, car_world * self.rigs[self.local_slot].body_local);
        }
        if let Some(index) = self.rigs[self.local_slot].dashboard_mesh_index {
            self.renderer.set_mesh_model(
                index,
                car_world * self.rigs[self.local_slot].dashboard_local,
            );
        }
        let speed_offset = Mat4::from_translation(Vec3::from_array(self.cockpit_dial_offsets[0]));
        let rpm_offset = Mat4::from_translation(Vec3::from_array(self.cockpit_dial_offsets[1]));
        let gear_offset = Mat4::from_translation(Vec3::from_array(self.cockpit_dial_offsets[2]));
        for cover in &self.rigs[self.local_slot].cockpit_covers {
            let offset = match cover.source {
                CockpitNeedleSource::Speed => speed_offset,
                CockpitNeedleSource::Rpm => rpm_offset,
                CockpitNeedleSource::Gear => gear_offset,
            };
            self.renderer.set_mesh_model(
                cover.mesh_index,
                car_world * self.rigs[self.local_slot].dashboard_local * offset,
            );
        }
        if let Some(index) = self.rigs[self.local_slot].speedometer_face_mesh_index {
            self.renderer.set_mesh_model(
                index,
                car_world * self.rigs[self.local_slot].dashboard_local * speed_offset,
            );
        }
        if let Some(index) = self.rigs[self.local_slot].rpm_face_mesh_index {
            self.renderer.set_mesh_model(
                index,
                car_world * self.rigs[self.local_slot].dashboard_local * rpm_offset,
            );
        }
        let speed_kph = state.velocity.length() * 3.6;
        let gear_fraction = ((state.gear.max(1) - 1) as f32 / 5.0).clamp(0.0, 1.0);
        for needle in &self.rigs[self.local_slot].cockpit_needles {
            let fraction = match needle.source {
                CockpitNeedleSource::Speed => (speed_kph / 330.0).clamp(0.0, 1.0),
                CockpitNeedleSource::Rpm => (state.engine_rpm / 10_000.0).clamp(0.0, 1.0),
                CockpitNeedleSource::Gear => gear_fraction,
            };
            let start = 135.0_f32.to_radians();
            let sweep = 270.0_f32.to_radians();
            let angle = start - fraction * sweep;
            let offset = match needle.source {
                CockpitNeedleSource::Speed => speed_offset,
                CockpitNeedleSource::Rpm => rpm_offset,
                CockpitNeedleSource::Gear => gear_offset,
            };
            let model = car_world
                * self.rigs[self.local_slot].dashboard_local
                * offset
                * Mat4::from_translation(needle.center)
                * Mat4::from_rotation_z(angle);
            self.renderer.set_mesh_model(needle.mesh_index, model);
        }
        for segment in &self.rigs[self.local_slot].gear_segments {
            let model = car_world
                * self.rigs[self.local_slot].dashboard_local
                * gear_offset
                * Mat4::from_translation(segment.center);
            self.renderer.set_mesh_model(segment.mesh_index, model);
            self.renderer.set_mesh_visible(
                segment.mesh_index,
                self.camera_mode == CameraMode::Cockpit && state.gear == segment.gear,
            );
        }
        if let Some(index) = self.rigs[self.local_slot].glass_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rigs[self.local_slot].body_local);
        }
        if let Some(index) = self.rigs[self.local_slot].brake_light_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rigs[self.local_slot].body_local);
        }
        if let Some(index) = self.rigs[self.local_slot].reverse_light_mesh_index {
            self.renderer
                .set_mesh_model(index, car_world * self.rigs[self.local_slot].body_local);
        }
        for (index, wheel) in self.rigs[self.local_slot].wheels.iter().enumerate() {
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
        if let Some(index) = self.rigs[self.local_slot].steering_mesh_index {
            let steer = state.wheels[0].steer_angle;
            let model = car_world
                * self.rigs[self.local_slot].steering_local
                * Mat4::from_rotation_x(-steer * 3.0);
            self.renderer.set_mesh_model(index, model);
        }
        if let Some(index) = self.rigs[self.local_slot].decal_mesh_index {
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
                self.rigs[self.local_slot].decal_width,
                self.rigs[self.local_slot].decal_length,
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
            &self.rigs[self.local_slot],
            &self.surface_names,
            &self.tuning.particles,
        );
        for slot in 0..self.rigs.len() {
            if slot == self.local_slot {
                continue;
            }
            let Some(remote_state) = self.remote_states[slot] else {
                self.remote_particles[slot].clear();
                continue;
            };
            let remote_world =
                Mat4::from_rotation_translation(remote_state.rotation, remote_state.position);
            self.remote_particles[slot].update(
                particle_dt,
                remote_world,
                &remote_state,
                &self.rigs[slot],
                &self.surface_names,
                &self.remote_particle_tuning,
            );
        }
        self.particle_instances.clear();
        self.particle_instances
            .extend_from_slice(self.particles.instances());
        for slot in 0..self.rigs.len() {
            if slot == self.local_slot {
                continue;
            }
            self.particle_instances
                .extend_from_slice(self.remote_particles[slot].instances());
        }
        self.renderer.set_particles(&self.particle_instances);
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
            self.renderer.set_grass_center_xz(eye.x, eye.z);
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
            self.renderer.set_grass_center_xz(eye.x, eye.z);
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
            let eye = focus + offset;
            self.renderer.set_camera(Camera {
                eye: eye.to_array(),
                target: focus.to_array(),
                fov_y: tuning.fov_y,
            });
            self.renderer.set_grass_center_xz(eye.x, eye.z);
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
        self.renderer.set_grass_center_xz(eye.x, eye.z);
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
