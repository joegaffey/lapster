use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3, Vec4};
use wasm_bindgen::JsValue;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::grass::{GrassChunkKey, GrassField, GrassInstance, GRASS_CLUMPS_PER_FRAME};
use crate::particles::{ParticleInstance, MAX_PARTICLES};
use crate::scene::{LineVertex, MeshData, MeshVertex, OutlineVertex, Scene, SceneTexture};
use crate::tuning::{GpuTuning, ParticleRenderTuning, ShadowTuning};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const GRASS_CHUNK_FADE_MS: f64 = 450.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentShadowMode {
    High,
    Hybrid,
    Baked,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarShadowMode {
    Dynamic,
    Silhouette,
    Off,
}

static MESH_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2];
static OUTLINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
static LINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
static PARTICLE_MESH_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
static PARTICLE_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];
static GRASS_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] =
    wgpu::vertex_attr_array![0 => Float32x3];
static GRASS_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GrassVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GrassUniform {
    time: f32,
    wind_strength: f32,
    wind_speed: f32,
    wind_freq: f32,
    brightness: f32,
    translucency: f32,
    pad0: f32,
    pad1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GrassChunkFade {
    fade: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    light_dir: [f32; 4],
    light_view_proj: [[f32; 4]; 4],
    shadow_params: [f32; 4],
    sky_horizon: [f32; 4],
    sky_zenith: [f32; 4],
    baked_view_proj: [[f32; 4]; 4],
    baked_shadow_params: [f32; 4],
    shadow_modes: [f32; 4],
    car_shadow_params: [f32; 4],
    camera_right: [f32; 4],
    camera_up: [f32; 4],
    particle_params: [f32; 4],
    particle_shade: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    alpha: [f32; 4],
    emissive: [f32; 4],
}

struct GpuOutline {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

struct MeshTextureResources<'a> {
    layout: &'a wgpu::BindGroupLayout,
    textures: &'a [SceneTexture],
    white_view: &'a wgpu::TextureView,
    sampler: &'a wgpu::Sampler,
}

struct GpuGrassChunk {
    buffer: Option<wgpu::Buffer>,
    count: u32,
    fade_buffer: Option<wgpu::Buffer>,
    fade_bind_group: Option<wgpu::BindGroup>,
    fade_start_ms: f64,
    fade: f32,
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
    grass_texture_bind_group: Option<wgpu::BindGroup>,
    outline: Option<GpuOutline>,
    visible: bool,
    alpha: f32,
    emissive: [f32; 3],
    casts_static_shadow: bool,
    casts_dynamic_shadow: bool,
    shadow_static_base: bool,
    shadow_dynamic_base: bool,
    is_decal: bool,
    model: Mat4,
}

fn model_alpha(mesh: &GpuMesh, grass_texture_enabled: bool) -> [f32; 4] {
    [
        mesh.alpha,
        if mesh.grass_texture_bind_group.is_some() && grass_texture_enabled {
            1.0
        } else {
            0.0
        },
        0.0,
        0.0,
    ]
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    msaa_color_view: Option<wgpu::TextureView>,
    sample_count: u32,
    supported_samples: Vec<u32>,
    shader: wgpu::ShaderModule,
    mesh_pipeline_layout: wgpu::PipelineLayout,
    outline_pipeline_layout: wgpu::PipelineLayout,
    line_pipeline_layout: wgpu::PipelineLayout,
    decal_pipeline_layout: wgpu::PipelineLayout,
    sky_pipeline: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    decal_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    grass_pipeline: wgpu::RenderPipeline,
    grass_vertex_buffer: wgpu::Buffer,
    grass_index_buffer: wgpu::Buffer,
    grass_index_count: u32,
    grass_chunks: HashMap<GrassChunkKey, GpuGrassChunk>,
    grass_field: GrassField,
    grass_texture_enabled: bool,
    grass_shader_enabled: bool,
    grass_uniform_buffer: wgpu::Buffer,
    grass_bind_group: wgpu::BindGroup,
    grass_chunk_bind_group_layout: wgpu::BindGroupLayout,
    grass_start_ms: f64,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    mesh_texture_layout: wgpu::BindGroupLayout,
    mesh_sampler: wgpu::Sampler,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    bake_camera_buffer: wgpu::Buffer,
    bake_camera_bind_group: wgpu::BindGroup,
    sun_time: f32,
    sun_direction: Vec3,
    sun_elevation: f32,
    sky_horizon: [f32; 4],
    sky_zenith: [f32; 4],
    shadow_strength: f32,
    shadow_depth_bias: f32,
    car_shadow_extent: f32,
    car_shadow_max_extent: f32,
    car_shadow_height: f32,
    particle_params: [f32; 4],
    particle_shade: [f32; 4],
    environment_mode: EnvironmentShadowMode,
    car_shadow_mode: CarShadowMode,
    car_dynamic_supported: bool,
    env_shadow_extent: f32,
    shadow_sampler: wgpu::Sampler,
    baked_view: wgpu::TextureView,
    baked_view_proj: Mat4,
    baked_shadow_params: [f32; 4],
    baked_region: [f32; 4],
    baked_resolution: u32,
    baked_valid: bool,
    car_shadow_params: [f32; 4],
    decal_visible: bool,
    last_bake_ms: f64,
    shadow_view: wgpu::TextureView,
    car_shadow_view: wgpu::TextureView,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    shadow_bind_group: wgpu::BindGroup,
    car_shadow_bind_group: wgpu::BindGroup,
    shadow_map_size: u32,
    car_shadow_map_size: u32,
    shadow_extent: f32,
    shadow_focus: [f32; 3],
    shadow_view_proj: Mat4,
    shadow_dirty: bool,
    meshes: Vec<GpuMesh>,
    particle_pipeline: wgpu::RenderPipeline,
    particle_vertex_buffer: wgpu::Buffer,
    particle_index_buffer: wgpu::Buffer,
    particle_index_count: u32,
    particle_instance_buffer: wgpu::Buffer,
    particle_count: u32,
    line_buffer: Option<wgpu::Buffer>,
    line_count: u32,
    lines_visible: bool,
    camera: Camera,
    backend: String,
    surface_format: String,
}

impl Renderer {
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
        scene: Scene,
        gpu: &GpuTuning,
    ) -> Result<Self, JsValue> {
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = create_surface(&instance, canvas)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|error| JsValue::from_str(&format!("no suitable adapter found: {error:?}")))?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let sun_time = 15.0;
        let (sun_direction, sun_elevation) = sun_for_time(sun_time);
        let (sky_horizon, sky_zenith) = sky_colors(sun_elevation);
        let shadow = ShadowTuning::default();
        let particles = ParticleRenderTuning::default();

        let adapter_limits = adapter.limits();
        let mut limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        if adapter_limits.max_texture_dimension_2d >= 4096 {
            limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.max(4096);
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sim-render device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(|error| JsValue::from_str(&format!("failed to create device: {error:?}")))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let surface_format = format!("{format:?}");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let supported_samples = supported_samples(&adapter, format);
        let sample_count = select_sample_count(&supported_samples, 4);
        surface.configure(&device, &config);

        let depth_view = create_depth_view(&device, &config, sample_count);
        let msaa_color_view = create_msaa_color_view(&device, &config, sample_count);

        let shadow_map_size = resolve_texture_size(
            gpu.shadow_map_size,
            preferred_shadow_map_size(&backend),
            adapter_limits.max_texture_dimension_2d,
        );
        let car_shadow_map_size = resolve_texture_size(
            gpu.car_shadow_map_size,
            preferred_car_shadow_map_size(&backend, &adapter_limits),
            adapter_limits.max_texture_dimension_2d,
        );
        let shadow_extent = 60.0;
        let shadow_focus = scene.camera.target;
        let shadow_view_proj = shadow_view_proj(
            shadow_map_size,
            Vec3::from_array(shadow_focus),
            shadow_extent,
            sun_direction,
        );
        let shadow_view = create_shadow_map(&device, shadow_map_size);
        let car_shadow_view = create_shadow_map(&device, car_shadow_map_size);
        let baked_resolution = resolve_texture_size(
            gpu.baked_resolution,
            preferred_baked_resolution(&backend, &adapter_limits),
            adapter_limits.max_texture_dimension_2d,
        );
        let baked_view = create_baked_placeholder(&device);
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            anisotropy_clamp: 1,
            border_color: None,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("track shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("track.wgsl"))),
        });
        let grass_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grass shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("grass.wgsl"))),
        });

        let camera_uniform = camera_uniform(
            &scene.camera,
            config.width,
            config.height,
            shadow_view_proj,
            shadow_map_size,
            shadow_extent,
            true,
            sun_direction,
            sky_horizon,
            sky_zenith,
            shadow_view_proj,
            [
                1.0 / baked_resolution as f32,
                2.0 * shadow_extent / baked_resolution as f32,
                shadow.depth_bias,
                0.0,
            ],
            [1.0, 0.0, 0.0, 0.0],
            [0.0; 4],
            shadow.strength,
            shadow.depth_bias,
            [
                particles.near_fade_start,
                particles.near_fade_end,
                particles.radial_start,
                particles.radial_end,
            ],
            [particles.shade_min, particles.shade_max, 0.0, 0.0],
        );
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniform"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let bake_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bake camera uniform"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bake_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bake camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bake_camera_buffer.as_entire_binding(),
            }],
        });

        let model_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let shadow_bind_group = create_shadow_bind_group(
            &device,
            &shadow_bind_group_layout,
            &shadow_view,
            &shadow_sampler,
            &baked_view,
            "shadow bind group",
        );
        let car_shadow_bind_group = create_shadow_bind_group(
            &device,
            &shadow_bind_group_layout,
            &car_shadow_view,
            &shadow_sampler,
            &baked_view,
            "car shadow bind group",
        );

        let grass_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass uniform"),
            contents: bytemuck::bytes_of(&GrassUniform {
                time: 0.0,
                wind_strength: 0.16,
                wind_speed: 1.2,
                wind_freq: 0.35,
                brightness: 1.0,
                translucency: 0.65,
                pad0: 0.0,
                pad1: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let grass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("grass bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let grass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grass bind group"),
            layout: &grass_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: grass_uniform_buffer.as_entire_binding(),
            }],
        });
        let grass_chunk_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("grass chunk bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("track pipeline layout"),
            bind_group_layouts: &[
                Some(&camera_bind_group_layout),
                Some(&model_bind_group_layout),
                Some(&texture_bind_group_layout),
                Some(&shadow_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let outline_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("outline pipeline layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&model_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("line pipeline layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout)],
            immediate_size: 0,
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow pipeline layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&model_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let grass_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("grass pipeline layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&shadow_bind_group_layout),
                    Some(&grass_bind_group_layout),
                    Some(&grass_chunk_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let pipelines = create_pipelines(
            &device,
            &shader,
            &mesh_pipeline_layout,
            &outline_pipeline_layout,
            &line_pipeline_layout,
            &camera_bind_group_layout,
            format,
            sample_count,
        );
        let shadow_pipeline = create_shadow_pipeline(&device, &shader, &shadow_pipeline_layout);
        let grass_pipeline = create_grass_pipeline(
            &device,
            &grass_shader,
            &grass_pipeline_layout,
            format,
            sample_count,
        );
        let decal_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("decal pipeline layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&model_bind_group_layout),
                    Some(&texture_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let decal_pipeline = create_decal_pipeline(
            &device,
            &shader,
            &decal_pipeline_layout,
            format,
            sample_count,
        );
        let particle_pipeline =
            create_particle_pipeline(&device, &camera_bind_group_layout, format, sample_count);
        let (particle_vertex_buffer, particle_index_buffer, particle_index_count) =
            create_particle_mesh(&device);
        let particle_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle instances"),
            size: (MAX_PARTICLES * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let white_view = create_white_texture(&device, &queue);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mesh sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 8,
            border_color: None,
        });
        let texture_resources = MeshTextureResources {
            layout: &texture_bind_group_layout,
            textures: &scene.textures,
            white_view: &white_view,
            sampler: &sampler,
        };
        let meshes = scene
            .meshes
            .iter()
            .map(|mesh| {
                create_mesh(
                    &device,
                    &queue,
                    mesh,
                    &model_bind_group_layout,
                    &texture_resources,
                )
            })
            .collect::<Vec<_>>();

        let (grass_vertex_buffer, grass_index_buffer, grass_index_count) =
            create_grass_mesh(&device);
        let grass_field = scene.grass;

        let (line_buffer, line_count) = if scene.lines.is_empty() {
            (None, 0)
        } else {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("debug lines"),
                contents: bytemuck::cast_slice(&scene.lines),
                usage: wgpu::BufferUsages::VERTEX,
            });
            (Some(buffer), scene.lines.len() as u32)
        };

        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            depth_view,
            msaa_color_view,
            sample_count,
            supported_samples,
            shader,
            mesh_pipeline_layout,
            outline_pipeline_layout,
            line_pipeline_layout,
            decal_pipeline_layout: decal_pipeline_layout.clone(),
            sky_pipeline: pipelines.sky,
            mesh_pipeline: pipelines.mesh,
            transparent_pipeline: pipelines.transparent,
            shadow_pipeline,
            decal_pipeline,
            outline_pipeline: pipelines.outline,
            line_pipeline: pipelines.line,
            grass_pipeline,
            grass_vertex_buffer,
            grass_index_buffer,
            grass_index_count,
            grass_chunks: HashMap::new(),
            grass_field,
            grass_texture_enabled: true,
            grass_shader_enabled: false,
            grass_uniform_buffer,
            grass_bind_group,
            grass_chunk_bind_group_layout,
            grass_start_ms: performance_now(),
            camera_bind_group_layout: camera_bind_group_layout.clone(),
            mesh_texture_layout: texture_bind_group_layout,
            mesh_sampler: sampler,
            camera_buffer,
            camera_bind_group,
            bake_camera_buffer,
            bake_camera_bind_group,
            sun_time,
            sun_direction,
            sun_elevation,
            sky_horizon,
            sky_zenith,
            shadow_strength: shadow.strength,
            shadow_depth_bias: shadow.depth_bias,
            car_shadow_extent: shadow.car_extent,
            car_shadow_max_extent: shadow.car_max_extent,
            car_shadow_height: shadow.car_height,
            particle_params: [
                particles.near_fade_start,
                particles.near_fade_end,
                particles.radial_start,
                particles.radial_end,
            ],
            particle_shade: [particles.shade_min, particles.shade_max, 0.0, 0.0],
            environment_mode: EnvironmentShadowMode::High,
            car_shadow_mode: CarShadowMode::Dynamic,
            car_dynamic_supported: true,
            env_shadow_extent: shadow_extent,
            shadow_sampler,
            baked_view,
            baked_view_proj: shadow_view_proj,
            baked_shadow_params: [
                1.0 / baked_resolution as f32,
                2.0 * shadow_extent / baked_resolution as f32,
                shadow.depth_bias,
                0.0,
            ],
            baked_region: [
                shadow_focus[0],
                shadow_focus[2],
                shadow_extent,
                shadow_focus[1],
            ],
            baked_resolution,
            baked_valid: false,
            car_shadow_params: [0.0, 0.0, 0.0, 1.0],
            decal_visible: false,
            last_bake_ms: 0.0,
            shadow_view,
            car_shadow_view,
            shadow_bind_group_layout: shadow_bind_group_layout.clone(),
            shadow_bind_group,
            car_shadow_bind_group,
            shadow_map_size,
            car_shadow_map_size,
            shadow_extent,
            shadow_focus,
            shadow_view_proj,
            shadow_dirty: true,
            meshes,
            particle_pipeline,
            particle_vertex_buffer,
            particle_index_buffer,
            particle_index_count,
            particle_instance_buffer,
            particle_count: 0,
            line_buffer,
            line_count,
            lines_visible: true,
            camera: scene.camera,
            backend,
            surface_format,
        };
        renderer.sync_grass_chunks();
        renderer.set_grass_texture_enabled(true);
        Ok(renderer)
    }

    pub fn set_particles(&mut self, instances: &[ParticleInstance]) {
        self.particle_count = instances.len().min(MAX_PARTICLES) as u32;
        if self.particle_count == 0 {
            return;
        }
        self.queue.write_buffer(
            &self.particle_instance_buffer,
            0,
            bytemuck::cast_slice(&instances[..self.particle_count as usize]),
        );
    }

    pub fn set_grass_texture_enabled(&mut self, enabled: bool) {
        self.grass_texture_enabled = enabled;
        let value = if enabled { 1.0 } else { 0.0 };
        for mesh in &self.meshes {
            if mesh.grass_texture_bind_group.is_none() {
                continue;
            }
            let offset = std::mem::offset_of!(ModelUniform, alpha) as u64;
            self.queue.write_buffer(
                &mesh.model_buffer,
                offset,
                bytemuck::bytes_of(&[mesh.alpha, value, 0.0, 0.0]),
            );
        }
    }

    pub fn set_grass_shader_enabled(&mut self, enabled: bool) {
        self.grass_shader_enabled = enabled;
    }

    pub fn set_grass_center(&mut self, center: Vec3) {
        self.grass_field.update(center);
    }

    fn sync_grass_chunks(&mut self) {
        let stale = self
            .grass_chunks
            .keys()
            .copied()
            .filter(|key| !self.grass_field.chunks().contains_key(key))
            .collect::<Vec<_>>();
        for key in stale {
            self.grass_chunks.remove(&key);
        }

        let missing = self
            .grass_field
            .chunks()
            .keys()
            .copied()
            .filter(|key| !self.grass_chunks.contains_key(key))
            .collect::<Vec<_>>();
        for key in missing {
            let Some(instances) = self.grass_field.chunks().get(&key) else {
                continue;
            };
            let count = instances.len() as u32;
            let buffer = if count == 0 {
                None
            } else {
                Some(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("grass chunk instances"),
                            contents: bytemuck::cast_slice(instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            };
            let fade = GrassChunkFade {
                fade: 0.0,
                pad0: 0.0,
                pad1: 0.0,
                pad2: 0.0,
            };
            let fade_buffer = if count == 0 {
                None
            } else {
                Some(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("grass chunk fade"),
                            contents: bytemuck::bytes_of(&fade),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        }),
                )
            };
            let fade_bind_group = fade_buffer.as_ref().map(|buffer| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("grass chunk fade bind group"),
                    layout: &self.grass_chunk_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            });
            self.grass_chunks.insert(
                key,
                GpuGrassChunk {
                    buffer,
                    count,
                    fade_buffer,
                    fade_bind_group,
                    fade_start_ms: performance_now(),
                    fade: if count == 0 { 1.0 } else { 0.0 },
                },
            );
        }
    }

    fn update_grass_fades(&mut self) {
        let now = performance_now();
        for chunk in self.grass_chunks.values_mut() {
            if chunk.fade >= 1.0 {
                continue;
            }
            if chunk.count == 0 {
                chunk.fade = 1.0;
                continue;
            }
            let fade = ((now - chunk.fade_start_ms).max(0.0) / GRASS_CHUNK_FADE_MS).min(1.0) as f32;
            if fade <= chunk.fade {
                continue;
            }
            chunk.fade = fade;
            if let Some(buffer) = &chunk.fade_buffer {
                let data = GrassChunkFade {
                    fade,
                    pad0: 0.0,
                    pad1: 0.0,
                    pad2: 0.0,
                };
                self.queue
                    .write_buffer(buffer, 0, bytemuck::bytes_of(&data));
            }
        }
    }

    pub fn set_lines_visible(&mut self, visible: bool) {
        self.lines_visible = visible;
    }

    pub fn lines_visible(&self) -> bool {
        self.lines_visible
    }

    pub fn set_msaa_samples(&mut self, requested: u32) -> u32 {
        let actual = select_sample_count(&self.supported_samples, requested);
        if actual == self.sample_count {
            return actual;
        }
        self.sample_count = actual;
        self.msaa_color_view = create_msaa_color_view(&self.device, &self.config, actual);
        self.depth_view = create_depth_view(&self.device, &self.config, actual);
        let pipelines = create_pipelines(
            &self.device,
            &self.shader,
            &self.mesh_pipeline_layout,
            &self.outline_pipeline_layout,
            &self.line_pipeline_layout,
            &self.camera_bind_group_layout,
            self.config.format,
            actual,
        );
        self.sky_pipeline = pipelines.sky;
        self.mesh_pipeline = pipelines.mesh;
        self.transparent_pipeline = pipelines.transparent;
        self.outline_pipeline = pipelines.outline;
        self.line_pipeline = pipelines.line;
        self.decal_pipeline = create_decal_pipeline(
            &self.device,
            &self.shader,
            &self.decal_pipeline_layout,
            self.config.format,
            actual,
        );
        self.particle_pipeline = create_particle_pipeline(
            &self.device,
            &self.camera_bind_group_layout,
            self.config.format,
            actual,
        );
        actual
    }

    pub fn msaa_samples(&self) -> u32 {
        self.sample_count
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.config.width = width.max(1.0) as u32;
        self.config.height = height.max(1.0) as u32;
        self.surface.configure(&self.device, &self.config);
        self.msaa_color_view =
            create_msaa_color_view(&self.device, &self.config, self.sample_count);
        self.depth_view = create_depth_view(&self.device, &self.config, self.sample_count);
        self.write_camera();
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        if self.grass_shader_enabled {
            self.grass_field.process_pending(GRASS_CLUMPS_PER_FRAME);
            self.sync_grass_chunks();
            self.update_grass_fades();
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(JsValue::from_str("surface texture validation failed"));
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let (color_view, resolve_target) = match &self.msaa_color_view {
            Some(msaa) => (msaa, Some(&view)),
            None => (&view, None),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("track encoder"),
            });

        if self.grass_shader_enabled && !self.grass_chunks.is_empty() {
            let time = ((performance_now() - self.grass_start_ms).max(0.0) / 1000.0) as f32;
            let uniform = GrassUniform {
                time,
                wind_strength: 0.16,
                wind_speed: 1.2,
                wind_freq: 0.35,
                brightness: 1.0,
                translucency: 0.65,
                pad0: 0.0,
                pad1: 0.0,
            };
            self.queue
                .write_buffer(&self.grass_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        }

        if self.high_shadow_active() && self.shadow_dirty {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: if self.car_dynamic_separate() {
                        &self.car_shadow_view
                    } else {
                        &self.shadow_view
                    },
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for mesh in &self.meshes {
                if !self.mesh_required_in_high_shadow(mesh) {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
            self.shadow_dirty = false;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("track pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.62,
                            g: 0.72,
                            b: 0.80,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);

            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_pipeline(&self.outline_pipeline);
            for mesh in &self.meshes {
                if !mesh.visible {
                    continue;
                }
                let Some(outline) = &mesh.outline else {
                    continue;
                };
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.set_index_buffer(outline.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..outline.index_count, 0, 0..1);
            }

            pass.set_bind_group(
                3,
                if self.car_dynamic_separate() {
                    &self.car_shadow_bind_group
                } else {
                    &self.shadow_bind_group
                },
                &[],
            );
            pass.set_pipeline(&self.mesh_pipeline);
            for mesh in self
                .meshes
                .iter()
                .filter(|mesh| !mesh.is_decal && mesh.alpha >= 0.999)
            {
                if !mesh.visible {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                let texture_bind_group = if self.grass_texture_enabled {
                    mesh.grass_texture_bind_group
                        .as_ref()
                        .unwrap_or(&mesh.texture_bind_group)
                } else {
                    &mesh.texture_bind_group
                };
                pass.set_bind_group(2, texture_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }

            if self.grass_shader_enabled && !self.grass_chunks.is_empty() {
                pass.set_pipeline(&self.grass_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(
                    1,
                    if self.car_dynamic_separate() {
                        &self.car_shadow_bind_group
                    } else {
                        &self.shadow_bind_group
                    },
                    &[],
                );
                pass.set_bind_group(2, &self.grass_bind_group, &[]);
                pass.set_vertex_buffer(0, self.grass_vertex_buffer.slice(..));
                pass.set_index_buffer(self.grass_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                for chunk in self.grass_chunks.values() {
                    let Some(buffer) = &chunk.buffer else {
                        continue;
                    };
                    let Some(fade_bind_group) = &chunk.fade_bind_group else {
                        continue;
                    };
                    if chunk.count == 0 {
                        continue;
                    }
                    pass.set_bind_group(3, fade_bind_group, &[]);
                    pass.set_vertex_buffer(1, buffer.slice(..));
                    pass.draw_indexed(0..self.grass_index_count, 0, 0..chunk.count);
                }
                pass.set_bind_group(
                    3,
                    if self.car_dynamic_separate() {
                        &self.car_shadow_bind_group
                    } else {
                        &self.shadow_bind_group
                    },
                    &[],
                );
            }

            pass.set_pipeline(&self.transparent_pipeline);
            for mesh in self
                .meshes
                .iter()
                .filter(|mesh| !mesh.is_decal && mesh.alpha < 0.999)
            {
                if !mesh.visible {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                let texture_bind_group = if self.grass_texture_enabled {
                    mesh.grass_texture_bind_group
                        .as_ref()
                        .unwrap_or(&mesh.texture_bind_group)
                } else {
                    &mesh.texture_bind_group
                };
                pass.set_bind_group(2, texture_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }

            if self.decal_visible {
                pass.set_pipeline(&self.decal_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                for mesh in self.meshes.iter().filter(|mesh| mesh.is_decal) {
                    if !mesh.visible {
                        continue;
                    }
                    pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                    let texture_bind_group = if self.grass_texture_enabled {
                        mesh.grass_texture_bind_group
                            .as_ref()
                            .unwrap_or(&mesh.texture_bind_group)
                    } else {
                        &mesh.texture_bind_group
                    };
                    pass.set_bind_group(2, texture_bind_group, &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }

            if self.particle_count > 0 {
                pass.set_pipeline(&self.particle_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.particle_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.particle_instance_buffer.slice(..));
                pass.set_index_buffer(
                    self.particle_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..self.particle_index_count, 0, 0..self.particle_count);
            }

            if self.lines_visible {
                if let Some(line_buffer) = &self.line_buffer {
                    pass.set_pipeline(&self.line_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, line_buffer.slice(..));
                    pass.draw(0..self.line_count, 0..1);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);
        Ok(())
    }

    pub fn backend(&self) -> &str {
        &self.backend
    }

    pub fn surface_format(&self) -> &str {
        &self.surface_format
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.write_camera();
    }

    pub fn set_shadow_focus(&mut self, focus: [f32; 3], extent: f32) {
        self.shadow_focus = focus;
        self.env_shadow_extent = extent.max(1.0);
        self.shadow_extent = self.active_high_extent();
        if self.car_dynamic_separate() {
            let direction = car_shadow_direction(self.sun_direction);
            let length = car_shadow_length(self.sun_elevation, self.car_shadow_height);
            let origin = Vec3::from_array(focus) - direction * (length * 0.5);
            self.car_shadow_params = [
                origin.x,
                origin.z,
                (length * 0.55).max(4.0),
                (length * 1.05).max(6.0),
            ];
        }
        let next = shadow_view_proj(
            self.active_shadow_map_size(),
            Vec3::from_array(focus),
            self.shadow_extent,
            self.sun_direction,
        );
        if next != self.shadow_view_proj {
            self.shadow_view_proj = next;
            self.shadow_dirty = true;
            self.write_camera();
        }
    }

    pub fn set_shadow_tuning(&mut self, tuning: &ShadowTuning) {
        self.shadow_strength = tuning.strength.clamp(0.0, 1.0);
        self.shadow_depth_bias = tuning.depth_bias.clamp(0.0, 0.02);
        self.car_shadow_extent = tuning.car_extent.clamp(1.0, 100.0);
        self.car_shadow_max_extent = tuning
            .car_max_extent
            .max(self.car_shadow_extent)
            .clamp(1.0, 200.0);
        self.car_shadow_height = tuning.car_height.clamp(0.1, 20.0);
        self.shadow_extent = self.active_high_extent();
        self.shadow_view_proj = shadow_view_proj(
            self.active_shadow_map_size(),
            Vec3::from_array(self.shadow_focus),
            self.shadow_extent,
            self.sun_direction,
        );
        self.shadow_dirty = true;
        self.write_camera();
    }

    pub fn set_particle_render_tuning(&mut self, tuning: &ParticleRenderTuning) {
        self.particle_params = [
            tuning.near_fade_start,
            tuning.near_fade_end,
            tuning.radial_start,
            tuning.radial_end,
        ];
        self.particle_shade = [tuning.shade_min, tuning.shade_max, 0.0, 0.0];
        self.write_camera();
    }

    pub fn shadow_map_size(&self) -> u32 {
        self.active_shadow_map_size()
    }

    pub fn shadow_focus(&self) -> [f32; 3] {
        self.shadow_focus
    }

    pub fn sun_direction(&self) -> [f32; 3] {
        self.sun_direction.to_array()
    }

    pub fn sun_elevation(&self) -> f32 {
        self.sun_elevation
    }

    pub fn time_of_day(&self) -> f32 {
        self.sun_time
    }

    fn shadow_modes(&self) -> [f32; 4] {
        [
            if self.environment_mode == EnvironmentShadowMode::High {
                1.0
            } else {
                0.0
            },
            if self.environment_mode == EnvironmentShadowMode::Baked {
                1.0
            } else {
                0.0
            },
            if self.car_dynamic_separate() {
                1.0
            } else {
                0.0
            },
            if self.environment_mode == EnvironmentShadowMode::Hybrid {
                1.0
            } else {
                0.0
            },
        ]
    }

    fn effective_car_mode(&self) -> CarShadowMode {
        match self.car_shadow_mode {
            CarShadowMode::Dynamic if self.car_dynamic_supported => CarShadowMode::Dynamic,
            CarShadowMode::Dynamic => CarShadowMode::Silhouette,
            mode => mode,
        }
    }

    fn car_dynamic_separate(&self) -> bool {
        self.effective_car_mode() == CarShadowMode::Dynamic
            && self.environment_mode != EnvironmentShadowMode::High
            && self.environment_mode != EnvironmentShadowMode::Hybrid
    }

    pub fn car_shadow_separate(&self) -> bool {
        self.car_dynamic_separate()
    }

    pub fn car_shadow_focus(&self, base: [f32; 3]) -> [f32; 3] {
        let direction = car_shadow_direction(self.sun_direction);
        let offset = car_shadow_length(self.sun_elevation, self.car_shadow_height) * 0.5;
        (Vec3::from_array(base) + direction * offset).to_array()
    }

    fn active_shadow_map_size(&self) -> u32 {
        if self.car_dynamic_separate() {
            self.car_shadow_map_size
        } else {
            self.shadow_map_size
        }
    }

    pub fn car_shadow_map_size(&self) -> u32 {
        self.car_shadow_map_size
    }

    fn high_shadow_active(&self) -> bool {
        matches!(
            self.environment_mode,
            EnvironmentShadowMode::High | EnvironmentShadowMode::Hybrid
        ) || self.car_dynamic_separate()
    }

    fn active_high_extent(&self) -> f32 {
        if matches!(
            self.environment_mode,
            EnvironmentShadowMode::High | EnvironmentShadowMode::Hybrid
        ) {
            self.env_shadow_extent
        } else if self.car_dynamic_separate() {
            car_shadow_extent_for_elevation(
                self.sun_elevation,
                self.car_shadow_extent,
                self.car_shadow_max_extent,
                self.car_shadow_height,
            )
        } else {
            self.env_shadow_extent
        }
    }

    fn mesh_required_in_high_shadow(&self, mesh: &GpuMesh) -> bool {
        if !mesh.visible {
            return false;
        }
        if matches!(
            self.environment_mode,
            EnvironmentShadowMode::High | EnvironmentShadowMode::Hybrid
        ) {
            mesh.casts_static_shadow
                || (self.effective_car_mode() == CarShadowMode::Dynamic
                    && mesh.casts_dynamic_shadow)
        } else if self.car_dynamic_separate() {
            mesh.casts_dynamic_shadow
        } else {
            false
        }
    }

    pub fn set_environment_shadow_mode(&mut self, mode: EnvironmentShadowMode) {
        self.environment_mode = mode;
        self.shadow_extent = self.active_high_extent();
        self.shadow_view_proj = shadow_view_proj(
            self.active_shadow_map_size(),
            Vec3::from_array(self.shadow_focus),
            self.shadow_extent,
            self.sun_direction,
        );
        self.shadow_dirty = true;
        if matches!(
            mode,
            EnvironmentShadowMode::Baked | EnvironmentShadowMode::Hybrid
        ) && !self.baked_valid
        {
            self.bake_environment();
        }
        self.write_camera();
    }

    pub fn set_baked_region(&mut self, center: [f32; 3], half_extent: f32, ground_y: f32) {
        self.baked_region = [center[0], center[2], half_extent.max(1.0), ground_y];
        self.baked_valid = false;
    }

    pub fn environment_shadow_mode(&self) -> EnvironmentShadowMode {
        self.environment_mode
    }

    pub fn set_car_shadow_mode(&mut self, mode: CarShadowMode, dynamic_supported: bool) {
        self.car_shadow_mode = mode;
        self.car_dynamic_supported = dynamic_supported;
        self.decal_visible = self.effective_car_mode() == CarShadowMode::Silhouette;
        self.shadow_extent = self.active_high_extent();
        self.shadow_view_proj = shadow_view_proj(
            self.active_shadow_map_size(),
            Vec3::from_array(self.shadow_focus),
            self.shadow_extent,
            self.sun_direction,
        );
        self.shadow_dirty = true;
        self.write_camera();
    }

    pub fn car_shadow_mode(&self) -> CarShadowMode {
        self.car_shadow_mode
    }

    pub fn effective_car_shadow_mode(&self) -> CarShadowMode {
        self.effective_car_mode()
    }

    pub fn baked_resolution(&self) -> u32 {
        self.baked_resolution
    }

    pub fn baked_region_half_extent(&self) -> f32 {
        self.baked_region[2]
    }

    pub fn last_bake_ms(&self) -> f64 {
        self.last_bake_ms
    }

    pub fn set_time_of_day(&mut self, hours: f32) -> f64 {
        self.sun_time = hours.clamp(6.0, 20.0);
        let (direction, elevation) = sun_for_time(self.sun_time);
        self.sun_direction = direction;
        self.sun_elevation = elevation;
        let (horizon, zenith) = sky_colors(elevation);
        self.sky_horizon = horizon;
        self.sky_zenith = zenith;
        self.shadow_extent = self.active_high_extent();
        self.shadow_view_proj = shadow_view_proj(
            self.active_shadow_map_size(),
            Vec3::from_array(self.shadow_focus),
            self.shadow_extent,
            self.sun_direction,
        );
        self.shadow_dirty = true;
        self.baked_valid = false;
        if matches!(
            self.environment_mode,
            EnvironmentShadowMode::Baked | EnvironmentShadowMode::Hybrid
        ) {
            self.bake_environment();
            self.write_camera();
            self.last_bake_ms
        } else {
            0.0
        }
    }

    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.set_environment_shadow_mode(if enabled {
            EnvironmentShadowMode::High
        } else {
            EnvironmentShadowMode::Off
        });
    }

    pub fn shadows_enabled(&self) -> bool {
        self.environment_mode != EnvironmentShadowMode::Off
            || self.car_dynamic_separate()
            || self.effective_car_mode() == CarShadowMode::Silhouette
    }

    fn bake_environment(&mut self) {
        let start = performance_now();
        let resolution = self.baked_resolution;
        let region = self.baked_region;
        let half_extent = region[2];
        let center = Vec3::new(region[0], region[3], region[1]);
        let bake_view_proj = shadow_view_proj(resolution, center, half_extent, self.sun_direction);
        self.baked_view_proj = bake_view_proj;
        self.baked_shadow_params = [
            1.0 / resolution as f32,
            2.0 * half_extent / resolution as f32,
            self.shadow_depth_bias,
            0.0,
        ];
        let bake_uniform = camera_uniform(
            &self.camera,
            self.config.width,
            self.config.height,
            bake_view_proj,
            resolution,
            half_extent,
            true,
            self.sun_direction,
            self.sky_horizon,
            self.sky_zenith,
            bake_view_proj,
            self.baked_shadow_params,
            self.shadow_modes(),
            [0.0; 4],
            self.shadow_strength,
            self.shadow_depth_bias,
            self.particle_params,
            self.particle_shade,
        );
        self.queue.write_buffer(
            &self.bake_camera_buffer,
            0,
            bytemuck::bytes_of(&bake_uniform),
        );

        let depth = create_shadow_map_for_size(&self.device, resolution);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bake shadow encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bake static shadow depth"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.bake_camera_bind_group, &[]);
            for mesh in &self.meshes {
                if !mesh.visible || !mesh.casts_static_shadow || mesh.is_decal {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.baked_view = depth;
        self.baked_valid = true;
        self.rebuild_shadow_bind_group();
        self.last_bake_ms = performance_now() - start;
        self.write_camera();
    }

    fn rebuild_shadow_bind_group(&mut self) {
        self.shadow_bind_group = create_shadow_bind_group(
            &self.device,
            &self.shadow_bind_group_layout,
            &self.shadow_view,
            &self.shadow_sampler,
            &self.baked_view,
            "shadow bind group",
        );
        self.car_shadow_bind_group = create_shadow_bind_group(
            &self.device,
            &self.shadow_bind_group_layout,
            &self.car_shadow_view,
            &self.shadow_sampler,
            &self.baked_view,
            "car shadow bind group",
        );
    }

    pub fn set_mesh_model(&mut self, index: usize, model: Mat4) {
        let grass_texture_enabled = self.grass_texture_enabled;
        if let Some(mesh) = self.meshes.get_mut(index) {
            if mesh.model == model {
                return;
            }
            mesh.model = model;
            if mesh.casts_static_shadow || mesh.casts_dynamic_shadow {
                self.shadow_dirty = true;
            }
            let uniform = ModelUniform {
                model: model.to_cols_array_2d(),
                alpha: model_alpha(mesh, grass_texture_enabled),
                emissive: [mesh.emissive[0], mesh.emissive[1], mesh.emissive[2], 0.0],
            };
            self.queue
                .write_buffer(&mesh.model_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_shadow_decal(
        &mut self,
        index: usize,
        base: [f32; 3],
        normal: [f32; 3],
        yaw: f32,
        width: f32,
        length: f32,
        light: [f32; 3],
        height: f32,
    ) {
        let model = shadow_decal_model(
            Vec3::from_array(base),
            Vec3::from_array(normal),
            yaw,
            width,
            length,
            Vec3::from_array(light),
            height,
        );
        self.set_mesh_model(index, model);
    }

    pub fn set_mesh_visible(&mut self, index: usize, visible: bool) {
        if let Some(mesh) = self.meshes.get_mut(index) {
            if mesh.visible != visible {
                mesh.visible = visible;
                self.shadow_dirty = true;
            }
        }
    }

    pub fn set_static_shadow_range(&mut self, range: Range<usize>, enabled: bool) {
        let mut changed = false;
        for mesh in self.meshes.iter_mut().take(range.end).skip(range.start) {
            if mesh.shadow_static_base != enabled
                || mesh.casts_static_shadow != (enabled && mesh.alpha >= 0.999)
                || mesh.shadow_dynamic_base
                || mesh.casts_dynamic_shadow
            {
                changed = true;
            }
            mesh.shadow_static_base = enabled;
            mesh.casts_static_shadow = enabled && mesh.alpha >= 0.999;
            mesh.shadow_dynamic_base = false;
            mesh.casts_dynamic_shadow = false;
        }
        if !changed {
            return;
        }
        self.shadow_dirty = true;
        self.baked_valid = false;
        if self.environment_mode == EnvironmentShadowMode::Baked {
            self.bake_environment();
        } else {
            self.write_camera();
        }
    }

    pub fn set_mesh_texture(&mut self, index: usize, texture: &SceneTexture) {
        if texture.width == 0 || texture.height == 0 {
            return;
        }
        if texture.rgba8.len() != (texture.width * texture.height * 4) as usize {
            return;
        }
        let view = create_scene_texture(&self.device, &self.queue, texture, "runtime");
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("runtime mesh texture bind group"),
            layout: &self.mesh_texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.mesh_sampler),
                },
            ],
        });
        if let Some(mesh) = self.meshes.get_mut(index) {
            mesh.texture_bind_group = bind_group;
        }
    }

    pub fn set_mesh_alpha(&mut self, index: usize, alpha: f32) {
        let grass_texture_enabled = self.grass_texture_enabled;
        if let Some(mesh) = self.meshes.get_mut(index) {
            if mesh.alpha != alpha {
                mesh.alpha = alpha;
                mesh.casts_static_shadow = mesh.shadow_static_base && alpha >= 0.999;
                mesh.casts_dynamic_shadow = mesh.shadow_dynamic_base && alpha >= 0.999;
                self.shadow_dirty = true;
            }
            let offset = std::mem::offset_of!(ModelUniform, alpha) as u64;
            self.queue.write_buffer(
                &mesh.model_buffer,
                offset,
                bytemuck::bytes_of(&model_alpha(mesh, grass_texture_enabled)),
            );
        }
    }

    pub fn set_mesh_emissive(&mut self, index: usize, emissive: [f32; 3]) {
        let Some(mesh) = self.meshes.get_mut(index) else {
            return;
        };
        if mesh.emissive == emissive {
            return;
        }
        mesh.emissive = emissive;
        let offset = std::mem::offset_of!(ModelUniform, emissive) as u64;
        self.queue.write_buffer(
            &mesh.model_buffer,
            offset,
            bytemuck::bytes_of(&[emissive[0], emissive[1], emissive[2], 0.0]),
        );
    }

    fn write_camera(&self) {
        let uniform = camera_uniform(
            &self.camera,
            self.config.width,
            self.config.height,
            self.shadow_view_proj,
            self.active_shadow_map_size(),
            self.shadow_extent,
            self.environment_mode != EnvironmentShadowMode::Off || self.car_dynamic_separate(),
            self.sun_direction,
            self.sky_horizon,
            self.sky_zenith,
            self.baked_view_proj,
            self.baked_shadow_params,
            self.shadow_modes(),
            self.car_shadow_params,
            self.shadow_strength,
            self.shadow_depth_bias,
            self.particle_params,
            self.particle_shade,
        );
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}

#[cfg(target_arch = "wasm32")]
fn create_surface(
    instance: &wgpu::Instance,
    canvas: web_sys::HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, JsValue> {
    instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|error| JsValue::from_str(&format!("failed to create surface: {error:?}")))
}

#[cfg(not(target_arch = "wasm32"))]
fn create_surface(
    _instance: &wgpu::Instance,
    _canvas: web_sys::HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, JsValue> {
    Err(JsValue::from_str(
        "sim-render is only supported on wasm32 targets",
    ))
}

fn create_mesh(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mesh: &MeshData,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    texture_resources: &MeshTextureResources<'_>,
) -> GpuMesh {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} vertices", mesh.name)),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} indices", mesh.name)),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    let grass_texture_enabled = if mesh.grass_texture.is_some() {
        1.0
    } else {
        0.0
    };
    let model_uniform = ModelUniform {
        model: mesh.model.to_cols_array_2d(),
        alpha: [mesh.alpha, grass_texture_enabled, 0.0, 0.0],
        emissive: [mesh.emissive[0], mesh.emissive[1], mesh.emissive[2], 0.0],
    };
    let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} model", mesh.name)),
        contents: bytemuck::bytes_of(&model_uniform),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{} model bind group", mesh.name)),
        layout: model_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: model_buffer.as_entire_binding(),
        }],
    });

    let texture_bind_group = create_mesh_texture_bind_group(
        device,
        queue,
        texture_resources,
        &format!("{} texture bind group", mesh.name),
        mesh.texture,
    );
    let grass_texture_bind_group = mesh.grass_texture.map(|index| {
        create_mesh_texture_bind_group(
            device,
            queue,
            texture_resources,
            &format!("{} grass texture bind group", mesh.name),
            Some(index),
        )
    });

    let outline = mesh.outline.as_ref().map(|outline| {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} outline vertices", mesh.name)),
            contents: bytemuck::cast_slice(&outline.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} outline indices", mesh.name)),
            contents: bytemuck::cast_slice(&outline.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        GpuOutline {
            vertex_buffer,
            index_buffer,
            index_count: outline.indices.len() as u32,
        }
    });

    GpuMesh {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        model_buffer,
        model_bind_group,
        texture_bind_group,
        grass_texture_bind_group,
        outline,
        visible: true,
        alpha: mesh.alpha,
        emissive: mesh.emissive,
        casts_static_shadow: mesh.casts_static_shadow && mesh.alpha >= 0.999,
        casts_dynamic_shadow: mesh.casts_dynamic_shadow && mesh.alpha >= 0.999,
        shadow_static_base: mesh.casts_static_shadow,
        shadow_dynamic_base: mesh.casts_dynamic_shadow,
        is_decal: mesh.is_decal,
        model: mesh.model,
    }
}

fn create_mesh_texture_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &MeshTextureResources<'_>,
    label: &str,
    index: Option<usize>,
) -> wgpu::BindGroup {
    let owned_texture;
    let texture_view = if let Some(texture) = index.and_then(|index| resources.textures.get(index))
    {
        owned_texture = create_scene_texture(device, queue, texture, label);
        &owned_texture
    } else {
        resources.white_view
    };
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: resources.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(resources.sampler),
            },
        ],
    })
}

fn create_white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("white texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn mip_level_count(width: u32, height: u32) -> u32 {
    let max_dimension = width.max(height);
    u32::BITS - max_dimension.leading_zeros()
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5) as u8
}

