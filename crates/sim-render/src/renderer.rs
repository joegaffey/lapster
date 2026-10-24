use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wasm_bindgen::JsValue;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::scene::{LineVertex, MeshData, MeshVertex, OutlineVertex, Scene, SceneTexture};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

static MESH_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2];
static OUTLINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
static LINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    light_dir: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    alpha: [f32; 4],
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

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
    outline: Option<GpuOutline>,
    visible: bool,
    alpha: f32,
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
    sky_pipeline: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    meshes: Vec<GpuMesh>,
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

        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("track shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("track.wgsl"))),
        });

        let camera_uniform = camera_uniform(&scene.camera, config.width, config.height);
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

        let model_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model bind group layout"),
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

        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("track pipeline layout"),
            bind_group_layouts: &[
                Some(&camera_bind_group_layout),
                Some(&model_bind_group_layout),
                Some(&texture_bind_group_layout),
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

        let pipelines = create_pipelines(
            &device,
            &shader,
            &mesh_pipeline_layout,
            &outline_pipeline_layout,
            &line_pipeline_layout,
            format,
            sample_count,
        );

        let white_view = create_white_texture(&device, &queue);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mesh sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
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

        Ok(Self {
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
            sky_pipeline: pipelines.sky,
            mesh_pipeline: pipelines.mesh,
            transparent_pipeline: pipelines.transparent,
            outline_pipeline: pipelines.outline,
            line_pipeline: pipelines.line,
            camera_buffer,
            camera_bind_group,
            meshes,
            line_buffer,
            line_count,
            lines_visible: true,
            camera: scene.camera,
            backend,
            surface_format,
        })
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
            self.config.format,
            actual,
        );
        self.sky_pipeline = pipelines.sky;
        self.mesh_pipeline = pipelines.mesh;
        self.transparent_pipeline = pipelines.transparent;
        self.outline_pipeline = pipelines.outline;
        self.line_pipeline = pipelines.line;
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

            pass.set_pipeline(&self.mesh_pipeline);
            for mesh in self.meshes.iter().filter(|mesh| mesh.alpha >= 0.999) {
                if !mesh.visible {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                pass.set_bind_group(2, &mesh.texture_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }

            pass.set_pipeline(&self.transparent_pipeline);
            for mesh in self.meshes.iter().filter(|mesh| mesh.alpha < 0.999) {
                if !mesh.visible {
                    continue;
                }
                pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                pass.set_bind_group(2, &mesh.texture_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
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

    pub fn set_mesh_model(&mut self, index: usize, model: Mat4) {
        if let Some(mesh) = self.meshes.get(index) {
            let uniform = ModelUniform {
                model: model.to_cols_array_2d(),
                alpha: [mesh.alpha, 0.0, 0.0, 0.0],
            };
            self.queue
                .write_buffer(&mesh.model_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }

    pub fn set_mesh_visible(&mut self, index: usize, visible: bool) {
        if let Some(mesh) = self.meshes.get_mut(index) {
            mesh.visible = visible;
        }
    }

    pub fn set_mesh_alpha(&mut self, index: usize, alpha: f32) {
        if let Some(mesh) = self.meshes.get_mut(index) {
            mesh.alpha = alpha;
            let offset = std::mem::offset_of!(ModelUniform, alpha) as u64;
            self.queue.write_buffer(
                &mesh.model_buffer,
                offset,
                bytemuck::bytes_of(&[alpha, 0.0, 0.0, 0.0]),
            );
        }
    }

    fn write_camera(&self) {
        let uniform = camera_uniform(&self.camera, self.config.width, self.config.height);
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
    let model_uniform = ModelUniform {
        model: mesh.model.to_cols_array_2d(),
        alpha: [mesh.alpha, 0.0, 0.0, 0.0],
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

    let owned_texture;
    let texture_view = if let Some(index) = mesh.texture {
        if let Some(texture) = texture_resources.textures.get(index) {
            owned_texture = create_scene_texture(device, queue, texture, &mesh.name);
            &owned_texture
        } else {
            texture_resources.white_view
        }
    } else {
        texture_resources.white_view
    };
    let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{} texture bind group", mesh.name)),
        layout: texture_resources.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(texture_resources.sampler),
            },
        ],
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
        outline,
        visible: true,
        alpha: mesh.alpha,
    }
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

fn create_scene_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &SceneTexture,
    label: &str,
) -> wgpu::TextureView {
    let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("{label} texture")),
        size: wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
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
            texture: &gpu_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texture.rgba8,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width * 4),
            rows_per_image: Some(texture.height),
        },
        wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
    );
    gpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn camera_uniform(camera: &Camera, width: u32, height: u32) -> CameraUniform {
    let aspect = width as f32 / height as f32;
    let view_proj = camera.view_proj(aspect).to_cols_array_2d();
    let eye = camera.eye;
    let light = glam::Vec3::new(-0.4, -1.0, -0.3).normalize();
    CameraUniform {
        view_proj,
        camera_pos: [eye[0], eye[1], eye[2], 1.0],
        light_dir: [light.x, light.y, light.z, 0.0],
    }
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

fn create_pipelines(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    mesh_layout: &wgpu::PipelineLayout,
    outline_layout: &wgpu::PipelineLayout,
    line_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> Pipelines {
    let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sky pipeline"),
        layout: None,
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
    use super::select_sample_count;

    #[test]
    fn sample_count_prefers_requested_with_fallback() {
        assert_eq!(select_sample_count(&[1, 2, 4], 4), 4);
        assert_eq!(select_sample_count(&[1, 2, 4], 2), 2);
        assert_eq!(select_sample_count(&[1, 2, 4], 1), 1);
        assert_eq!(select_sample_count(&[1, 4], 2), 1);
        assert_eq!(select_sample_count(&[1, 2], 4), 2);
        assert_eq!(select_sample_count(&[1], 4), 1);
    }
}
