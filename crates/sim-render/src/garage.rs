use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use sim_types::{
    Aabb, AssetManifest, CarPhysics, CarVisual, MeshBuffers, MeshRef, NodeTransform,
    TrackCollision, TrackMetadata, TrackVisual,
};
use std::collections::{HashMap, HashSet};

use crate::camera::Camera;
use crate::scene::{
    build_outline, build_vertices, push_line, scene_texture, CarRig, CockpitNeedle,
    CockpitNeedleSource, GarageBay, LineVertex, MeshData, MeshVertex, RigWheel, Scene,
    SceneTexture, SceneWheel,
};
use crate::shadow_decal_yaw;

impl Scene {
    #[allow(clippy::too_many_arguments)]
    pub fn build_garage(
        track_visual: &TrackVisual,
        track_metadata: &TrackMetadata,
        track_collision: &TrackCollision,
        track_manifest: &AssetManifest,
        car_physics: &CarPhysics,
        car_visual: &CarVisual,
        car_manifest: &AssetManifest,
        car_count: usize,
        color_offset: usize,
        static_car_shadows: bool,
        brake_lights: bool,
    ) -> Result<Scene, String> {
        let mut scene = Scene::build(
            track_visual,
            track_metadata,
            track_collision,
            track_manifest,
        )?;

        let track_stats = scene.stats;
        let car_transform = car_transform(track_metadata)?;
        let (_, car_rotation, car_translation) = car_transform.to_scale_rotation_translation();
        let car_forward = car_rotation * Vec3::NEG_Z;
        let car_right = car_rotation * Vec3::X;
        let car_start = scene.meshes.len();
        let car_texture_base = scene.textures.len();
        for texture in &car_visual.textures {
            scene.textures.push(scene_texture(texture)?);
        }
        let silhouette = if static_car_shadows {
            None
        } else {
            build_car_silhouette(car_visual)?
        };
        let decal_texture = silhouette.as_ref().map(|silhouette| {
            let index = scene.textures.len();
            scene.textures.push(silhouette.texture.clone());
            index
        });

        let bay_count = car_count.clamp(1, 10);
        let mut bays = Vec::with_capacity(bay_count);
        let mut first_bounds = None;
        for index in 0..bay_count {
            let color_index = (color_offset + index) % BAY_COLORS.len();
            let color = BAY_COLORS[color_index];
            let (translation, rotation) = if bay_count == 1 {
                (car_translation, car_rotation)
            } else {
                let row = index / 5;
                let slot = index % 5;
                let lateral = slot as f32 * BAY_SPACING;
                let row_translation = car_translation
                    + car_right * lateral
                    + if row == 1 {
                        car_forward * ROW_GAP
                    } else {
                        Vec3::ZERO
                    };
                let row_rotation = if row == 1 {
                    Quat::from_rotation_y(std::f32::consts::PI) * car_rotation
                } else {
                    car_rotation
                };
                (row_translation, row_rotation)
            };
            let transform = Mat4::from_rotation_translation(rotation, translation);
            let car_mesh_start = scene.meshes.len();
            let built = build_car(
                &mut scene,
                car_visual,
                car_texture_base,
                transform,
                color,
                &format!("bay_{index}"),
                brake_lights,
            )?;
            if static_car_shadows {
                for mesh in &mut scene.meshes[car_mesh_start..] {
                    mesh.casts_static_shadow = true;
                    mesh.casts_dynamic_shadow = false;
                }
            }
            let row_forward = rotation * Vec3::NEG_Z;
            let row_right = rotation * Vec3::X;
            let camera = Camera {
                eye: (translation + row_forward * 4.860569 - row_right * 3.148117
                    + Vec3::Y * 1.029985)
                    .to_array(),
                target: (translation + Vec3::Y * 0.6).to_array(),
                fov_y: 0.85,
            };
            let decal_mesh_index = match (&silhouette, decal_texture) {
                (Some(silhouette), Some(texture)) => Some(add_car_decal(
                    &mut scene,
                    texture,
                    silhouette.width,
                    silhouette.length,
                )?),
                _ => None,
            };
            if index == 0 {
                scene.camera = camera;
                let mut rig = built.rig.clone();
                rig.decal_mesh_index = decal_mesh_index;
                rig.decal_width = silhouette.as_ref().map(|s| s.width).unwrap_or(1.0);
                rig.decal_length = silhouette.as_ref().map(|s| s.length).unwrap_or(1.0);
                scene.car_rig = Some(rig);
                scene.wheels = built.wheels.clone();
                first_bounds = Some(built.bounds);
            }
            let decal_yaw = shadow_decal_yaw(row_forward);
            bays.push(GarageBay {
                wheels: built.wheels,
                camera,
                name: format!("Bay {}", index + 1),
                color,
                color_name: BAY_COLOR_NAMES[color_index],
                decal_mesh_index,
                decal_center: [translation.x, built.ground_y, translation.z],
                decal_yaw,
                decal_width: silhouette.as_ref().map(|s| s.width).unwrap_or(1.0),
                decal_length: silhouette.as_ref().map(|s| s.length).unwrap_or(1.0),
                decal_ground_y: built.ground_y,
            });
        }
        scene.garage_bays = bays;
        if static_car_shadows {
            scene.car_mesh_range = Some(car_start..scene.meshes.len());
        }
        let body_bounds = first_bounds.unwrap_or(Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        });