fn downsample_rgba8(
    rgba8: &[u8],
    width: u32,
    height: u32,
    to_linear: &[f32; 256],
    to_srgb: &[u8; 4096],
) -> (Vec<u8>, u32, u32) {
    let next_width = (width / 2).max(1);
    let next_height = (height / 2).max(1);
    let mut next = vec![0u8; (next_width * next_height * 4) as usize];
    for y in 0..next_height {
        let y0 = (y * 2).min(height - 1);
        let y1 = (y * 2 + 1).min(height - 1);
        for x in 0..next_width {
            let x0 = (x * 2).min(width - 1);
            let x1 = (x * 2 + 1).min(width - 1);
            for channel in 0..3 {
                let sum = to_linear[rgba8[((y0 * width + x0) * 4 + channel) as usize] as usize]
                    + to_linear[rgba8[((y0 * width + x1) * 4 + channel) as usize] as usize]
                    + to_linear[rgba8[((y1 * width + x0) * 4 + channel) as usize] as usize]
                    + to_linear[rgba8[((y1 * width + x1) * 4 + channel) as usize] as usize];
                let linear = sum * 0.25;
                let index = (linear.clamp(0.0, 1.0) * 4095.0 + 0.5) as usize;
                next[((y * next_width + x) * 4 + channel) as usize] = to_srgb[index];
            }
            let alpha = rgba8[((y0 * width + x0) * 4 + 3) as usize] as u32
                + rgba8[((y0 * width + x1) * 4 + 3) as usize] as u32
                + rgba8[((y1 * width + x0) * 4 + 3) as usize] as u32
                + rgba8[((y1 * width + x1) * 4 + 3) as usize] as u32;
            next[((y * next_width + x) * 4 + 3) as usize] = ((alpha + 2) / 4) as u8;
        }
    }
    (next, next_width, next_height)
}

