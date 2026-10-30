use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::particles::MAX_PARTICLES;
use sim_core::PhysicsTuning;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TuningConfig {
    pub camera: CameraTuning,
    pub particles: ParticleTuning,
    pub shadows: ShadowTuning,
    pub gpu: GpuTuning,
    pub physics: PhysicsTuning,
    pub audio: AudioTuning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CameraTuning {
    pub bonnet: CameraShot,
    pub cockpit: CameraShot,
    pub isometric: IsometricTuning,
    pub chase: ChaseTuning,
    pub cockpit_glass_alpha: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CameraShot {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub fov_y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IsometricTuning {
    pub offset: [f32; 3],
    pub focus_height: f32,
    pub fov_y: f32,
    pub fit_aspect_floor: f32,
    pub fit_power: f32,
    pub fit_min: f32,
    pub fit_max: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChaseTuning {
    pub aspect_near: f32,
    pub aspect_far: f32,
    pub back_near: f32,
    pub back_far: f32,
    pub height_near: f32,
    pub height_far: f32,
    pub look_near: f32,
    pub look_far: f32,
    pub target_height: f32,
    pub fov_y: f32,
    pub min_eye_y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioTuning {
    pub master_gain: f32,
    pub tire_gain: f32,
    pub transmission_mix: f32,
    pub rpm_tau: f32,
    pub load_attack: f32,
    pub load_release: f32,
    pub max_rpm_slew: f32,
    pub max_audio_rpm: f32,
    pub tire_lateral_threshold: f32,
    pub tire_longitudinal_threshold: f32,
    pub tire_lateral_scale: f32,
    pub tire_longitudinal_scale: f32,
    pub tire_load_reference: f32,
    pub tire_squeal_scale: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GpuTuning {
    pub shadow_map_size: u32,
    pub car_shadow_map_size: u32,
    pub baked_resolution: u32,
    pub baked_half_extent: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShadowTuning {
    pub drive_extent: f32,
    pub full_track: bool,
    pub strength: f32,
    pub depth_bias: f32,
    pub car_extent: f32,
    pub car_max_extent: f32,
    pub car_height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleTuning {
    pub max_active: usize,
    pub max_spawn_per_step: usize,
    pub min_speed: f32,
    pub max_dt: f32,
    pub front_gate_speed: f32,
    pub front_gate_range: f32,
    pub lateral_threshold: f32,
    pub lateral_scale: f32,
    pub longitudinal_threshold: f32,
    pub load_reference: f32,
    pub load_min: f32,
    pub speed_factor_range: f32,
    pub accumulator_max: f32,
    pub surfaces: ParticleSurfaces,
    pub spawn: ParticleSpawn,
    pub fade: ParticleFade,
    pub render: ParticleRenderTuning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleSurfaces {
    pub asphalt: ParticleSurface,
    pub concrete: ParticleSurface,
    pub grass: ParticleSurface,
    pub kerb: ParticleSurface,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleSurface {
    pub rate_scale: f32,
    pub rate_cap: f32,
    pub rolling_speed: f32,
    pub rolling_amount: f32,
    pub rolling_rate: f32,
    pub color: [f32; 3],
    pub start_size_min: f32,
    pub start_size_max: f32,
    pub end_size_min: f32,
    pub end_size_max: f32,
    pub end_variance_min: f32,
    pub end_variance_max: f32,
    pub life_min: f32,
    pub life_max: f32,
    pub alpha: f32,
    pub rise: f32,
    pub drag: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleSpawn {
    pub contact_offset: f32,
    pub width_jitter_fraction: f32,
    pub forward_jitter: f32,
    pub up_jitter_min: f32,
    pub up_jitter_max: f32,
    pub velocity_inherit: f32,
    pub forward_speed_base: f32,
    pub forward_speed_scale: f32,
    pub lateral_spread: f32,
    pub outward_min: f32,
    pub outward_max: f32,
    pub up_velocity_min: f32,
    pub up_velocity_max: f32,
    pub tint_min: f32,
    pub tint_max: f32,
    pub spin_min: f32,
    pub spin_max: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleFade {
    pub fade_in_end: f32,
    pub fade_out_start: f32,
    pub grow_power: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParticleRenderTuning {
    pub near_fade_start: f32,
    pub near_fade_end: f32,
    pub radial_start: f32,
    pub radial_end: f32,
    pub shade_min: f32,
    pub shade_max: f32,
}

impl Default for CameraTuning {
    fn default() -> Self {
        Self {
            bonnet: CameraShot {
                eye: [0.0, 1.25, -0.1],
                target: [0.0, 1.05, -20.0],
                fov_y: 1.0,
            },
            cockpit: CameraShot {
                eye: [-0.35, 1.10, 0.25],
                target: [-0.35, 1.00, -20.0],
                fov_y: 1.0,
            },
            isometric: IsometricTuning::default(),
            chase: ChaseTuning::default(),
            cockpit_glass_alpha: 0.06,
        }
    }
}

impl Default for CameraShot {
    fn default() -> Self {
        CameraTuning::default().bonnet
    }
}

impl Default for IsometricTuning {
    fn default() -> Self {
        Self {
            offset: [14.0, 24.0, 14.0],
            focus_height: 0.45,
            fov_y: 0.72,
            fit_aspect_floor: 0.65,
            fit_power: 0.5,
            fit_min: 1.0,
            fit_max: 1.45,
        }
    }
}

impl Default for ChaseTuning {
    fn default() -> Self {
        Self {
            aspect_near: 0.75,
            aspect_far: 2.0,
            back_near: 7.2,
            back_far: 4.8,
            height_near: 2.7,
            height_far: 2.0,
            look_near: -5.0,
            look_far: -2.5,
            target_height: 0.65,
            fov_y: 0.95,
            min_eye_y: 0.8,
        }
    }
}

impl Default for AudioTuning {
    fn default() -> Self {
        Self {
            master_gain: 0.35,
            tire_gain: 0.9,
            transmission_mix: 0.0,
            rpm_tau: 0.06,
            load_attack: 0.04,
            load_release: 0.32,
            max_rpm_slew: 12_000.0,
            max_audio_rpm: 9_000.0,
            tire_lateral_threshold: 0.09,
            tire_longitudinal_threshold: 0.14,
            tire_lateral_scale: 1.4,
            tire_longitudinal_scale: 1.0,
            tire_load_reference: 4_500.0,
            tire_squeal_scale: 4.2,
        }
    }
}

impl Default for ShadowTuning {
    fn default() -> Self {
        Self {
            drive_extent: 32.0,
            full_track: false,
            strength: 0.5,
            depth_bias: 0.0008,
            car_extent: 8.0,
            car_max_extent: 14.0,
            car_height: 1.6,
        }
    }
}

impl Default for ParticleTuning {
    fn default() -> Self {
        Self {
            max_active: MAX_PARTICLES,
            max_spawn_per_step: 8,
            min_speed: 1.5,
            max_dt: 0.05,
            front_gate_speed: 8.0,
            front_gate_range: 10.0,
            lateral_threshold: 0.09,
            lateral_scale: 1.5,
            longitudinal_threshold: 0.14,
            load_reference: 4_500.0,
            load_min: 0.12,
            speed_factor_range: 8.0,
            accumulator_max: 2.0,
            surfaces: ParticleSurfaces::default(),
            spawn: ParticleSpawn::default(),
            fade: ParticleFade::default(),
            render: ParticleRenderTuning::default(),
        }
    }
}

impl Default for ParticleSurfaces {
    fn default() -> Self {
        Self {
            asphalt: ParticleSurface {
                rate_scale: 2_000.0,
                rate_cap: 210.0,
                rolling_speed: 12.0,
                rolling_amount: 0.0,
                rolling_rate: 0.0,
                color: [0.55, 0.56, 0.58],
                start_size_min: 0.10,
                start_size_max: 0.17,
                end_size_min: 0.75,
                end_size_max: 1.55,
                end_variance_min: 0.90,
                end_variance_max: 1.20,
                life_min: 1.10,
                life_max: 1.70,
                alpha: 0.30,
                rise: 0.60,
                drag: 1.0,
            },
            concrete: ParticleSurface {
                rate_scale: 800.0,
                rate_cap: 150.0,
                rolling_speed: 12.0,
                rolling_amount: 0.18,
                rolling_rate: 500.0,
                color: [0.50, 0.44, 0.33],
                start_size_min: 0.09,
                start_size_max: 0.15,
                end_size_min: 0.55,
                end_size_max: 1.15,
                end_variance_min: 1.0,
                end_variance_max: 1.0,
                life_min: 0.80,
                life_max: 1.25,
                alpha: 0.56,
                rise: 0.45,
                drag: 1.3,
            },
            grass: ParticleSurface {
                rate_scale: 350.0,
                rate_cap: 150.0,
                rolling_speed: 12.0,
                rolling_amount: 0.18,
                rolling_rate: 600.0,
                color: [1.0, 1.0, 1.0],
                start_size_min: 0.018,
                start_size_max: 0.030,
                end_size_min: 0.08,
                end_size_max: 0.15,
                end_variance_min: 0.85,
                end_variance_max: 1.15,
                life_min: 0.45,
                life_max: 0.80,
                alpha: 0.90,
                rise: 0.45,
                drag: 2.2,
            },
            kerb: ParticleSurface {
                rate_scale: 400.0,
                rate_cap: 100.0,
                rolling_speed: 12.0,
                rolling_amount: 0.0,
                rolling_rate: 0.0,
                color: [0.50, 0.44, 0.32],
                start_size_min: 0.07,
                start_size_max: 0.11,
                end_size_min: 0.35,
                end_size_max: 0.75,
                end_variance_min: 1.0,
                end_variance_max: 1.0,
                life_min: 0.50,
                life_max: 0.85,
                alpha: 0.50,
                rise: 0.30,
                drag: 1.7,
            },
        }
    }
}

impl Default for ParticleSurface {
    fn default() -> Self {
        ParticleSurfaces::default().asphalt
    }
}

impl Default for ParticleSpawn {
    fn default() -> Self {
        Self {
            contact_offset: 0.02,
            width_jitter_fraction: 0.35,
            forward_jitter: 0.12,
            up_jitter_min: 0.06,
            up_jitter_max: 0.16,
            velocity_inherit: 0.12,
            forward_speed_base: 0.35,
            forward_speed_scale: 0.06,
            lateral_spread: 0.35,
            outward_min: 0.1,
            outward_max: 0.4,
            up_velocity_min: 1.0,
            up_velocity_max: 2.3,
            tint_min: 0.84,
            tint_max: 1.14,
            spin_min: -0.8,
            spin_max: 0.8,
        }
    }
}

impl Default for ParticleFade {
    fn default() -> Self {
        Self {
            fade_in_end: 0.10,
            fade_out_start: 0.55,
            grow_power: 2.0,
        }
    }
}

impl Default for ParticleRenderTuning {
    fn default() -> Self {
        Self {
            near_fade_start: 0.45,
            near_fade_end: 2.6,
            radial_start: 0.55,
            radial_end: 1.0,
            shade_min: 0.78,
            shade_max: 1.0,
        }
    }
}

impl TuningConfig {
    pub fn schema_json() -> String {
        Value::Array(schema_fields()).to_string()
    }

    pub fn sanitize(&mut self) {
        self.camera.cockpit_glass_alpha = self.camera.cockpit_glass_alpha.clamp(0.0, 1.0);
        self.camera.bonnet.fov_y = self.camera.bonnet.fov_y.clamp(0.1, 3.0);
        self.camera.cockpit.fov_y = self.camera.cockpit.fov_y.clamp(0.1, 3.0);
        self.camera.isometric.fov_y = self.camera.isometric.fov_y.clamp(0.1, 3.0);
        self.camera.isometric.fit_power = self.camera.isometric.fit_power.clamp(0.1, 2.0);
        self.camera.isometric.fit_min = self.camera.isometric.fit_min.max(0.1);
        self.camera.isometric.fit_max = self
            .camera
            .isometric
            .fit_max
            .max(self.camera.isometric.fit_min);
        self.camera.chase.fov_y = self.camera.chase.fov_y.clamp(0.1, 3.0);
        self.shadows.drive_extent = self.shadows.drive_extent.clamp(1.0, 1_000.0);
        self.shadows.strength = self.shadows.strength.clamp(0.0, 1.0);
        self.shadows.depth_bias = self.shadows.depth_bias.clamp(0.0, 0.02);
        self.shadows.car_extent = self.shadows.car_extent.clamp(1.0, 100.0);
        self.shadows.car_max_extent = self
            .shadows
            .car_max_extent
            .max(self.shadows.car_extent)
            .clamp(1.0, 200.0);
        self.shadows.car_height = self.shadows.car_height.clamp(0.1, 20.0);
        self.gpu.shadow_map_size = self.gpu.shadow_map_size.min(4096);
        self.gpu.car_shadow_map_size = self.gpu.car_shadow_map_size.min(4096);
        self.gpu.baked_resolution = self.gpu.baked_resolution.min(4096);
        self.gpu.baked_half_extent = self.gpu.baked_half_extent.clamp(0.0, 1_000.0);
        self.physics.sanitize();
        self.audio.master_gain = self.audio.master_gain.clamp(0.0, 1.0);
        self.audio.tire_gain = self.audio.tire_gain.clamp(0.0, 2.0);
        self.audio.transmission_mix = self.audio.transmission_mix.clamp(0.0, 1.0);
        self.audio.rpm_tau = self.audio.rpm_tau.clamp(0.005, 0.5);
        self.audio.load_attack = self.audio.load_attack.clamp(0.005, 1.0);
        self.audio.load_release = self.audio.load_release.clamp(0.005, 2.0);
        self.audio.max_rpm_slew = self.audio.max_rpm_slew.clamp(100.0, 50_000.0);
        self.audio.max_audio_rpm = self.audio.max_audio_rpm.clamp(500.0, 20_000.0);
        self.audio.tire_lateral_threshold = self.audio.tire_lateral_threshold.clamp(0.0, 1.0);
        self.audio.tire_longitudinal_threshold =
            self.audio.tire_longitudinal_threshold.clamp(0.0, 1.0);
        self.audio.tire_lateral_scale = self.audio.tire_lateral_scale.clamp(0.0, 10.0);
        self.audio.tire_longitudinal_scale = self.audio.tire_longitudinal_scale.clamp(0.0, 10.0);
        self.audio.tire_load_reference = self.audio.tire_load_reference.max(1.0);
        self.audio.tire_squeal_scale = self.audio.tire_squeal_scale.clamp(0.0, 20.0);
        self.particles.max_active = self.particles.max_active.min(MAX_PARTICLES);
        self.particles.max_spawn_per_step = self.particles.max_spawn_per_step.min(64);
        self.particles.max_dt = self.particles.max_dt.clamp(0.001, 0.2);
        self.particles.front_gate_range = self.particles.front_gate_range.max(0.01);
        self.particles.load_reference = self.particles.load_reference.max(1.0);
        self.particles.speed_factor_range = self.particles.speed_factor_range.max(0.01);
        self.particles.fade.fade_in_end = self.particles.fade.fade_in_end.clamp(0.0, 1.0);
        self.particles.fade.fade_out_start = self
            .particles
            .fade
            .fade_out_start
            .max(self.particles.fade.fade_in_end)
            .clamp(0.0, 1.0);
        self.particles.fade.grow_power = self.particles.fade.grow_power.clamp(0.1, 5.0);
        self.particles.render.near_fade_start =
            self.particles.render.near_fade_start.clamp(0.0, 10.0);
        self.particles.render.near_fade_end = self
            .particles
            .render
            .near_fade_end
            .max(self.particles.render.near_fade_start + 0.01)
            .clamp(0.01, 50.0);
        self.particles.render.radial_start = self.particles.render.radial_start.clamp(0.0, 1.0);
        self.particles.render.radial_end = self
            .particles
            .render
            .radial_end
            .max(self.particles.render.radial_start + 0.01)
            .clamp(0.01, 2.0);
        self.particles.render.shade_min = self.particles.render.shade_min.clamp(0.0, 2.0);
        self.particles.render.shade_max = self
            .particles
            .render
            .shade_max
            .max(self.particles.render.shade_min)
            .clamp(0.0, 2.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn field(
    path: &str,
    label: &str,
    group: &str,
    kind: &str,
    min: f64,
    max: f64,
    step: f64,
    unit: &str,
) -> Value {
    let apply = if path.starts_with("physics.") {
        "physics"
    } else if path.starts_with("gpu.") {
        "gpu"
    } else {
        "live"
    };
    json!({
        "path": path,
        "label": label,
        "group": group,
        "kind": kind,
        "min": min,
        "max": max,
        "step": step,
        "unit": unit,
        "apply": apply,
    })
}

fn shot_fields(prefix: &str, group: &str) -> Vec<Value> {
    let mut fields = Vec::new();
    for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
        fields.push(field(
            &format!("{prefix}.eye.{axis}"),
            &format!("Eye {name}"),
            group,
            "float",
            -20.0,
            20.0,
            0.01,
            "m",
        ));
    }
    for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
        fields.push(field(
            &format!("{prefix}.target.{axis}"),
            &format!("Target {name}"),
            group,
            "float",
            -50.0,
            50.0,
            0.05,
            "m",
        ));
    }
    fields.push(field(
        &format!("{prefix}.fovY"),
        "FOV",
        group,
        "float",
        0.1,
        3.0,
        0.01,
        "rad",
    ));
    fields
}

fn surface_fields(key: &str, group: &str) -> Vec<Value> {
    let prefix = format!("particles.surfaces.{key}");
    vec![
        field(
            &format!("{prefix}.rateScale"),
            "Skid rate scale",
            group,
            "float",
            0.0,
            10_000.0,
            10.0,
            "/s",
        ),
        field(
            &format!("{prefix}.rateCap"),
            "Skid rate cap",
            group,
            "float",
            0.0,
            1_000.0,
            1.0,
            "/s",
        ),
        field(
            &format!("{prefix}.rollingSpeed"),
            "Rolling speed ref",
            group,
            "float",
            0.1,
            100.0,
            0.1,
            "m/s",
        ),
        field(
            &format!("{prefix}.rollingAmount"),
            "Rolling amount",
            group,
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
        field(
            &format!("{prefix}.rollingRate"),
            "Rolling rate scale",
            group,
            "float",
            0.0,
            5_000.0,
            10.0,
            "/s",
        ),
        field(
            &format!("{prefix}.color"),
            "Color",
            group,
            "color",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            &format!("{prefix}.startSizeMin"),
            "Start size min",
            group,
            "float",
            0.0,
            3.0,
            0.01,
            "m",
        ),
        field(
            &format!("{prefix}.startSizeMax"),
            "Start size max",
            group,
            "float",
            0.0,
            3.0,
            0.01,
            "m",
        ),
        field(
            &format!("{prefix}.endSizeMin"),
            "End size min",
            group,
            "float",
            0.0,
            5.0,
            0.01,
            "m",
        ),
        field(
            &format!("{prefix}.endSizeMax"),
            "End size max",
            group,
            "float",
            0.0,
            5.0,
            0.01,
            "m",
        ),
        field(
            &format!("{prefix}.endVarianceMin"),
            "End variance min",
            group,
            "float",
            0.0,
            3.0,
            0.01,
            "×",
        ),
        field(
            &format!("{prefix}.endVarianceMax"),
            "End variance max",
            group,
            "float",
            0.0,
            3.0,
            0.01,
            "×",
        ),
        field(
            &format!("{prefix}.lifeMin"),
            "Life min",
            group,
            "float",
            0.0,
            5.0,
            0.01,
            "s",
        ),
        field(
            &format!("{prefix}.lifeMax"),
            "Life max",
            group,
            "float",
            0.0,
            5.0,
            0.01,
            "s",
        ),
        field(
            &format!("{prefix}.alpha"),
            "Alpha",
            group,
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            &format!("{prefix}.rise"),
            "Rise",
            group,
            "float",
            -5.0,
            5.0,
            0.01,
            "m/s²",
        ),
        field(
            &format!("{prefix}.drag"),
            "Drag",
            group,
            "float",
            0.0,
            10.0,
            0.01,
            "/s",
        ),
    ]
}

fn schema_fields() -> Vec<Value> {
    let mut fields = Vec::new();
    fields.extend(shot_fields("camera.bonnet", "Camera / Bonnet"));
    fields.extend(shot_fields("camera.cockpit", "Camera / Cockpit"));
    fields.push(field(
        "camera.cockpitGlassAlpha",
        "Cockpit glass alpha",
        "Camera / Cockpit",
        "float",
        0.0,
        1.0,
        0.01,
        "",
    ));
    fields.extend([
        field(
            "camera.isometric.offset.0",
            "Offset X",
            "Camera / Isometric",
            "float",
            0.0,
            100.0,
            0.1,
            "m",
        ),
        field(
            "camera.isometric.offset.1",
            "Offset Y",
            "Camera / Isometric",
            "float",
            0.0,
            100.0,
            0.1,
            "m",
        ),
        field(
            "camera.isometric.offset.2",
            "Offset Z",
            "Camera / Isometric",
            "float",
            0.0,
            100.0,
            0.1,
            "m",
        ),
        field(
            "camera.isometric.focusHeight",
            "Focus height",
            "Camera / Isometric",
            "float",
            -2.0,
            5.0,
            0.01,
            "m",
        ),
        field(
            "camera.isometric.fovY",
            "FOV",
            "Camera / Isometric",
            "float",
            0.1,
            3.0,
            0.01,
            "rad",
        ),
        field(
            "camera.isometric.fitAspectFloor",
            "Aspect floor",
            "Camera / Isometric",
            "float",
            0.1,
            2.0,
            0.01,
            "",
        ),
        field(
            "camera.isometric.fitPower",
            "Fit power",
            "Camera / Isometric",
            "float",
            0.1,
            2.0,
            0.01,
            "",
        ),
        field(
            "camera.isometric.fitMin",
            "Fit min",
            "Camera / Isometric",
            "float",
            0.1,
            3.0,
            0.01,
            "×",
        ),
        field(
            "camera.isometric.fitMax",
            "Fit max",
            "Camera / Isometric",
            "float",
            0.1,
            5.0,
            0.01,
            "×",
        ),
    ]);
    fields.extend([
        field(
            "camera.chase.aspectNear",
            "Aspect near",
            "Camera / Chase",
            "float",
            0.1,
            5.0,
            0.01,
            "",
        ),
        field(
            "camera.chase.aspectFar",
            "Aspect far",
            "Camera / Chase",
            "float",
            0.1,
            5.0,
            0.01,
            "",
        ),
        field(
            "camera.chase.backNear",
            "Back near",
            "Camera / Chase",
            "float",
            0.0,
            30.0,
            0.1,
            "m",
        ),
        field(
            "camera.chase.backFar",
            "Back far",
            "Camera / Chase",
            "float",
            0.0,
            30.0,
            0.1,
            "m",
        ),
        field(
            "camera.chase.heightNear",
            "Height near",
            "Camera / Chase",
            "float",
            -2.0,
            10.0,
            0.05,
            "m",
        ),
        field(
            "camera.chase.heightFar",
            "Height far",
            "Camera / Chase",
            "float",
            -2.0,
            10.0,
            0.05,
            "m",
        ),
        field(
            "camera.chase.lookNear",
            "Look ahead near",
            "Camera / Chase",
            "float",
            -50.0,
            10.0,
            0.1,
            "m",
        ),
        field(
            "camera.chase.lookFar",
            "Look ahead far",
            "Camera / Chase",
            "float",
            -50.0,
            10.0,
            0.1,
            "m",
        ),
        field(
            "camera.chase.targetHeight",
            "Target height",
            "Camera / Chase",
            "float",
            -2.0,
            5.0,
            0.01,
            "m",
        ),
        field(
            "camera.chase.fovY",
            "FOV",
            "Camera / Chase",
            "float",
            0.1,
            3.0,
            0.01,
            "rad",
        ),
        field(
            "camera.chase.minEyeY",
            "Minimum eye height",
            "Camera / Chase",
            "float",
            -2.0,
            5.0,
            0.01,
            "m",
        ),
    ]);
    fields.extend([
        field(
            "shadows.driveExtent",
            "Drive shadow extent",
            "Rendering / Shadows",
            "float",
            5.0,
            500.0,
            1.0,
            "m",
        ),
        field(
            "shadows.fullTrack",
            "Full-track dynamic shadows",
            "Rendering / Shadows",
            "bool",
            0.0,
            1.0,
            1.0,
            "",
        ),
        field(
            "shadows.strength",
            "Shadow strength",
            "Rendering / Shadows",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "shadows.depthBias",
            "Depth bias",
            "Rendering / Shadows",
            "float",
            0.0,
            0.02,
            0.0001,
            "",
        ),
        field(
            "shadows.carExtent",
            "Car shadow extent",
            "Rendering / Shadows",
            "float",
            1.0,
            100.0,
            0.5,
            "m",
        ),
        field(
            "shadows.carMaxExtent",
            "Car shadow max extent",
            "Rendering / Shadows",
            "float",
            1.0,
            200.0,
            0.5,
            "m",
        ),
        field(
            "shadows.carHeight",
            "Car shadow height",
            "Rendering / Shadows",
            "float",
            0.1,
            20.0,
            0.05,
            "m",
        ),
        field(
            "audio.masterGain",
            "Master gain",
            "Audio / Mix",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "audio.tireGain",
            "Tire screech gain",
            "Audio / Mix",
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
        field(
            "audio.transmissionMix",
            "Transmission mix",
            "Audio / Mix",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "audio.rpmTau",
            "RPM smoothing",
            "Audio / Conditioner",
            "float",
            0.005,
            0.5,
            0.005,
            "s",
        ),
        field(
            "audio.loadAttack",
            "Load attack",
            "Audio / Conditioner",
            "float",
            0.005,
            1.0,
            0.005,
            "s",
        ),
        field(
            "audio.loadRelease",
            "Load release",
            "Audio / Conditioner",
            "float",
            0.005,
            2.0,
            0.005,
            "s",
        ),
        field(
            "audio.maxRpmSlew",
            "Max RPM slew",
            "Audio / Conditioner",
            "float",
            100.0,
            50_000.0,
            100.0,
            "rpm/s",
        ),
        field(
            "audio.maxAudioRpm",
            "Max audio RPM",
            "Audio / Conditioner",
            "float",
            500.0,
            20_000.0,
            100.0,
            "rpm",
        ),
        field(
            "audio.tireLateralThreshold",
            "Tire lateral slip threshold",
            "Audio / Tires",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "audio.tireLongitudinalThreshold",
            "Tire longitudinal slip threshold",
            "Audio / Tires",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "audio.tireLateralScale",
            "Tire lateral slip scale",
            "Audio / Tires",
            "float",
            0.0,
            10.0,
            0.01,
            "",
        ),
        field(
            "audio.tireLongitudinalScale",
            "Tire longitudinal slip scale",
            "Audio / Tires",
            "float",
            0.0,
            10.0,
            0.01,
            "",
        ),
        field(
            "audio.tireLoadReference",
            "Tire load reference",
            "Audio / Tires",
            "float",
            1.0,
            20_000.0,
            50.0,
            "N",
        ),
        field(
            "audio.tireSquealScale",
            "Tire squeal scale",
            "Audio / Tires",
            "float",
            0.0,
            20.0,
            0.1,
            "",
        ),
        field(
            "physics.gravity",
            "Gravity",
            "Physics / World",
            "float",
            0.0,
            50.0,
            0.01,
            "m/s²",
        ),
        field(
            "physics.airDensity",
            "Air density",
            "Physics / World",
            "float",
            0.0,
            5.0,
            0.001,
            "kg/m³",
        ),
        field(
            "physics.wheelMass",
            "Wheel mass",
            "Physics / World",
            "float",
            1.0,
            200.0,
            0.1,
            "kg",
        ),
        field(
            "physics.bumpStopRateMultiplier",
            "Bump-stop multiplier",
            "Physics / Suspension",
            "float",
            0.0,
            100.0,
            0.5,
            "×",
        ),
        field(
            "physics.maxLoadFactor",
            "Max load factor",
            "Physics / Suspension",
            "float",
            1.0,
            20.0,
            0.1,
            "×",
        ),
        field(
            "physics.steeringSpeedFalloff",
            "Steering speed falloff",
            "Physics / Steering",
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
        field(
            "physics.steeringMinLimit",
            "Minimum steering lock",
            "Physics / Steering",
            "float",
            0.001,
            1.0,
            0.001,
            "rad",
        ),
        field(
            "physics.bodyProbeHeight",
            "Body probe height",
            "Physics / Collision",
            "float",
            0.1,
            50.0,
            0.1,
            "m",
        ),
        field(
            "physics.bodyProbeMargin",
            "Body probe margin",
            "Physics / Collision",
            "float",
            0.0,
            50.0,
            0.1,
            "m",
        ),
        field(
            "gpu.shadowMapSize",
            "Dynamic shadow map (env + car)",
            "GPU / Textures",
            "int",
            0.0,
            4096.0,
            256.0,
            "px",
        ),
        field(
            "gpu.carShadowMapSize",
            "Car-only shadow map",
            "GPU / Textures",
            "int",
            0.0,
            4096.0,
            256.0,
            "px",
        ),
        field(
            "gpu.bakedResolution",
            "Baked environment resolution",
            "GPU / Textures",
            "int",
            0.0,
            4096.0,
            256.0,
            "px",
        ),
        field(
            "gpu.bakedHalfExtent",
            "Baked region half-extent (0 = auto)",
            "GPU / Textures",
            "float",
            0.0,
            500.0,
            5.0,
            "m",
        ),
        field(
            "particles.maxActive",
            "Max active",
            "Particles / Global",
            "int",
            0.0,
            MAX_PARTICLES as f64,
            1.0,
            "",
        ),
        field(
            "particles.maxSpawnPerStep",
            "Max spawn / step",
            "Particles / Global",
            "int",
            0.0,
            64.0,
            1.0,
            "",
        ),
        field(
            "particles.minSpeed",
            "Minimum speed",
            "Particles / Global",
            "float",
            0.0,
            50.0,
            0.1,
            "m/s",
        ),
        field(
            "particles.maxDt",
            "Max time step",
            "Particles / Global",
            "float",
            0.001,
            0.2,
            0.001,
            "s",
        ),
        field(
            "particles.frontGateSpeed",
            "Front wheel gate",
            "Particles / Global",
            "float",
            0.0,
            50.0,
            0.1,
            "m/s",
        ),
        field(
            "particles.frontGateRange",
            "Front gate range",
            "Particles / Global",
            "float",
            0.01,
            50.0,
            0.1,
            "m/s",
        ),
        field(
            "particles.lateralThreshold",
            "Lateral slip threshold",
            "Particles / Global",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.lateralScale",
            "Lateral slip scale",
            "Particles / Global",
            "float",
            0.0,
            10.0,
            0.01,
            "",
        ),
        field(
            "particles.longitudinalThreshold",
            "Longitudinal slip threshold",
            "Particles / Global",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.loadReference",
            "Load reference",
            "Particles / Global",
            "float",
            100.0,
            20_000.0,
            50.0,
            "N",
        ),
        field(
            "particles.loadMin",
            "Load minimum",
            "Particles / Global",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.speedFactorRange",
            "Speed factor range",
            "Particles / Global",
            "float",
            0.01,
            50.0,
            0.1,
            "m/s",
        ),
        field(
            "particles.accumulatorMax",
            "Accumulator max",
            "Particles / Global",
            "float",
            0.0,
            10.0,
            0.1,
            "",
        ),
    ]);
    fields.extend([
        field(
            "particles.spawn.contactOffset",
            "Contact offset",
            "Particles / Spawn",
            "float",
            0.0,
            1.0,
            0.01,
            "m",
        ),
        field(
            "particles.spawn.widthJitterFraction",
            "Width jitter",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "×",
        ),
        field(
            "particles.spawn.forwardJitter",
            "Forward jitter",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "m",
        ),
        field(
            "particles.spawn.upJitterMin",
            "Up jitter min",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "m",
        ),
        field(
            "particles.spawn.upJitterMax",
            "Up jitter max",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "m",
        ),
        field(
            "particles.spawn.velocityInherit",
            "Velocity inherit",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "×",
        ),
        field(
            "particles.spawn.forwardSpeedBase",
            "Forward speed base",
            "Particles / Spawn",
            "float",
            0.0,
            5.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.forwardSpeedScale",
            "Forward speed scale",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
        field(
            "particles.spawn.lateralSpread",
            "Lateral spread",
            "Particles / Spawn",
            "float",
            0.0,
            5.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.outwardMin",
            "Outward min",
            "Particles / Spawn",
            "float",
            0.0,
            5.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.outwardMax",
            "Outward max",
            "Particles / Spawn",
            "float",
            0.0,
            5.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.upVelocityMin",
            "Up velocity min",
            "Particles / Spawn",
            "float",
            0.0,
            10.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.upVelocityMax",
            "Up velocity max",
            "Particles / Spawn",
            "float",
            0.0,
            10.0,
            0.01,
            "m/s",
        ),
        field(
            "particles.spawn.tintMin",
            "Tint min",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "×",
        ),
        field(
            "particles.spawn.tintMax",
            "Tint max",
            "Particles / Spawn",
            "float",
            0.0,
            2.0,
            0.01,
            "×",
        ),
        field(
            "particles.spawn.spinMin",
            "Spin min",
            "Particles / Spawn",
            "float",
            -10.0,
            10.0,
            0.01,
            "rad/s",
        ),
        field(
            "particles.spawn.spinMax",
            "Spin max",
            "Particles / Spawn",
            "float",
            -10.0,
            10.0,
            0.01,
            "rad/s",
        ),
    ]);
    fields.extend([
        field(
            "particles.fade.fadeInEnd",
            "Fade in end",
            "Particles / Fade",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.fade.fadeOutStart",
            "Fade out start",
            "Particles / Fade",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.fade.growPower",
            "Growth power",
            "Particles / Fade",
            "float",
            0.1,
            5.0,
            0.01,
            "",
        ),
    ]);
    fields.extend([
        field(
            "particles.render.nearFadeStart",
            "Near fade start",
            "Particles / Render",
            "float",
            0.0,
            10.0,
            0.01,
            "m",
        ),
        field(
            "particles.render.nearFadeEnd",
            "Near fade end",
            "Particles / Render",
            "float",
            0.01,
            50.0,
            0.01,
            "m",
        ),
        field(
            "particles.render.radialStart",
            "Radial softness start",
            "Particles / Render",
            "float",
            0.0,
            1.0,
            0.01,
            "",
        ),
        field(
            "particles.render.radialEnd",
            "Radial softness end",
            "Particles / Render",
            "float",
            0.01,
            2.0,
            0.01,
            "",
        ),
        field(
            "particles.render.shadeMin",
            "Shade min",
            "Particles / Render",
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
        field(
            "particles.render.shadeMax",
            "Shade max",
            "Particles / Render",
            "float",
            0.0,
            2.0,
            0.01,
            "",
        ),
    ]);
    for (key, label) in [
        ("asphalt", "Asphalt"),
        ("concrete", "Concrete"),
        ("grass", "Grass"),
        ("kerb", "Kerb"),
    ] {
        fields.extend(surface_fields(key, &format!("Particles / {label}")));
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current = value;
        for key in path.split('.') {
            current = match current {
                Value::Object(map) => map.get(key)?,
                Value::Array(items) => items.get(key.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    #[test]
    fn schema_paths_resolve_against_defaults() {
        let defaults = serde_json::to_value(TuningConfig::default()).expect("defaults");
        for field in schema_fields() {
            let path = field["path"].as_str().expect("schema path");
            assert!(
                path_value(&defaults, path).is_some(),
                "missing tuning path {path}"
            );
            assert!(
                field["group"]
                    .as_str()
                    .is_some_and(|group| !group.is_empty()),
                "missing group for {path}"
            );
        }
    }

    #[test]
    fn sanitize_limits_destructive_values() {
        let mut tuning = TuningConfig::default();
        tuning.particles.max_active = MAX_PARTICLES + 1;
        tuning.particles.max_spawn_per_step = 1_000;
        tuning.particles.fade.fade_in_end = 0.8;
        tuning.particles.fade.fade_out_start = 0.2;
        tuning.camera.chase.fov_y = 10.0;
        tuning.sanitize();
        assert_eq!(tuning.particles.max_active, MAX_PARTICLES);
        assert_eq!(tuning.particles.max_spawn_per_step, 64);
        assert!(tuning.particles.fade.fade_out_start >= tuning.particles.fade.fade_in_end);
        assert_eq!(tuning.camera.chase.fov_y, 3.0);
    }
}