        let car_draw_calls = scene.meshes.len() - car_start;
        let car_vertices = scene.meshes[car_start..]
            .iter()
            .map(|mesh| mesh.vertices.len() as u32)
            .sum::<u32>();
        let car_triangles = scene.meshes[car_start..]
            .iter()
            .map(|mesh| mesh.indices.len() as u32 / 3)
            .sum::<u32>();

        scene.stats.vertices += car_vertices;
        scene.stats.triangles += car_triangles;
        scene.stats.draw_calls += car_draw_calls as u32;
        scene.stats.textures +=
            car_visual.textures.len() as u32 + u32::from(decal_texture.is_some());

        scene.debug_info = json!({
            "asset": { "id": track_manifest.id, "kind": "Track" },
            "car": {
                "id": car_manifest.id,
                "mass": car_physics.mass,
                "wheelbase": car_physics.wheelbase,
                "trackWidth": car_physics.track_width,
                "wheels": car_physics.wheels.iter().map(|wheel| json!({
                    "name": wheel.name,
                    "position": wheel.position,
                    "radius": wheel.radius,
                    "width": wheel.width,
                    "front": wheel.is_front,
                    "left": wheel.is_left,
                    "driven": wheel.driven,
                })).collect::<Vec<_>>(),
                "meshCount": car_visual.meshes.len(),
                "meshBufferCount": car_visual.mesh_buffers.len(),
                "steeringWheel": car_visual.steering_wheel.is_some(),
                "bounds": {
                    "min": body_bounds.min,
                    "max": body_bounds.max,
                },
            },
            "stats": {
                "vertices": scene.stats.vertices,
                "triangles": scene.stats.triangles,
                "drawCalls": scene.stats.draw_calls,
                "textures": scene.stats.textures,
                "texturedMeshes": scene.meshes.iter().filter(|mesh| mesh.texture.is_some()).count(),
                "outlinedMeshes": scene.meshes.iter().filter(|mesh| mesh.outline.is_some()).count(),
                "collisionTriangles": scene.stats.collision_triangles,
                "trackTriangles": track_stats.triangles,
                "carTriangles": car_triangles,
                "trackDrawCalls": track_stats.draw_calls,
                "carDrawCalls": car_draw_calls,
            },
            "bounds": { "min": track_manifest.bounds.min, "max": track_manifest.bounds.max },
            "surfaces": track_metadata.surfaces.iter().map(|surface| json!({
                "id": surface.id,
                "name": surface.name,
                "friction": surface.friction,
                "roughness": surface.roughness,
            })).collect::<Vec<_>>(),
            "spawns": track_metadata.spawns.iter().map(|spawn| json!({
                "name": spawn.name,
                "position": spawn.position,
                "rotation": spawn.rotation_xyzw,
            })).collect::<Vec<_>>(),
            "cameras": track_metadata.cameras.iter().map(|camera| json!({
                "name": camera.name,
                "position": camera.position,
                "target": camera.target,
                "fovY": camera.fov_y,
            })).collect::<Vec<_>>(),
            "meshes": scene.meshes.iter().map(|mesh| json!({
                "name": mesh.name,
                "vertices": mesh.vertices.len(),
                "triangles": mesh.indices.len() / 3,
            })).collect::<Vec<_>>(),
            "warnings": track_manifest.warnings.iter().chain(car_manifest.warnings.iter()).cloned().collect::<Vec<_>>(),
        })
        .to_string();

        Ok(scene)
    }
}

struct CarSilhouette {
    texture: SceneTexture,
    width: f32,
    length: f32,
}

fn build_car_silhouette(car_visual: &CarVisual) -> Result<Option<CarSilhouette>, String> {
    let Some(body_index) = mesh_index_by_name(car_visual, "body") else {
        return Ok(None);
    };
    let Some(buffers) = car_visual.mesh_buffers.get(body_index) else {
        return Ok(None);
    };
    if buffers.positions.is_empty() || buffers.indices.is_empty() {
        return Ok(None);
    }
    let body_local = node_matrix(&car_visual.body);
    let positions = buffers
        .positions
        .iter()
        .map(|position| body_local.transform_point3(Vec3::from_array(*position)))
        .collect::<Vec<_>>();
    let mut min_x = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for point in &positions {
        min_x = min_x.min(point.x);
        min_z = min_z.min(point.z);
        max_x = max_x.max(point.x);
        max_z = max_z.max(point.z);
    }
    if !min_x.is_finite() || !min_z.is_finite() {
        return Ok(None);
    }
    let margin = 0.08;
    min_x -= margin;
    min_z -= margin;
    max_x += margin;
    max_z += margin;
    let width = (max_x - min_x).max(0.1);
    let length = (max_z - min_z).max(0.1);
    let texture_width = 256usize;
    let texture_height = 512usize;
    let mut alpha = vec![0u8; texture_width * texture_height];

    for triangle in buffers.indices.as_chunks::<3>().0 {
        let a = positions.get(triangle[0] as usize);
        let b = positions.get(triangle[1] as usize);
        let c = positions.get(triangle[2] as usize);
        let (Some(a), Some(b), Some(c)) = (a, b, c) else {
            continue;
        };
        let ax = (a.x - min_x) / width * (texture_width - 1) as f32;
        let ay = (a.z - min_z) / length * (texture_height - 1) as f32;
        let bx = (b.x - min_x) / width * (texture_width - 1) as f32;
        let by = (b.z - min_z) / length * (texture_height - 1) as f32;
        let cx = (c.x - min_x) / width * (texture_width - 1) as f32;
        let cy = (c.z - min_z) / length * (texture_height - 1) as f32;
        let area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
        if area.abs() < 1e-6 {
            continue;
        }
        let left = ax.min(bx).min(cx).floor().max(0.0) as usize;
        let right = ax.max(bx).max(cx).ceil().min((texture_width - 1) as f32) as usize;
        let top = ay.min(by).min(cy).floor().max(0.0) as usize;
        let bottom = ay.max(by).max(cy).ceil().min((texture_height - 1) as f32) as usize;
        for py in top..=bottom {
            for px in left..=right {
                let x = px as f32 + 0.5;
                let y = py as f32 + 0.5;
                let w0 = ((bx - x) * (cy - y) - (by - y) * (cx - x)) / area;
                let w1 = ((cx - x) * (ay - y) - (cy - y) * (ax - x)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    alpha[py * texture_width + px] = 255;
                }
            }
        }
    }

    blur_alpha(&mut alpha, texture_width, texture_height);

    let mut rgba8 = Vec::with_capacity(texture_width * texture_height * 4);
    for value in alpha {
        rgba8.extend_from_slice(&[255, 255, 255, value]);
    }
    Ok(Some(CarSilhouette {
        texture: SceneTexture {
            width: texture_width as u32,
            height: texture_height as u32,
            rgba8,
        },
        width,
        length,
    }))
}