fn mip_chain(rgba8: &[u8], width: u32, height: u32) -> Vec<(u32, u32, Vec<u8>)> {
    let level_count = mip_level_count(width, height);
    let to_linear = std::array::from_fn(|value| srgb_to_linear(value as u8));
    let to_srgb = std::array::from_fn(|value| linear_to_srgb(value as f32 / 4095.0));
    let mut levels = Vec::with_capacity(level_count as usize);
    levels.push((width, height, rgba8.to_vec()));
    let mut current_width = width;
    let mut current_height = height;
    for _ in 1..level_count {
        let (next, next_width, next_height) = {
            let source = &levels.last().expect("mip chain has a base level").2;
            downsample_rgba8(source, current_width, current_height, &to_linear, &to_srgb)
        };
        levels.push((next_width, next_height, next));
        current_width = next_width;
        current_height = next_height;
    }
    levels
}

fn create_scene_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &SceneTexture,
    label: &str,
) -> wgpu::TextureView {
    let levels = mip_chain(&texture.rgba8, texture.width, texture.height);
    let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("{label} texture")),
        size: wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (width, height, rgba8)) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(*height),
            },
            wgpu::Extent3d {
                width: *width,
                height: *height,
                depth_or_array_layers: 1,
            },
        );
    }
    gpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

