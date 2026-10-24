use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use sim_types::{
    Aabb, AssetManifest, CarPhysics, CarVisual, MeshBuffers, MeshRef, NodeTransform,
    TrackCollision, TrackMetadata, TrackVisual,
};

use crate::camera::Camera;
use crate::scene::{
    build_outline, build_vertices, push_line, scene_texture, CarRig, GarageBay, LineVertex,
    MeshData, RigWheel, Scene, SceneWheel,
};

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
            let built = build_car(
                &mut scene,
                car_visual,
                car_texture_base,
                transform,
                color,
                &format!("bay_{index}"),
            )?;
            let row_forward = rotation * Vec3::NEG_Z;
            let row_right = rotation * Vec3::X;
            let camera = Camera {
                eye: (translation + row_forward * 4.860569 - row_right * 3.148117
                    + Vec3::Y * 1.029985)
                    .to_array(),
                target: (translation + Vec3::Y * 0.6).to_array(),
                fov_y: 0.85,
            };
            if index == 0 {
                scene.camera = camera;
                scene.car_rig = Some(built.rig.clone());
                scene.wheels = built.wheels.clone();
                first_bounds = Some(built.bounds);
            }
            bays.push(GarageBay {
                wheels: built.wheels,
                camera,
                name: format!("Bay {}", index + 1),
                color,
                color_name: BAY_COLOR_NAMES[color_index],
            });
        }
        scene.garage_bays = bays;
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
        scene.stats.textures += car_visual.textures.len() as u32;

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

fn build_car(
    scene: &mut Scene,
    car_visual: &CarVisual,
    car_texture_base: usize,
    transform: Mat4,
    body_color: [f32; 3],
    prefix: &str,
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
        wheels: rig_wheels,
        steering_mesh_index,
        steering_local,
    };
    Ok(CarBuild {
        rig,
        wheels,
        bounds: body_bounds,
    })
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
    });
    Ok(scene.meshes.len() - 1)
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