fn blur_alpha(alpha: &mut [u8], width: usize, height: usize) {
    const RADIUS: i32 = 6;
    const SIGMA: f32 = 2.8;
    let mut kernel = Vec::with_capacity((RADIUS * 2 + 1) as usize);
    let mut total = 0.0f32;
    for offset in -RADIUS..=RADIUS {
        let weight = (-(offset * offset) as f32 / (2.0 * SIGMA * SIGMA)).exp();
        kernel.push(weight);
        total += weight;
    }
    for weight in &mut kernel {
        *weight /= total;
    }

    let mut horizontal = vec![0u8; alpha.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0f32;
            for (index, weight) in kernel.iter().enumerate() {
                let sx = (x as i32 + index as i32 - RADIUS).clamp(0, width as i32 - 1) as usize;
                sum += alpha[y * width + sx] as f32 * weight;
            }
            horizontal[y * width + x] = sum.round() as u8;
        }
    }

    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0f32;
            for (index, weight) in kernel.iter().enumerate() {
                let sy = (y as i32 + index as i32 - RADIUS).clamp(0, height as i32 - 1) as usize;
                sum += horizontal[sy * width + x] as f32 * weight;
            }
            alpha[y * width + x] = sum.round() as u8;
        }
    }
}

fn add_car_decal(
    scene: &mut Scene,
    texture: usize,
    width: f32,
    length: f32,
) -> Result<usize, String> {
    if width <= 0.0 || length <= 0.0 {
        return Err("car decal has no footprint".to_string());
    }
    let positions = [
        [-0.5, 0.0, -0.5],
        [0.5, 0.0, -0.5],
        [0.5, 0.0, 0.5],
        [-0.5, 0.0, 0.5],
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut vertices = Vec::with_capacity(4);
    for (position, uv) in positions.iter().zip(uvs.iter()) {
        vertices.push(MeshVertex {
            position: *position,
            normal: [0.0, 1.0, 0.0],
            color: [1.0, 1.0, 1.0],
            uv: *uv,
        });
    }
    let _ = (width, length);
    scene.meshes.push(MeshData {
        name: "car_shadow_decal".to_string(),
        vertices,
        indices: vec![0, 1, 2, 0, 2, 3],
        model: Mat4::IDENTITY,
        texture: Some(texture),
        outline: None,
        alpha: 0.20,
        emissive: [0.0; 3],
        casts_static_shadow: false,
        casts_dynamic_shadow: false,
        is_decal: true,
    });
    Ok(scene.meshes.len() - 1)
}

fn recolor_paint(buffers: &mut MeshBuffers, color: [f32; 3]) {
    if buffers.colors.is_empty() {
        return;
    }
    for vertex_color in &mut buffers.colors {
        if vertex_color[0] > 0.2
            && vertex_color[0] > vertex_color[1] * 1.8
            && vertex_color[0] > vertex_color[2] * 1.8
        {
            *vertex_color = color;
        }
    }
}

struct CarBuild {
    rig: CarRig,
    wheels: Vec<SceneWheel>,
    bounds: Aabb,
    ground_y: f32,
}

const BAY_SPACING: f32 = 8.0;
const ROW_GAP: f32 = 68.5;

pub(crate) const BAY_COLOR_NAMES: [&str; 10] = [
    "Red", "Blue", "Yellow", "Green", "Orange", "Black", "White", "Pink", "Navy", "Cyan",
];

const BAY_COLORS: [[f32; 3]; 10] = [
    [0.78, 0.08, 0.06],
    [0.08, 0.22, 0.72],
    [0.86, 0.62, 0.07],
    [0.08, 0.42, 0.18],
    [0.88, 0.34, 0.04],
    [0.025, 0.025, 0.03],
    [0.86, 0.86, 0.83],
    [0.88, 0.18, 0.48],
    [0.02, 0.07, 0.22],
    [0.02, 0.56, 0.55],
];

const COCKPIT_DIALS: [(CockpitNeedleSource, [f32; 3], f32, f32); 3] = [
    (
        CockpitNeedleSource::Speed,
        [-0.462, 0.935, -0.22],
        0.044,
        0.035,
    ),
    (
        CockpitNeedleSource::Rpm,
        [-0.35, 0.945, -0.22],
        0.060,
        0.045,
    ),
    (
        CockpitNeedleSource::Gear,
        [-0.238, 0.935, -0.22],
        0.044,
        0.035,
    ),
];

fn build_car(
    scene: &mut Scene,
    car_visual: &CarVisual,
    car_texture_base: usize,
    transform: Mat4,
    body_color: [f32; 3],
    prefix: &str,
    brake_lights: bool,
) -> Result<CarBuild, String> {
    let body_index = mesh_index_by_name(car_visual, "body").unwrap_or(0);
    let body_mesh = car_visual
        .meshes
        .get(body_index)
        .ok_or_else(|| "car has no body mesh".to_string())?;
    let mut body_buffers = car_visual
        .mesh_buffers
        .get(body_index)
        .ok_or_else(|| "car body mesh has no buffers".to_string())?
        .clone();
    let (brake_light_buffers, reverse_light_buffers) = if brake_lights {
        split_rear_lights(&mut body_buffers, &body_mesh.bounds)
    } else {
        (None, None)
    };
    recolor_paint(&mut body_buffers, body_color);
    let body_local = node_matrix(&car_visual.body);
    let body_model = transform * body_local;

    let body_mesh_index = push_car_mesh(
        scene,
        format!("{}_body", prefix).as_str(),
        body_mesh,
        &body_buffers,
        body_color,
        body_model,
        CarMeshStyle {
            texture: car_texture(car_visual, body_mesh, car_texture_base)?,
            outline: car_outline(car_visual, body_mesh),
            alpha: car_material_alpha(car_visual, body_mesh.material as usize),
        },
    )?;
    let body_bounds = world_bounds(&body_buffers, body_model);
    add_box_lines(&mut scene.lines, &body_bounds, [0.25, 0.75, 1.0]);

    let brake_light_mesh_index = if let Some(buffers) = brake_light_buffers {
        Some(push_car_light_mesh(
            scene,
            prefix,
            "brake_lights",
            body_mesh,
            &buffers,
            body_model,
        )?)
    } else {
        None
    };
    let reverse_light_mesh_index = if let Some(buffers) = reverse_light_buffers {
        Some(push_car_light_mesh(
            scene,
            prefix,
            "reverse_lights",
            body_mesh,
            &buffers,
            body_model,
        )?)
    } else {
        None
    };

    let glass_mesh_index = if let Some(index) = mesh_index_by_name(car_visual, "body_glass") {
        if let (Some(mesh), Some(buffers)) = (
            car_visual.meshes.get(index),
            car_visual.mesh_buffers.get(index),
        ) {
            let color = car_material_color(car_visual, mesh.material as usize);
            Some(push_car_mesh(
                scene,
                format!("{}_glass", prefix).as_str(),
                mesh,
                buffers,
                color,
                body_model,
                CarMeshStyle {
                    texture: car_texture(car_visual, mesh, car_texture_base)?,
                    outline: false,
                    alpha: 1.0,
                },
            )?)
        } else {
            None
        }
    } else {
        None
    };

    let dashboard_mesh_index = if brake_lights {
        let dashboard = mesh_index_by_name(car_visual, "body_glass")
            .and_then(|index| car_visual.mesh_buffers.get(index))
            .and_then(cockpit_cowl_buffers);
        if let Some(buffers) = dashboard {
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for position in &buffers.positions {
                for axis in 0..3 {
                    min[axis] = min[axis].min(position[axis]);
                    max[axis] = max[axis].max(position[axis]);
                }
            }
            let mesh_ref = MeshRef {
                name: "dashboard".to_string(),
                vertex_offset: 0,
                vertex_count: buffers.positions.len() as u32,
                index_offset: 0,
                index_count: buffers.indices.len() as u32,
                material: 0,
                bounds: Aabb { min, max },
                lod: 0,
            };
            let index = push_car_mesh(
                scene,
                format!("{}_dashboard", prefix).as_str(),
                &mesh_ref,
                &buffers,
                [0.04, 0.045, 0.05],
                body_model,
                CarMeshStyle {
                    texture: None,
                    outline: false,
                    alpha: 1.0,
                },
            )?;
            scene.meshes[index].casts_static_shadow = false;
            scene.meshes[index].casts_dynamic_shadow = false;
            Some(index)
        } else {
            None
        }
    } else {
        None
    };

    let mut cockpit_needles = Vec::new();
    if brake_lights && dashboard_mesh_index.is_some() {
        for (source, center, radius, depth) in COCKPIT_DIALS {
            let buffers = needle_buffers(radius);
            let center = Vec3::new(center[0], center[1], center[2] + depth * 0.5 + 0.003);
            let mesh_ref = MeshRef {
                name: format!("dashboard_needle_{}", source as u8),
                vertex_offset: 0,
                vertex_count: buffers.positions.len() as u32,
                index_offset: 0,
                index_count: buffers.indices.len() as u32,
                material: 0,
                bounds: Aabb {
                    min: [-radius * 0.025, 0.0, 0.0],
                    max: [radius * 0.025, radius * 0.55, 0.0],
                },
                lod: 0,
            };
            let mesh_index = push_car_mesh(
                scene,
                &format!("{}_needle_{}", prefix, source as u8),
                &mesh_ref,
                &buffers,
                [0.012, 0.014, 0.018],
                body_model * Mat4::from_translation(center),
                CarMeshStyle {
                    texture: None,
                    outline: false,
                    alpha: 1.0,
                },
            )?;
            scene.meshes[mesh_index].casts_static_shadow = false;
            scene.meshes[mesh_index].casts_dynamic_shadow = false;
            cockpit_needles.push(CockpitNeedle {
                mesh_index,
                center,
                source,
            });
        }
    }

    let mut wheels = Vec::with_capacity(car_visual.wheels.len());
    let mut rig_wheels = Vec::with_capacity(car_visual.wheels.len());
    for wheel in &car_visual.wheels {
        let mesh = car_visual
            .meshes
            .get(wheel.mesh as usize)
            .ok_or_else(|| format!("wheel {} references a missing mesh", wheel.name))?;
        let buffers = car_visual
            .mesh_buffers
            .get(wheel.mesh as usize)
            .ok_or_else(|| format!("wheel {} has no mesh buffers", wheel.name))?;
        let local_model = node_matrix(&wheel.transform);
        let model = transform * local_model;
        let color = car_material_color(car_visual, mesh.material as usize);
        let mesh_index = push_car_mesh(
            scene,
            &format!("{}_wheel_{}", prefix, wheel.name),
            mesh,
            buffers,
            color,
            model,
            CarMeshStyle {
                texture: car_texture(car_visual, mesh, car_texture_base)?,
                outline: car_outline(car_visual, mesh),
                alpha: car_material_alpha(car_visual, mesh.material as usize),
            },
        )?;
        add_wheel_marker(
            &mut scene.lines,
            model,
            if wheel.is_front {
                [0.2, 0.8, 1.0]
            } else {
                [1.0, 0.55, 0.2]
            },
        );
        wheels.push(SceneWheel {
            mesh_index,
            base_model: model,
            is_front: wheel.is_front,
        });
        rig_wheels.push(RigWheel {
            mesh_index,
            local_model,
            is_front: wheel.is_front,
            radius: wheel.radius,
            width: wheel.width,
        });
    }

    let mut steering_local = Mat4::IDENTITY;
    let mut steering_mesh_index = None;
    if let (Some(steering), Some(index)) = (
        car_visual.steering_wheel,
        mesh_index_by_name(car_visual, format!("{}_steering", prefix).as_str()),
    ) {
        if let (Some(mesh), Some(buffers)) = (
            car_visual.meshes.get(index),
            car_visual.mesh_buffers.get(index),
        ) {
            let local = node_matrix(&steering);
            let model = transform * local;
            let color = car_material_color(car_visual, mesh.material as usize);
            steering_mesh_index = Some(push_car_mesh(
                scene,
                format!("{}_steering", prefix).as_str(),
                mesh,
                buffers,
                color,
                model,
                CarMeshStyle {
                    texture: car_texture(car_visual, mesh, car_texture_base)?,
                    outline: car_outline(car_visual, mesh),
                    alpha: car_material_alpha(car_visual, mesh.material as usize),
                },
            )?);
            add_wheel_marker(&mut scene.lines, model, [0.2, 1.0, 0.7]);
            steering_local = local;
        }
    }
    let rig = CarRig {
        body_mesh_index,
        body_local,
        glass_mesh_index,
        brake_light_mesh_index,
        reverse_light_mesh_index,
        dashboard_mesh_index,
        dashboard_local: body_local,
        cockpit_needles,
        wheels: rig_wheels,
        steering_mesh_index,
        steering_local,
        decal_mesh_index: None,
        decal_width: 1.0,
        decal_length: 1.0,
    };
    let mut ground_y = f32::INFINITY;
    for wheel in &car_visual.wheels {
        let center = transform.transform_point3(Vec3::from_array(wheel.transform.translation));
        ground_y = ground_y.min(center.y - wheel.radius);
    }
    if !ground_y.is_finite() {
        ground_y = body_bounds.min[1];
    }
    Ok(CarBuild {
        rig,
        wheels,
        bounds: body_bounds,
        ground_y,
    })
}

fn cockpit_cowl_buffers(glass: &MeshBuffers) -> Option<MeshBuffers> {
    let mut edge_counts: HashMap<(u32, u32), u32> = HashMap::new();
    for triangle in glass.indices.as_chunks::<3>().0 {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_counts.entry(key).or_default() += 1;
        }
    }

    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    for ((a, b), count) in edge_counts {
        if count != 1 {
            continue;
        }
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }

    let mut unvisited: HashSet<u32> = adjacency.keys().copied().collect();
    let mut front_loop: Option<Vec<u32>> = None;
    let mut front_z = f32::INFINITY;
    while let Some(start) = unvisited.iter().next().copied() {
        let mut component = Vec::new();
        let mut stack = vec![start];
        unvisited.remove(&start);
        while let Some(vertex) = stack.pop() {
            component.push(vertex);
            if let Some(neighbors) = adjacency.get(&vertex) {
                for neighbor in neighbors {
                    if unvisited.remove(neighbor) {
                        stack.push(*neighbor);
                    }
                }
            }
        }
        if component.len() < 3 {
            continue;
        }
        let min_z = component
            .iter()
            .filter_map(|index| glass.positions.get(*index as usize))
            .map(|position| position[2])
            .fold(f32::INFINITY, f32::min);
        if min_z < front_z {
            front_z = min_z;
            front_loop = Some(component);
        }
    }

    let component = front_loop?;
    let y_of = |index: u32| {
        glass
            .positions
            .get(index as usize)
            .map(|position| position[1])
            .unwrap_or(f32::INFINITY)
    };
    let start = *component.iter().min_by(|a, b| {
        y_of(**a)
            .partial_cmp(&y_of(**b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    let mut order = vec![start];
    let mut previous = None;
    let mut current = start;
    loop {
        let next = adjacency
            .get(&current)?
            .iter()
            .copied()
            .find(|vertex| Some(*vertex) != previous)?;
        if next == start {
            break;
        }
        order.push(next);
        previous = Some(current);
        current = next;
        if order.len() > component.len() {
            return None;
        }
    }

    let min_y = order
        .iter()
        .map(|index| y_of(*index))
        .fold(f32::INFINITY, f32::min);
    let threshold = min_y + 0.05;
    let mut best_start = 0;
    let mut best_len = 0;
    for candidate in 0..order.len() {
        if y_of(order[candidate]) > threshold {
            continue;
        }
        let mut length = 0;
        while length < order.len() && y_of(order[(candidate + length) % order.len()]) <= threshold {
            length += 1;
        }
        if length > best_len {
            best_len = length;
            best_start = candidate;
        }
    }
    if best_len < 3 {
        return None;
    }
    let arc: Vec<u32> = (0..best_len)
        .map(|offset| order[(best_start + offset) % order.len()])
        .collect();

    let max_y = arc
        .iter()
        .map(|index| y_of(*index))
        .fold(f32::NEG_INFINITY, f32::max);
    let top_color = [0.04, 0.045, 0.05];
    let bulkhead_color = [0.022, 0.026, 0.03];
    let mut positions = Vec::with_capacity(arc.len() * 3);
    let mut normals = Vec::with_capacity(arc.len() * 3);
    let mut uvs = Vec::with_capacity(arc.len() * 3);
    let mut colors = Vec::with_capacity(arc.len() * 3);
    for index in &arc {
        let position = Vec3::from_array(*glass.positions.get(*index as usize)?);
        positions.push(position.to_array());
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.0, 0.0]);
        colors.push(top_color);

        positions.push([position.x * 0.92, max_y, position.z + 0.38]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([1.0, 0.0]);
        colors.push(top_color);
    }
    for index in &arc {
        let position = Vec3::from_array(*glass.positions.get(*index as usize)?);
        positions.push([position.x * 0.92, max_y - 0.28, position.z + 0.38]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([1.0, 1.0]);
        colors.push(bulkhead_color);
    }

    let mut indices = Vec::with_capacity((arc.len() - 1) * 12);
    for index in 0..arc.len() - 1 {
        let front = (index * 2) as u32;
        let next_front = ((index + 1) * 2) as u32;
        let inner = front + 1;
        let next_inner = next_front + 1;
        indices.extend_from_slice(&[front, next_front, next_inner, front, next_inner, inner]);
        let bottom = (arc.len() * 2 + index) as u32;
        let next_bottom = (arc.len() * 2 + index + 1) as u32;
        indices.extend_from_slice(&[inner, next_inner, next_bottom, inner, next_bottom, bottom]);
    }

    let mut buffers = MeshBuffers {
        positions,
        normals,
        uvs,
        colors,
        indices,
    };
    for (_, center, radius, depth) in COCKPIT_DIALS {
        append_dial(&mut buffers, center, radius, depth);
    }
    Some(buffers)
}

fn append_dial(buffers: &mut MeshBuffers, center: [f32; 3], radius: f32, depth: f32) {
    let segments = 24;
    let z_front = center[2] + depth * 0.5;
    let z_back = center[2] - depth * 0.5;
    let outer = [0.018, 0.022, 0.028];
    let face = [0.035, 0.04, 0.046];
    let white = [0.82, 0.84, 0.79];
    let tau = std::f32::consts::TAU;

    for index in 0..segments {
        let a = tau * index as f32 / segments as f32;
        let b = tau * (index + 1) as f32 / segments as f32;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let quad = [
            (
                [center[0] + ca * radius, center[1] + sa * radius, z_front],
                [ca, sa, 0.0],
            ),
            (
                [center[0] + cb * radius, center[1] + sb * radius, z_front],
                [cb, sb, 0.0],
            ),
            (
                [center[0] + cb * radius, center[1] + sb * radius, z_back],
                [cb, sb, 0.0],
            ),
            (
                [center[0] + ca * radius, center[1] + sa * radius, z_back],
                [ca, sa, 0.0],
            ),
        ];
        let base = buffers.positions.len() as u32;
        for (position, normal) in quad {
            buffers.positions.push(position);
            buffers.normals.push(normal);
            buffers.uvs.push([0.0, 0.0]);
            buffers.colors.push(outer);
        }
        buffers
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let inner_radius = radius * 0.84;
    let white_z = z_front - 0.004;
    for index in 0..segments {
        let a = tau * index as f32 / segments as f32;
        let b = tau * (index + 1) as f32 / segments as f32;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let quad = [
            (
                [center[0] + ca * radius, center[1] + sa * radius, z_front],
                [0.0, 0.0, 1.0],
            ),
            (
                [center[0] + cb * radius, center[1] + sb * radius, z_front],
                [0.0, 0.0, 1.0],
            ),
            (
                [
                    center[0] + cb * inner_radius,
                    center[1] + sb * inner_radius,
                    z_front,
                ],
                [0.0, 0.0, 1.0],
            ),
            (
                [
                    center[0] + ca * inner_radius,
                    center[1] + sa * inner_radius,
                    z_front,
                ],
                [0.0, 0.0, 1.0],
            ),
        ];
        let base = buffers.positions.len() as u32;
        for (position, normal) in quad {
            buffers.positions.push(position);
            buffers.normals.push(normal);
            buffers.uvs.push([0.0, 0.0]);
            buffers.colors.push(face);
        }
        buffers
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    for index in 0..segments {
        let a = tau * index as f32 / segments as f32;
        let b = tau * (index + 1) as f32 / segments as f32;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let quad = [
            (
                [
                    center[0] + ca * inner_radius,
                    center[1] + sa * inner_radius,
                    z_front,
                ],
                [-ca, -sa, 0.0],
            ),
            (
                [
                    center[0] + cb * inner_radius,
                    center[1] + sb * inner_radius,
                    z_front,
                ],
                [-cb, -sb, 0.0],
            ),
            (
                [
                    center[0] + cb * inner_radius,
                    center[1] + sb * inner_radius,
                    white_z,
                ],
                [-cb, -sb, 0.0],
            ),
            (
                [
                    center[0] + ca * inner_radius,
                    center[1] + sa * inner_radius,
                    white_z,
                ],
                [-ca, -sa, 0.0],
            ),
        ];
        let base = buffers.positions.len() as u32;
        for (position, normal) in quad {
            buffers.positions.push(position);
            buffers.normals.push(normal);
            buffers.uvs.push([0.0, 0.0]);
            buffers.colors.push(face);
        }
        buffers
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let white_center = buffers.positions.len() as u32;
    buffers.positions.push([center[0], center[1], white_z]);
    buffers.normals.push([0.0, 0.0, 1.0]);
    buffers.uvs.push([0.5, 0.5]);
    buffers.colors.push(white);
    let white_base = buffers.positions.len() as u32;
    for index in 0..segments {
        let angle = tau * index as f32 / segments as f32;
        let (sin, cos) = angle.sin_cos();
        buffers.positions.push([
            center[0] + cos * inner_radius,
            center[1] + sin * inner_radius,
            white_z,
        ]);
        buffers.normals.push([0.0, 0.0, 1.0]);
        buffers.uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
        buffers.colors.push(white);
    }
    for index in 0..segments {
        let next = (index + 1) % segments;
        buffers.indices.extend_from_slice(&[
            white_center,
            white_base + index as u32,
            white_base + next as u32,
        ]);
    }
}

fn needle_buffers(radius: f32) -> MeshBuffers {
    let width = radius * 0.02;
    let length = radius * 0.55;
    MeshBuffers {
        positions: vec![[-width, 0.0, 0.0], [width, 0.0, 0.0], [0.0, length, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
        colors: vec![[0.012, 0.014, 0.018]; 3],
        indices: vec![0, 1, 2],
    }
}

struct CarMeshStyle {
    texture: Option<usize>,
    outline: bool,
    alpha: f32,
}

fn push_car_mesh(
    scene: &mut Scene,
    name: &str,
    mesh: &MeshRef,
    buffers: &MeshBuffers,
    color: [f32; 3],
    model: Mat4,
    style: CarMeshStyle,
) -> Result<usize, String> {
    if buffers.positions.is_empty() || buffers.indices.is_empty() {
        return Err(format!("car mesh {} is empty", mesh.name));
    }
    let vertices = build_vertices(buffers, color);
    for index in &buffers.indices {
        if *index as usize >= vertices.len() {
            return Err(format!(
                "car mesh {} index {} is outside {} vertices",
                mesh.name,
                index,
                vertices.len()
            ));
        }
    }
    scene.meshes.push(MeshData {
        name: name.to_string(),
        vertices,
        indices: buffers.indices.clone(),
        model,
        texture: style.texture,
        outline: if style.outline {
            build_outline(buffers)
        } else {
            None
        },
        alpha: style.alpha,
        emissive: [0.0; 3],
        casts_static_shadow: false,
        casts_dynamic_shadow: true,
        is_decal: false,
    });
    Ok(scene.meshes.len() - 1)
}

fn push_car_light_mesh(
    scene: &mut Scene,
    prefix: &str,
    name: &str,
    body_mesh: &MeshRef,
    buffers: &MeshBuffers,
    model: Mat4,
) -> Result<usize, String> {
    let light_ref = MeshRef {
        name: format!("body_{name}"),
        vertex_offset: 0,
        vertex_count: buffers.positions.len() as u32,
        index_offset: 0,
        index_count: buffers.indices.len() as u32,
        material: body_mesh.material,
        bounds: body_mesh.bounds,
        lod: 0,
    };
    let index = push_car_mesh(
        scene,
        &format!("{prefix}_{name}"),
        &light_ref,
        buffers,
        [1.0, 1.0, 1.0],
        model,
        CarMeshStyle {
            texture: None,
            outline: false,
            alpha: 1.0,
        },
    )?;
    scene.meshes[index].casts_static_shadow = false;
    scene.meshes[index].casts_dynamic_shadow = false;
    Ok(index)
}

fn split_rear_lights(
    buffers: &mut MeshBuffers,
    bounds: &Aabb,
) -> (Option<MeshBuffers>, Option<MeshBuffers>) {
    let rear_start = bounds.max[2] - 0.35;
    let mut brake = new_light_buffers();
    let mut reverse = new_light_buffers();
    let mut body_indices = Vec::with_capacity(buffers.indices.len());

    for triangle in buffers.indices.as_chunks::<3>().0 {
        let rear = triangle.iter().all(|index| {
            buffers
                .positions
                .get(*index as usize)
                .is_some_and(|position| position[2] >= rear_start)
        });
        if !rear {
            body_indices.extend_from_slice(triangle);
            continue;
        }
        let red_orange = triangle.iter().any(|index| {
            buffers.colors.get(*index as usize).is_some_and(|color| {
                color[0] > 0.12
                    && color[0] > color[1] * 1.5
                    && color[1] > color[2] * 1.1
                    && color[1] < 0.35
            })
        });
        let reverse_lens = triangle.iter().all(|index| {
            buffers
                .positions
                .get(*index as usize)
                .is_some_and(|position| position[0].abs() > 0.30)
        }) && triangle.iter().any(|index| {
            buffers.colors.get(*index as usize).is_some_and(|color| {
                let max = color[0].max(color[1]).max(color[2]);
                let min = color[0].min(color[1]).min(color[2]);
                max - min < 0.04 && color[0] > 0.15
            })
        });
        if red_orange {
            push_light_triangle(&mut brake, buffers, triangle);
        } else if reverse_lens {
            push_light_triangle(&mut reverse, buffers, triangle);
        } else {
            body_indices.extend_from_slice(triangle);
        }
    }

    buffers.indices = body_indices;
    (
        if brake.indices.is_empty() {
            None
        } else {
            Some(brake)
        },
        if reverse.indices.is_empty() {
            None
        } else {
            Some(reverse)
        },
    )
}

fn new_light_buffers() -> MeshBuffers {
    MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    }
}

fn push_light_triangle(target: &mut MeshBuffers, source: &MeshBuffers, triangle: &[u32; 3]) {
    let base = target.positions.len() as u32;
    for index in triangle {
        let index = *index as usize;
        let Some(position) = source.positions.get(index).copied() else {
            continue;
        };
        target.positions.push(position);
        target.normals.push(
            source
                .normals
                .get(index)
                .copied()
                .unwrap_or([0.0, 0.0, 1.0]),
        );
        target
            .uvs
            .push(source.uvs.get(index).copied().unwrap_or([0.0, 0.0]));
        target
            .colors
            .push(source.colors.get(index).copied().unwrap_or([1.0; 3]));
    }
    target
        .indices
        .extend_from_slice(&[base, base + 1, base + 2]);
}

fn car_texture(
    car_visual: &CarVisual,
    mesh: &MeshRef,
    texture_base: usize,
) -> Result<Option<usize>, String> {
    let Some(material) = car_visual.materials.get(mesh.material as usize) else {
        return Ok(None);
    };
    let Some(index) = material.base_color_texture else {
        return Ok(None);
    };
    if index as usize >= car_visual.textures.len() {
        return Err(format!(
            "car mesh {} references missing texture {}",
            mesh.name, index
        ));
    }
    Ok(Some(texture_base + index as usize))
}

fn car_outline(car_visual: &CarVisual, mesh: &MeshRef) -> bool {
    car_visual
        .materials
        .get(mesh.material as usize)
        .map(|material| material.outline)
        .unwrap_or(false)
}

fn car_material_alpha(car_visual: &CarVisual, index: usize) -> f32 {
    car_visual
        .materials
        .get(index)
        .map(|material| material.base_color[3])
        .unwrap_or(1.0)
}

fn car_material_color(car_visual: &CarVisual, index: usize) -> [f32; 3] {
    car_visual
        .materials
        .get(index)
        .map(|material| {
            [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ]
        })
        .unwrap_or([1.0, 1.0, 1.0])
}

fn mesh_index_by_name(car_visual: &CarVisual, name: &str) -> Option<usize> {
    car_visual.meshes.iter().position(|mesh| mesh.name == name)
}

fn car_transform(metadata: &TrackMetadata) -> Result<Mat4, String> {
    let spawn = metadata
        .spawns
        .first()
        .ok_or_else(|| "track has no spawn point".to_string())?;
    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    );
    let rotation = if rotation.length_squared() > 1e-8 {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    };
    let rotation = Quat::from_rotation_y(std::f32::consts::PI) * rotation;
    Ok(Mat4::from_rotation_translation(
        rotation,
        Vec3::from_array(spawn.position),
    ))
}

fn node_matrix(node: &NodeTransform) -> Mat4 {
    let rotation = Quat::from_xyzw(
        node.rotation_xyzw[0],
        node.rotation_xyzw[1],
        node.rotation_xyzw[2],
        node.rotation_xyzw[3],
    );
    let rotation = if rotation.length_squared() > 1e-8 {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    };
    let mut scale = Vec3::from_array(node.scale);
    if scale.x.abs() < 1e-8 || scale.y.abs() < 1e-8 || scale.z.abs() < 1e-8 {
        scale = Vec3::ONE;
    }
    Mat4::from_scale_rotation_translation(scale, rotation, Vec3::from_array(node.translation))
}

fn world_bounds(buffers: &MeshBuffers, model: Mat4) -> Aabb {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &buffers.positions {
        let point = model.transform_point3(Vec3::from_array(*position));
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Aabb { min, max }
}

fn add_box_lines(lines: &mut Vec<LineVertex>, bounds: &Aabb, color: [f32; 3]) {
    let min = bounds.min;
    let max = bounds.max;
    let corners = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], min[1], max[2]],
        [min[0], min[1], max[2]],
        [min[0], max[1], min[2]],
        [max[0], max[1], min[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    for (from, to) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        push_line(lines, corners[from], corners[to], color);
    }
}

fn add_wheel_marker(lines: &mut Vec<LineVertex>, model: Mat4, color: [f32; 3]) {
    let center = model.transform_point3(Vec3::ZERO);
    let size = 0.35;
    push_line(
        lines,
        [center.x - size, center.y, center.z],
        [center.x + size, center.y, center.z],
        color,
    );
    push_line(
        lines,
        [center.x, center.y - size, center.z],
        [center.x, center.y + size, center.z],
        color,
    );
    push_line(
        lines,
        [center.x, center.y, center.z - size],
        [center.x, center.y, center.z + size],
        color,
    );
}