#[allow(clippy::too_many_arguments)]
fn camera_uniform(
    camera: &Camera,
    width: u32,
    height: u32,
    shadow_view_proj: Mat4,
    shadow_map_size: u32,
    shadow_extent: f32,
    shadows_enabled: bool,
    sun_direction: Vec3,
    sky_horizon: [f32; 4],
    sky_zenith: [f32; 4],
    baked_view_proj: Mat4,
    baked_shadow_params: [f32; 4],
    shadow_modes: [f32; 4],
    car_shadow_params: [f32; 4],
    shadow_strength: f32,
    shadow_depth_bias: f32,
    particle_params: [f32; 4],
    particle_shade: [f32; 4],
) -> CameraUniform {
    let aspect = width as f32 / height as f32;
    let view_proj = camera.view_proj(aspect).to_cols_array_2d();
    let eye = camera.eye;
    let light = sun_direction;
    let shadow_texel = 1.0 / shadow_map_size as f32;
    let shadow_world_texel = 2.0 * shadow_extent / shadow_map_size as f32;
    CameraUniform {
        view_proj,
        camera_pos: [eye[0], eye[1], eye[2], 1.0],
        light_dir: [light.x, light.y, light.z, 0.0],
        light_view_proj: shadow_view_proj.to_cols_array_2d(),
        shadow_params: [
            shadow_texel,
            shadow_world_texel,
            if shadows_enabled {
                shadow_strength
            } else {
                0.0
            },
            shadow_depth_bias,
        ],
        sky_horizon,
        sky_zenith,
        baked_view_proj: baked_view_proj.to_cols_array_2d(),
        baked_shadow_params,
        shadow_modes,
        car_shadow_params,
        camera_right: {
            let eye = Vec3::from_array(camera.eye);
            let forward = (Vec3::from_array(camera.target) - eye).normalize_or(Vec3::NEG_Z);
            let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
            [right.x, right.y, right.z, 0.0]
        },
        camera_up: {
            let eye = Vec3::from_array(camera.eye);
            let forward = (Vec3::from_array(camera.target) - eye).normalize_or(Vec3::NEG_Z);
            let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(forward).normalize_or(Vec3::Y);
            [up.x, up.y, up.z, 0.0]
        },
        particle_params,
        particle_shade,
    }
}

fn sun_for_time(hours: f32) -> (Vec3, f32) {
    let hours = hours.clamp(6.0, 20.0);
    let t = (hours - 6.0) / 14.0;
    let elevation = (8.0 + 64.0 * (std::f32::consts::PI * t).sin()).to_radians();
    let azimuth = (-75.0 + 150.0 * t).to_radians();
    let to_sun = Vec3::new(
        elevation.cos() * azimuth.cos(),
        elevation.sin(),
        elevation.cos() * azimuth.sin(),
    );
    (-to_sun, elevation)
}

fn sky_colors(elevation: f32) -> ([f32; 4], [f32; 4]) {
    let day = ((elevation.to_degrees() - 12.0) / 55.0).clamp(0.0, 1.0);
    let warm_horizon = Vec3::new(0.92, 0.56, 0.36);
    let warm_zenith = Vec3::new(0.18, 0.22, 0.38);
    let day_horizon = Vec3::new(0.62, 0.72, 0.80);
    let day_zenith = Vec3::new(0.12, 0.30, 0.55);
    let horizon = warm_horizon.lerp(day_horizon, day);
    let zenith = warm_zenith.lerp(day_zenith, day);
    (
        [horizon.x, horizon.y, horizon.z, 1.0],
        [zenith.x, zenith.y, zenith.z, 1.0],
    )
}

fn shadow_decal_model(
    base: Vec3,
    normal: Vec3,
    yaw: f32,
    width: f32,
    length: f32,
    light: Vec3,
    height: f32,
) -> Mat4 {
    let normal = normal.normalize_or(Vec3::Y);
    let light = light.normalize_or(Vec3::NEG_Y);
    let width = width.max(0.1);
    let length = length.max(0.1);
    let rotation = Quat::from_rotation_arc(Vec3::Y, normal) * Quat::from_rotation_y(yaw);
    let horizontal = Vec3::new(light.x, 0.0, light.z);
    let direction = horizontal.normalize_or(Vec3::X);
    let extension = height * horizontal.length() / (-light.y).max(0.1);
    let right = rotation * Vec3::X;
    let forward = rotation * Vec3::NEG_Z;
    let half_extent =
        0.5 * (width * direction.dot(right).abs() + length * direction.dot(forward).abs());
    let stretch = (1.0 + extension / (2.0 * half_extent.max(0.1))).clamp(1.0, 4.0);
    let anchor = base + normal * 0.015 + direction * half_extent * (stretch - 1.0);
    let amount = stretch - 1.0;
    let stretch_matrix = Mat4::from_cols(
        Vec4::new(
            1.0 + amount * direction.x * direction.x,
            amount * direction.y * direction.x,
            amount * direction.z * direction.x,
            0.0,
        ),
        Vec4::new(
            amount * direction.x * direction.y,
            1.0 + amount * direction.y * direction.y,
            amount * direction.z * direction.y,
            0.0,
        ),
        Vec4::new(
            amount * direction.x * direction.z,
            amount * direction.y * direction.z,
            1.0 + amount * direction.z * direction.z,
            0.0,
        ),
        Vec4::W,
    );
    let local =
        Mat4::from_scale_rotation_translation(Vec3::new(width, 1.0, length), rotation, Vec3::ZERO);
    Mat4::from_translation(anchor) * stretch_matrix * local
}

fn shadow_view_proj(map_size: u32, focus: Vec3, extent: f32, light: Vec3) -> Mat4 {
    let up = if light.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let right = light.cross(up).normalize_or_zero();
    let light_up = right.cross(light).normalize_or_zero();
    let texel = (extent * 2.0) / map_size as f32;
    let snapped = focus
        + right * ((focus.dot(right) / texel).round() * texel - focus.dot(right))
        + light_up * ((focus.dot(light_up) / texel).round() * texel - focus.dot(light_up));
    let distance = extent * 3.0;
    let eye = snapped - light * distance;
    let view = Mat4::look_at_rh(eye, snapped, up);
    let projection = Mat4::orthographic_rh(
        -extent,
        extent,
        -extent,
        extent,
        0.1,
        distance + extent * 2.0,
    );
    projection * view
}

fn create_shadow_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shadow_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    baked_view: &wgpu::TextureView,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(baked_view),
            },
        ],
    })
}

fn preferred_shadow_map_size(backend: &str) -> u32 {
    if backend == "Gl" || small_viewport() {
        1024
    } else {
        2048
    }
}

fn small_viewport() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window();
        let width = window
            .as_ref()
            .and_then(|window| window.inner_width().ok())
            .and_then(|value| value.as_f64())
            .unwrap_or(1280.0);
        let height = window
            .as_ref()
            .and_then(|window| window.inner_height().ok())
            .and_then(|value| value.as_f64())
            .unwrap_or(720.0);
        width.min(height) <= 600.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

fn create_shadow_map(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
    create_shadow_map_for_size(device, size)
}

fn create_shadow_map_for_size(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow map"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn performance_now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.performance())
            .map(|performance| performance.now())
            .unwrap_or(0.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

fn car_shadow_extent_for_elevation(
    elevation: f32,
    extent: f32,
    max_extent: f32,
    height: f32,
) -> f32 {
    let elevation = elevation.max(0.05);
    let length = height / elevation.tan();
    (length * 0.5 + 4.0).clamp(extent, max_extent)
}

fn car_shadow_direction(light: Vec3) -> Vec3 {
    Vec3::new(light.x, 0.0, light.z).normalize_or(Vec3::X)
}

fn car_shadow_length(elevation: f32, height: f32) -> f32 {
    height / elevation.max(0.05).tan()
}

fn resolve_texture_size(requested: u32, fallback: u32, max: u32) -> u32 {
    let requested = if requested == 0 {
        fallback
    } else {
        requested.max(256)
    };
    requested.min(max.max(256))
}

fn preferred_car_shadow_map_size(_backend: &str, adapter_limits: &wgpu::Limits) -> u32 {
    let preferred = if small_viewport() { 2048 } else { 4096 };
    preferred.min(adapter_limits.max_texture_dimension_2d)
}

fn preferred_baked_resolution(backend: &str, adapter_limits: &wgpu::Limits) -> u32 {
    if backend == "Gl" || small_viewport() {
        2048
    } else {
        adapter_limits.max_texture_dimension_2d.min(4096)
    }
}

fn create_baked_placeholder(device: &wgpu::Device) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("baked shadow placeholder"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParticleVertex {
    position: [f32; 3],
    uv: [f32; 2],
}

fn create_particle_mesh(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let vertices = [
        ParticleVertex {
            position: [-1.0, -1.0, 0.0],
            uv: [-1.0, -1.0],
        },
        ParticleVertex {
            position: [1.0, -1.0, 0.0],
            uv: [1.0, -1.0],
        },
        ParticleVertex {
            position: [1.0, 1.0, 0.0],
            uv: [1.0, 1.0],
        },
        ParticleVertex {
            position: [-1.0, 1.0, 0.0],
            uv: [-1.0, 1.0],
        },
    ];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("particle puff vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("particle puff indices"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn create_particle_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("particle shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("particle.wgsl"))),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("particle pipeline layout"),
        bind_group_layouts: &[Some(camera_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("particle pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_particle"),
            compilation_options: Default::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &PARTICLE_MESH_ATTRIBUTES,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &PARTICLE_INSTANCE_ATTRIBUTES,
                }),
            ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_particle"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_grass_mesh(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    const SEGMENTS: usize = 3;
    let mut vertices = Vec::with_capacity(SEGMENTS * 2 + 1);
    for segment in 0..SEGMENTS {
        let t = segment as f32 / SEGMENTS as f32;
        let half_width = 0.5 * (1.0 - t).powf(1.2);
        vertices.push(GrassVertex {
            position: [-half_width, t, 0.0],
        });
        vertices.push(GrassVertex {
            position: [half_width, t, 0.0],
        });
    }
    vertices.push(GrassVertex {
        position: [0.0, 1.0, 0.0],
    });

    let mut indices = Vec::with_capacity((SEGMENTS * 2 - 1) * 3);
    for segment in 0..SEGMENTS - 1 {
        let left = (segment * 2) as u32;
        let right = left + 1;
        let next_left = left + 2;
        let next_right = left + 3;
        indices.extend_from_slice(&[left, next_left, right, right, next_left, next_right]);
    }
    let last_left = ((SEGMENTS - 1) * 2) as u32;
    indices.extend_from_slice(&[last_left, (SEGMENTS * 2) as u32, last_left + 1]);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("grass blade vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("grass blade indices"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn create_grass_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grass pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_grass"),
            compilation_options: Default::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GrassVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &GRASS_VERTEX_ATTRIBUTES,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GrassInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &GRASS_INSTANCE_ATTRIBUTES,
                }),
            ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_grass"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_decal_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("car decal pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_mesh"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_decal"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_shadow_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_shadow"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            ..Default::default()
        },
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

fn supported_samples(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> Vec<u32> {
    let color_flags = adapter.get_texture_format_features(format).flags;
    let depth_flags = adapter.get_texture_format_features(DEPTH_FORMAT).flags;
    [1, 2, 4]
        .into_iter()
        .filter(|count| {
            *count == 1
                || (color_flags.sample_count_supported(*count)
                    && color_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
                    && depth_flags.sample_count_supported(*count))
        })
        .collect()
}

fn select_sample_count(supported: &[u32], requested: u32) -> u32 {
    supported
        .iter()
        .copied()
        .filter(|count| *count <= requested)
        .max()
        .or_else(|| supported.first().copied())
        .unwrap_or(1)
}

fn create_msaa_color_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> Option<wgpu::TextureView> {
    if sample_count <= 1 {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa color texture"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

fn create_depth_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth texture"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

struct Pipelines {
    sky: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    outline: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
}

#[allow(clippy::too_many_arguments)]
fn create_pipelines(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    mesh_layout: &wgpu::PipelineLayout,
    outline_layout: &wgpu::PipelineLayout,
    line_layout: &wgpu::PipelineLayout,
    camera_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> Pipelines {
    let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sky pipeline layout"),
        bind_group_layouts: &[Some(camera_layout)],
        immediate_size: 0,
    });
    let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sky pipeline"),
        layout: Some(&sky_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_sky"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_sky"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let mesh = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh pipeline"),
        layout: Some(mesh_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_mesh"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_mesh"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let outline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("outline pipeline"),
        layout: Some(outline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_outline"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<OutlineVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &OUTLINE_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 1,
                slope_scale: 1.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_outline"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let transparent = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("transparent mesh pipeline"),
        layout: Some(mesh_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_mesh"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_mesh"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let line = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("line pipeline"),
        layout: Some(line_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_line"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<LineVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &LINE_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_line"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    Pipelines {
        sky,
        mesh,
        outline,
        transparent,
        line,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        car_shadow_extent_for_elevation, mip_chain, mip_level_count, preferred_car_shadow_map_size,
        preferred_shadow_map_size, select_sample_count, shadow_decal_model, shadow_view_proj,
    };
    use crate::tuning::ShadowTuning;
    use glam::{Vec3, Vec4};

    #[test]
    fn mip_chain_halves_to_one_pixel() {
        assert_eq!(mip_level_count(256, 256), 9);
        assert_eq!(mip_level_count(300, 100), 9);
        let levels = mip_chain(&[255u8; 4 * 4 * 4], 4, 4);
        assert_eq!(levels.len(), 3);
        assert_eq!((levels[1].0, levels[1].1), (2, 2));
        assert_eq!((levels[2].0, levels[2].1), (1, 1));
        assert!(levels[1].2.iter().all(|value| *value == 255));
    }

    #[test]
    fn mip_chain_downsamples_in_linear_light() {
        let mut rgba8 = vec![0u8; 2 * 2 * 4];
        for (index, pixel) in rgba8.chunks_exact_mut(4).enumerate() {
            let value = if index < 2 { 0 } else { 255 };
            pixel.fill(value);
            pixel[3] = 255;
        }
        let levels = mip_chain(&rgba8, 2, 2);
        let mid = levels[1].2[0];
        assert!((187..=189).contains(&mid), "mid={mid}");
    }

    #[test]
    fn sample_count_prefers_requested_with_fallback() {
        assert_eq!(select_sample_count(&[1, 2, 4], 4), 4);
        assert_eq!(select_sample_count(&[1, 2, 4], 2), 2);
        assert_eq!(select_sample_count(&[1, 2, 4], 1), 1);
        assert_eq!(select_sample_count(&[1, 4], 2), 1);
        assert_eq!(select_sample_count(&[1, 2], 4), 2);
        assert_eq!(select_sample_count(&[1], 4), 1);
    }

    #[test]
    fn shadow_projection_centers_on_focus() {
        let focus = Vec3::new(11.0, 1.0, -7.0);
        let projection = shadow_view_proj(2048, focus, 60.0, super::sun_for_time(15.0).0);
        let clip = projection * Vec4::new(focus.x, focus.y, focus.z, 1.0);
        let ndc = clip.truncate() / clip.w;

        assert!(ndc.x.abs() < 0.002, "x={}", ndc.x);
        assert!(ndc.y.abs() < 0.002, "y={}", ndc.y);
        assert!((0.0..=1.0).contains(&ndc.z), "z={}", ndc.z);
    }

    #[test]
    fn shadow_decal_stretches_from_car_edge() {
        let car = (Vec3::ZERO, Vec3::Y, 0.0, 2.0, 4.0);
        let evening = shadow_decal_model(
            car.0,
            car.1,
            car.2,
            car.3,
            car.4,
            Vec3::new(1.0, -0.2, 0.0).normalize(),
            0.7,
        );
        let near = evening.transform_point3(Vec3::new(-0.5, 0.0, -0.5));
        let far = evening.transform_point3(Vec3::new(0.5, 0.0, -0.5));
        assert!((near.x + 1.0).abs() < 1e-5, "near edge moved: {near:?}");
        assert!(far.x > 3.0, "shadow did not stretch: {far:?}");

        let noon = shadow_decal_model(
            car.0,
            car.1,
            car.2,
            car.3,
            car.4,
            Vec3::new(0.2, -1.0, 0.0).normalize(),
            0.7,
        );
        let near = noon.transform_point3(Vec3::new(-0.5, 0.0, -0.5));
        let far = noon.transform_point3(Vec3::new(0.5, 0.0, -0.5));
        assert!((near.x + 1.0).abs() < 1e-5);
        assert!((far.x - 1.0).abs() < 0.2, "noon shadow too long: {far:?}");
    }

    #[test]
    fn shadow_map_size_is_lower_on_webgl() {
        assert_eq!(preferred_shadow_map_size("Gl"), 1024);
        assert_eq!(preferred_shadow_map_size("BrowserWebGpu"), 2048);
    }

    #[test]
    fn car_shadow_map_is_higher_resolution() {
        let limits = wgpu::Limits::default();
        assert_eq!(preferred_car_shadow_map_size("Gl", &limits), 4096);
        assert_eq!(
            preferred_car_shadow_map_size("BrowserWebGpu", &limits),
            4096
        );
    }

    #[test]
    fn car_shadow_extent_grows_at_low_sun() {
        let shadow = ShadowTuning::default();
        let noon = car_shadow_extent_for_elevation(
            super::sun_for_time(15.0).1,
            shadow.car_extent,
            shadow.car_max_extent,
            shadow.car_height,
        );
        let evening = car_shadow_extent_for_elevation(
            super::sun_for_time(20.0).1,
            shadow.car_extent,
            shadow.car_max_extent,
            shadow.car_height,
        );
        assert!(evening > noon);
        assert!(evening <= shadow.car_max_extent);
        assert!(noon >= shadow.car_extent);
    }
}
