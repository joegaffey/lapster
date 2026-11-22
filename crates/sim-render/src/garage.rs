use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use sim_types::{
    Aabb, AssetManifest, CarPhysics, CarVisual, MeshBuffers, MeshRef, NodeTransform,
    TrackCollision, TrackMetadata, TrackVisual,
};
use std::collections::{HashMap, HashSet};

use crate::camera::Camera;
use crate::renderer::{GarageLight, MAX_GARAGE_LIGHTS};
use crate::scene::{
    build_outline, build_vertices, push_line, scene_texture, CarRig, CockpitCover,
    CockpitGearSegment, CockpitNeedle, CockpitNeedleSource, GarageBay, LineVertex, MeshData,
    MeshVertex, RigWheel, Scene, SceneStats, SceneTexture, SceneWheel,
};
use crate::shadow_decal_yaw;

pub struct GarageCar<'a> {
    pub physics: &'a CarPhysics,
    pub visual: &'a CarVisual,
    pub manifest: &'a AssetManifest,
}

impl Scene {
    #[allow(clippy::too_many_arguments)]
    pub fn build_garage(
        track_visual: &TrackVisual,
        track_metadata: &TrackMetadata,
        track_collision: &TrackCollision,
        track_manifest: &AssetManifest,
        cars: &[GarageCar<'_>],
        car_count: usize,
        color_offset: usize,
        static_car_shadows: bool,
        brake_lights: bool,
    ) -> Result<Scene, String> {
        if cars.is_empty() {
            return Err("garage requires at least one car".to_string());
        }
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
        let mut car_texture_bases = Vec::with_capacity(cars.len());
        for car in cars {
            car_texture_bases.push(scene.textures.len());
            for texture in &car.visual.textures {
                scene.textures.push(scene_texture(texture)?);
            }
        }
        let silhouettes = if static_car_shadows {
            cars.iter().map(|_| None).collect::<Vec<_>>()
        } else {
            cars.iter()
                .map(|car| build_car_silhouette(car.visual))
                .collect::<Result<Vec<_>, _>>()?
        };
        let decal_textures = silhouettes
            .iter()
            .map(|silhouette| {
                silhouette.as_ref().map(|silhouette| {
                    let index = scene.textures.len();
                    scene.textures.push(silhouette.texture.clone());
                    index
                })
            })
            .collect::<Vec<_>>();

        let bay_count = car_count.clamp(1, 10);
        let mut bays = Vec::with_capacity(bay_count);
        let mut first_bounds = None;
        for index in 0..bay_count {
            let car_index = index % cars.len();
            let car = &cars[car_index];
            let car_texture_base = car_texture_bases[car_index];
            let silhouette = &silhouettes[car_index];
            let decal_texture = decal_textures[car_index];
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
                car.visual,
                car_texture_base,
                transform,
                color,
                &format!("bay_{index}"),
                brake_lights,
                true,
                car.physics.mass < 900.0,
            )?;
            if static_car_shadows {
                for mesh in &mut scene.meshes[car_mesh_start..] {
                    mesh.casts_static_shadow = true;
                    mesh.casts_dynamic_shadow = false;
                }
            }
            let row_forward = rotation * Vec3::NEG_Z;
            let camera = bay_camera(translation, rotation);
            let decal_mesh_index = match (silhouette.as_ref(), decal_texture) {
                (Some(silhouette), Some(texture)) => Some(add_car_decal(
                    &mut scene,
                    texture,
                    silhouette.width,
                    silhouette.length,
                )?),
                _ => None,
            };
            let mut rig = built.rig.clone();
            rig.decal_mesh_index = decal_mesh_index;
            rig.decal_width = silhouette.as_ref().map(|s| s.width).unwrap_or(1.0);
            rig.decal_length = silhouette.as_ref().map(|s| s.length).unwrap_or(1.0);
            scene.car_rigs.push(rig.clone());
            if index == 0 {
                scene.camera = camera;
                scene.car_rig = Some(rig);
                scene.wheels = built.wheels.clone();
                first_bounds = Some(built.bounds);
            }
            let decal_yaw = shadow_decal_yaw(row_forward);
            let car_info = json!({
                "id": car.manifest.id,
                "mass": car.physics.mass,
                "wheelbase": car.physics.wheelbase,
                "trackWidth": car.physics.track_width,
                "wheels": car.physics.wheels.iter().map(|wheel| json!({
                    "name": wheel.name,
                    "position": wheel.position,
                    "radius": wheel.radius,
                    "width": wheel.width,
                    "front": wheel.is_front,
                    "left": wheel.is_left,
                    "driven": wheel.driven,
                })).collect::<Vec<_>>(),
                "meshCount": car.visual.meshes.len(),
                "meshBufferCount": car.visual.mesh_buffers.len(),
                "steeringWheel": car.visual.steering_wheel.is_some(),
                "bounds": {
                    "min": built.bounds.min,
                    "max": built.bounds.max,
                },
            });
            bays.push(GarageBay {
                wheels: built.wheels,
                camera,
                name: format!("Bay {}", index + 1),
                car_id: car.manifest.id.clone(),
                car: car_info,
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

        let car_texture_count = cars
            .iter()
            .map(|car| car.visual.textures.len() as u32)
            .sum::<u32>();
        let decal_texture_count = decal_textures
            .iter()
            .filter(|texture| texture.is_some())
            .count() as u32;
        let first_car = &cars[0];

        scene.stats.vertices += car_vertices;
        scene.stats.triangles += car_triangles;
        scene.stats.draw_calls += car_draw_calls as u32;
        scene.stats.textures += car_texture_count + decal_texture_count;

        scene.debug_info = json!({
            "asset": { "id": track_manifest.id, "kind": "Track" },
            "car": {
                "id": first_car.manifest.id,
                "mass": first_car.physics.mass,
                "wheelbase": first_car.physics.wheelbase,
                "trackWidth": first_car.physics.track_width,
                "wheels": first_car.physics.wheels.iter().map(|wheel| json!({
                    "name": wheel.name,
                    "position": wheel.position,
                    "radius": wheel.radius,
                    "width": wheel.width,
                    "front": wheel.is_front,
                    "left": wheel.is_left,
                    "driven": wheel.driven,
                })).collect::<Vec<_>>(),
                "meshCount": first_car.visual.meshes.len(),
                "meshBufferCount": first_car.visual.mesh_buffers.len(),
                "steeringWheel": first_car.visual.steering_wheel.is_some(),
                "bounds": {
                    "min": body_bounds.min,
                    "max": body_bounds.max,
                },
            },
            "cars": cars.iter().map(|car| json!({
                "id": car.manifest.id,
                "meshCount": car.visual.meshes.len(),
            })).collect::<Vec<_>>(),
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
            "warnings": track_manifest.warnings.iter().chain(
                cars.iter().flat_map(|car| car.manifest.warnings.iter())
            ).cloned().collect::<Vec<_>>(),
        })
        .to_string();

        Ok(scene)
    }
}

/// Near-black studio ambient used by the concrete garage presentation.
pub(crate) const CONCRETE_AMBIENT: [f32; 3] = [0.045, 0.047, 0.055];

const CONCRETE_FLOOR_HALF_X: f32 = 60.0;
const CONCRETE_FLOOR_HALF_Z: f32 = 30.0;
const CONCRETE_FLOOR_TILE: f32 = 15.0;
const CONCRETE_LAMP_HEIGHT: f32 = 7.5;
const CONCRETE_LAMP_HALF_X: f32 = 1.2;
const CONCRETE_LAMP_HALF_Y: f32 = 0.08;
const CONCRETE_LAMP_HALF_Z: f32 = 0.38;
const CONCRETE_SPOT_RANGE: f32 = 16.0;
const CONCRETE_SPOT_INNER_COS: f32 = 0.94;
const CONCRETE_SPOT_OUTER_COS: f32 = 0.72;
const CONCRETE_SPOT_INTENSITY: f32 = 5.0;

pub(crate) struct ConcreteGarageBuild {
    pub(crate) scene: Scene,
    pub(crate) lights: Vec<GarageLight>,
}

/// Build the additive concrete-garage venue: one floor slab, a single line of
/// cars, and one overhead lamp and spot light per bay. No track data is used.
pub(crate) fn build_concrete_garage(
    cars: &[GarageCar<'_>],
    car_count: usize,
    color_offset: usize,
) -> Result<ConcreteGarageBuild, String> {
    if cars.is_empty() {
        return Err("garage requires at least one car".to_string());
    }
    let bay_count = car_count.clamp(1, MAX_GARAGE_LIGHTS);
    let mut scene = Scene {
        meshes: Vec::new(),
        textures: vec![concrete_floor_texture()],
        lines: Vec::new(),
        grass: crate::grass::GrassField::empty(),
        camera: Camera {
            eye: [3.15, 1.03, 4.86],
            target: [0.0, 0.6, 0.0],
            fov_y: 0.85,
        },
        debug_info: String::new(),
        wheels: Vec::new(),
        garage_bays: Vec::new(),
        car_mesh_range: None,
        stats: SceneStats {
            vertices: 0,
            triangles: 0,
            draw_calls: 0,
            textures: 1,
            collision_triangles: 0,
        },
        car_rig: None,
        car_rigs: Vec::new(),
        vegetation: None,
        mass_foliage: None,
        style_car_transform: None,
        style_car_bounds: None,
    };

    let floor = concrete_floor_mesh(0.0);
    let floor_triangles = (floor.indices.len() / 3) as u32;
    scene.meshes.push(floor);

    let mut car_texture_bases = Vec::with_capacity(cars.len());
    for car in cars {
        car_texture_bases.push(scene.textures.len());
        for texture in &car.visual.textures {
            scene.textures.push(scene_texture(texture)?);
        }
    }
    let silhouettes = cars
        .iter()
        .map(|car| build_car_silhouette(car.visual))
        .collect::<Result<Vec<_>, _>>()?;
    let decal_textures = silhouettes
        .iter()
        .map(|silhouette| {
            silhouette.as_ref().map(|silhouette| {
                let index = scene.textures.len();
                scene.textures.push(silhouette.texture.clone());
                index
            })
        })
        .collect::<Vec<_>>();

    let facing = Quat::from_rotation_y(std::f32::consts::PI);
    let car_start = scene.meshes.len();
    let mut bays = Vec::with_capacity(bay_count);
    let mut first_bounds = None;
    for index in 0..bay_count {
        let car_index = index % cars.len();
        let car = &cars[car_index];
        let car_texture_base = car_texture_bases[car_index];
        let silhouette = &silhouettes[car_index];
        let decal_texture = decal_textures[car_index];
        let color_index = (color_offset + index) % BAY_COLORS.len();
        let color = BAY_COLORS[color_index];
        let contact = wheel_contact_offset(car.visual, facing);
        let translation = Vec3::new(line_bay_x(index, bay_count), -contact, 0.0);
        let rotation = facing;
        let transform = Mat4::from_rotation_translation(rotation, translation);
        let built = build_car(
            &mut scene,
            car.visual,
            car_texture_base,
            transform,
            color,
            &format!("bay_{index}"),
            false,
            true,
            car.physics.mass < 900.0,
        )?;
        let camera = bay_camera(translation, rotation);
        let decal_mesh_index = match (silhouette.as_ref(), decal_texture) {
            (Some(silhouette), Some(texture)) => Some(add_car_decal(
                &mut scene,
                texture,
                silhouette.width,
                silhouette.length,
            )?),
            _ => None,
        };
        let mut rig = built.rig.clone();
        rig.decal_mesh_index = decal_mesh_index;
        rig.decal_width = silhouette.as_ref().map(|s| s.width).unwrap_or(1.0);
        rig.decal_length = silhouette.as_ref().map(|s| s.length).unwrap_or(1.0);
        scene.car_rigs.push(rig.clone());
        if index == 0 {
            scene.camera = camera;
            scene.car_rig = Some(rig);
            scene.wheels = built.wheels.clone();
            first_bounds = Some(built.bounds);
        }
        let row_forward = rotation * Vec3::NEG_Z;
        let decal_yaw = shadow_decal_yaw(row_forward);
        let car_info = json!({
            "id": car.manifest.id,
            "mass": car.physics.mass,
            "wheelbase": car.physics.wheelbase,
            "trackWidth": car.physics.track_width,
            "wheels": car.physics.wheels.iter().map(|wheel| json!({
                "name": wheel.name,
                "position": wheel.position,
                "radius": wheel.radius,
                "width": wheel.width,
                "front": wheel.is_front,
                "left": wheel.is_left,
                "driven": wheel.driven,
            })).collect::<Vec<_>>(),
            "meshCount": car.visual.meshes.len(),
            "meshBufferCount": car.visual.mesh_buffers.len(),
            "steeringWheel": car.visual.steering_wheel.is_some(),
            "bounds": {
                "min": built.bounds.min,
                "max": built.bounds.max,
            },
        });
        bays.push(GarageBay {
            wheels: built.wheels,
            camera,
            name: format!("Bay {}", index + 1),
            car_id: car.manifest.id.clone(),
            car: car_info,
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
    let car_end = scene.meshes.len();
    scene.garage_bays = bays;

    let mut lights = Vec::with_capacity(bay_count);
    for index in 0..bay_count {
        let x = line_bay_x(index, bay_count);
        let center = Vec3::new(x, CONCRETE_LAMP_HEIGHT, 0.0);
        add_lamp_mesh(&mut scene, index, center);
        lights.push(GarageLight {
            position: [x, CONCRETE_LAMP_HEIGHT - 0.15, 0.0],
            direction: [0.0, -1.0, 0.0],
            color: [1.0, 0.93, 0.80],
            intensity: CONCRETE_SPOT_INTENSITY,
            range: CONCRETE_SPOT_RANGE,
            inner_cos: CONCRETE_SPOT_INNER_COS,
            outer_cos: CONCRETE_SPOT_OUTER_COS,
        });
    }
    let lamp_meshes = bay_count;
    let spot_lights = lights.len();

    scene.stats = SceneStats {
        vertices: scene
            .meshes
            .iter()
            .map(|mesh| mesh.vertices.len() as u32)
            .sum(),
        triangles: scene
            .meshes
            .iter()
            .map(|mesh| mesh.indices.len() as u32 / 3)
            .sum(),
        draw_calls: scene.meshes.len() as u32,
        textures: scene.textures.len() as u32,
        collision_triangles: 0,
    };

    let car_draw_calls = car_end - car_start;
    let car_triangles = scene.meshes[car_start..car_end]
        .iter()
        .map(|mesh| mesh.indices.len() as u32 / 3)
        .sum::<u32>();
    let car_texture_count = cars
        .iter()
        .map(|car| car.visual.textures.len() as u32)
        .sum::<u32>();
    let decal_texture_count = decal_textures
        .iter()
        .filter(|texture| texture.is_some())
        .count() as u32;
    let first_car = &cars[0];
    let body_bounds = first_bounds.unwrap_or(Aabb {
        min: [0.0; 3],
        max: [0.0; 3],
    });

    scene.debug_info = json!({
        "asset": { "id": "concrete-garage", "kind": "Garage" },
        "car": {
            "id": first_car.manifest.id,
            "mass": first_car.physics.mass,
            "wheelbase": first_car.physics.wheelbase,
            "trackWidth": first_car.physics.track_width,
            "wheels": first_car.physics.wheels.iter().map(|wheel| json!({
                "name": wheel.name,
                "position": wheel.position,
                "radius": wheel.radius,
                "width": wheel.width,
                "front": wheel.is_front,
                "left": wheel.is_left,
                "driven": wheel.driven,
            })).collect::<Vec<_>>(),
            "meshCount": first_car.visual.meshes.len(),
            "meshBufferCount": first_car.visual.mesh_buffers.len(),
            "steeringWheel": first_car.visual.steering_wheel.is_some(),
            "bounds": {
                "min": body_bounds.min,
                "max": body_bounds.max,
            },
        },
        "cars": cars.iter().map(|car| json!({
            "id": car.manifest.id,
            "meshCount": car.visual.meshes.len(),
        })).collect::<Vec<_>>(),
        "garage": {
            "mode": "concrete",
            "layout": "line",
            "spotLights": spot_lights,
            "floorTriangles": floor_triangles,
            "lampMeshes": lamp_meshes,
            "bayCount": bay_count,
            "baySpacing": BAY_SPACING,
            "floorHalfExtent": [CONCRETE_FLOOR_HALF_X, CONCRETE_FLOOR_HALF_Z],
        },
        "stats": {
            "vertices": scene.stats.vertices,
            "triangles": scene.stats.triangles,
            "drawCalls": scene.stats.draw_calls,
            "textures": scene.stats.textures,
            "texturedMeshes": scene.meshes.iter().filter(|mesh| mesh.texture.is_some()).count(),
            "outlinedMeshes": scene.meshes.iter().filter(|mesh| mesh.outline.is_some()).count(),
            "collisionTriangles": 0,
            "trackTriangles": 0,
            "carTriangles": car_triangles,
            "trackDrawCalls": 0,
            "carDrawCalls": car_draw_calls,
            "carTextures": car_texture_count,
            "decalTextures": decal_texture_count,
        },
        "meshes": scene.meshes.iter().map(|mesh| json!({
            "name": mesh.name,
            "vertices": mesh.vertices.len(),
            "triangles": mesh.indices.len() / 3,
        })).collect::<Vec<_>>(),
        "warnings": cars.iter().flat_map(|car| car.manifest.warnings.iter()).cloned().collect::<Vec<_>>(),
    })
    .to_string();

    Ok(ConcreteGarageBuild { scene, lights })
}

fn line_bay_x(index: usize, bay_count: usize) -> f32 {
    let center = bay_count.saturating_sub(1) as f32 * 0.5;
    (index as f32 - center) * BAY_SPACING
}

fn wheel_contact_offset(car_visual: &CarVisual, rotation: Quat) -> f32 {
    let offset = car_visual
        .wheels
        .iter()
        .map(|wheel| {
            let center = rotation * Vec3::from_array(wheel.transform.translation);
            center.y - wheel.radius
        })
        .fold(f32::INFINITY, f32::min);
    if offset.is_finite() {
        offset
    } else {
        0.0
    }
}

fn bay_camera(translation: Vec3, rotation: Quat) -> Camera {
    let row_forward = rotation * Vec3::NEG_Z;
    let row_right = rotation * Vec3::X;
    Camera {
        eye: (translation + row_forward * 4.860569 - row_right * 3.148117 + Vec3::Y * 1.029985)
            .to_array(),
        target: (translation + Vec3::Y * 0.6).to_array(),
        fov_y: 0.85,
    }
}

fn concrete_floor_mesh(y: f32) -> MeshData {
    let positions = [
        [-CONCRETE_FLOOR_HALF_X, y, -CONCRETE_FLOOR_HALF_Z],
        [CONCRETE_FLOOR_HALF_X, y, -CONCRETE_FLOOR_HALF_Z],
        [CONCRETE_FLOOR_HALF_X, y, CONCRETE_FLOOR_HALF_Z],
        [-CONCRETE_FLOOR_HALF_X, y, CONCRETE_FLOOR_HALF_Z],
    ];
    let vertices = positions
        .iter()
        .map(|position| MeshVertex {
            position: *position,
            normal: [0.0, 1.0, 0.0],
            color: [0.62, 0.62, 0.60],
            uv: [
                (position[0] + CONCRETE_FLOOR_HALF_X) / CONCRETE_FLOOR_TILE,
                (position[2] + CONCRETE_FLOOR_HALF_Z) / CONCRETE_FLOOR_TILE,
            ],
            wind: [0.0, 0.0],
        })
        .collect();
    MeshData {
        name: "garage_floor".to_string(),
        vertices,
        // Counter-clockwise as seen from above so the normal points up.
        indices: vec![0, 2, 1, 0, 3, 2],
        model: Mat4::IDENTITY,
        texture: Some(0),
        grass_texture: None,
        outline: None,
        alpha: 1.0,
        emissive: [0.0; 3],
        casts_static_shadow: false,
        casts_dynamic_shadow: false,
        is_decal: false,
        foliage: false,
        billboard: false,
        track_blend: None,
    }
}

fn concrete_floor_texture() -> SceneTexture {
    let width = 256usize;
    let height = 256usize;
    let mut rgba8 = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let noise = concrete_noise(u, v);
            let joint = 1.0
                - smoothstep(
                    0.0,
                    0.012,
                    joint_distance(u * 2.0).min(joint_distance(v * 2.0)),
                );
            let value = (0.52 + (noise - 0.5) * 0.16 - joint * 0.18).clamp(0.0, 1.0);
            let grey = (value * 255.0 + 0.5) as u8;
            rgba8.extend_from_slice(&[grey, grey, grey, 255]);
        }
    }
    SceneTexture {
        width: width as u32,
        height: height as u32,
        rgba8,
    }
}

fn joint_distance(value: f32) -> f32 {
    let fraction = value.fract();
    fraction.min(1.0 - fraction)
}

fn concrete_noise(u: f32, v: f32) -> f32 {
    const OCTAVES: [(i32, u32); 4] = [
        (8, 0x9e37_79b9),
        (16, 0x85eb_ca77),
        (32, 0xc2b2_ae3d),
        (64, 0x27d4_eb2f),
    ];
    let mut sum = 0.0;
    let mut amplitude = 0.5;
    let mut total = 0.0;
    for (period, seed) in OCTAVES {
        sum += tileable_value_noise(u, v, period, seed) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
    }
    sum / total
}

fn tileable_value_noise(u: f32, v: f32, period: i32, seed: u32) -> f32 {
    let x = u * period as f32;
    let y = v * period as f32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let top = lattice_value(x0, y0, period, seed)
        + (lattice_value(x0 + 1, y0, period, seed) - lattice_value(x0, y0, period, seed)) * sx;
    let bottom = lattice_value(x0, y0 + 1, period, seed)
        + (lattice_value(x0 + 1, y0 + 1, period, seed) - lattice_value(x0, y0 + 1, period, seed))
            * sx;
    top + (bottom - top) * sy
}

fn lattice_value(x: i32, y: i32, period: i32, seed: u32) -> f32 {
    let x = x.rem_euclid(period) as u32;
    let y = y.rem_euclid(period) as u32;
    let mut hash = x
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(y.wrapping_mul(0x85eb_ca77))
        .wrapping_add(seed);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;
    (hash & 0xffff) as f32 / 65535.0
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn add_lamp_mesh(scene: &mut Scene, index: usize, center: Vec3) {
    let half = Vec3::new(
        CONCRETE_LAMP_HALF_X,
        CONCRETE_LAMP_HALF_Y,
        CONCRETE_LAMP_HALF_Z,
    );
    let min = center - half;
    let max = center + half;
    let mut buffers = MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    };
    let color = [0.16, 0.15, 0.13];
    let faces: [([Vec3; 4], Vec3); 6] = [
        (
            [
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(max.x, min.y, max.z),
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(max.x, max.y, min.z),
            ],
            Vec3::X,
        ),
        (
            [
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(min.x, max.y, max.z),
            ],
            Vec3::NEG_X,
        ),
        (
            [
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(max.x, max.y, min.z),
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(min.x, max.y, max.z),
            ],
            Vec3::Y,
        ),
        (
            [
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(max.x, min.y, max.z),
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(min.x, min.y, min.z),
            ],
            Vec3::NEG_Y,
        ),
        (
            [
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(min.x, max.y, max.z),
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(max.x, min.y, max.z),
            ],
            Vec3::Z,
        ),
        (
            [
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(max.x, max.y, min.z),
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(min.x, min.y, min.z),
            ],
            Vec3::NEG_Z,
        ),
    ];
    for (corners, normal) in faces {
        let base = buffers.positions.len() as u32;
        for corner in corners {
            buffers.positions.push(corner.to_array());
            buffers.normals.push(normal.to_array());
            buffers.uvs.push([0.0, 0.0]);
            buffers.colors.push(color);
        }
        buffers
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let vertices = build_vertices(&buffers, color);
    scene.meshes.push(MeshData {
        name: format!("garage_lamp_{index}"),
        vertices,
        indices: buffers.indices,
        model: Mat4::IDENTITY,
        texture: None,
        grass_texture: None,
        outline: None,
        alpha: 1.0,
        emissive: [1.6, 1.35, 0.9],
        casts_static_shadow: false,
        casts_dynamic_shadow: false,
        is_decal: false,
        foliage: false,
        billboard: false,
        track_blend: None,
    });
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
            wind: [0.0, 0.0],
        });
    }
    let _ = (width, length);
    scene.meshes.push(MeshData {
        name: "car_shadow_decal".to_string(),
        vertices,
        indices: vec![0, 1, 2, 0, 2, 3],
        model: Mat4::IDENTITY,
        texture: Some(texture),
        grass_texture: None,
        outline: None,
        alpha: 0.20,
        emissive: [0.0; 3],
        casts_static_shadow: false,
        casts_dynamic_shadow: false,
        is_decal: true,
        foliage: false,
        billboard: false,
        track_blend: None,
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

pub(crate) struct CarBuild {
    pub(crate) rig: CarRig,
    pub(crate) wheels: Vec<SceneWheel>,
    pub(crate) bounds: Aabb,
    pub(crate) ground_y: f32,
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

type CockpitDial = (CockpitNeedleSource, [f32; 3], f32, f32);

const COCKPIT_DIALS: [CockpitDial; 3] = [
    (
        CockpitNeedleSource::Speed,
        [-0.442, 0.935, -0.22],
        0.044,
        0.035,
    ),
    (
        CockpitNeedleSource::Rpm,
        [-0.35, 0.935, -0.22],
        0.050,
        0.045,
    ),
    (
        CockpitNeedleSource::Gear,
        [-0.258, 0.935, -0.22],
        0.044,
        0.035,
    ),
];

const CAMARO_COCKPIT_DIALS: [CockpitDial; 3] = [
    (
        CockpitNeedleSource::Speed,
        [-0.515, 0.945, -0.32],
        0.0419,
        0.0253,
    ),
    (
        CockpitNeedleSource::Rpm,
        [-0.315, 0.945, -0.32],
        0.0448,
        0.0253,
    ),
    (
        CockpitNeedleSource::Gear,
        [-0.415, 0.935, -0.305],
        0.0361,
        0.0217,
    ),
];

const OPEN_WHEEL_COCKPIT_DIALS: [CockpitDial; 1] =
    [(CockpitNeedleSource::Rpm, [0.0, 0.64, -0.32], 0.050, 0.045)];

/// Select the cockpit dial layout. Open-wheel cars always use the single-RPM
/// open-wheel layout, even when they retain a source interior mesh.
fn select_cockpit_dials(open_wheel: bool, has_interior: bool) -> &'static [CockpitDial] {
    if open_wheel {
        &OPEN_WHEEL_COCKPIT_DIALS
    } else if has_interior {
        &CAMARO_COCKPIT_DIALS
    } else {
        &COCKPIT_DIALS
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_car(
    scene: &mut Scene,
    car_visual: &CarVisual,
    car_texture_base: usize,
    transform: Mat4,
    body_color: [f32; 3],
    prefix: &str,
    brake_lights: bool,
    cockpit_gauges: bool,
    open_wheel: bool,
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
    let textured_body = car_visual
        .materials
        .get(body_mesh.material as usize)
        .is_some_and(|material| material.base_color_texture.is_some());
    let has_interior = car_visual
        .meshes
        .iter()
        .any(|mesh| mesh.name == "body_interior");
    let synthetic_cockpit = cockpit_gauges && brake_lights && !has_interior;
    let cockpit_dials = select_cockpit_dials(open_wheel, has_interior);
    let has_cockpit_gauges = synthetic_cockpit || has_interior;
    let (brake_light_buffers, reverse_light_buffers) = if brake_lights {
        split_rear_lights(&mut body_buffers, &body_mesh.bounds, textured_body)
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

    let mut extra_body_meshes = Vec::new();
    for (index, mesh) in car_visual.meshes.iter().enumerate() {
        if index == body_index || mesh.name == "body_glass" || !mesh.name.starts_with("body_") {
            continue;
        }
        let buffers = car_visual
            .mesh_buffers
            .get(index)
            .ok_or_else(|| format!("car mesh {} has no buffers", mesh.name))?;
        let color = car_material_color(car_visual, mesh.material as usize);
        let mesh_index = push_car_mesh(
            scene,
            &format!("{prefix}_{}", mesh.name),
            mesh,
            buffers,
            color,
            body_model,
            CarMeshStyle {
                texture: car_texture(car_visual, mesh, car_texture_base)?,
                outline: car_outline(car_visual, mesh),
                alpha: car_material_alpha(car_visual, mesh.material as usize),
            },
        )?;
        extra_body_meshes.push(mesh_index);
    }

    let body_texture = car_texture(car_visual, body_mesh, car_texture_base)?;
    let brake_light_mesh_index = if let Some(buffers) = brake_light_buffers {
        Some(push_car_light_mesh(
            scene,
            prefix,
            "brake_lights",
            body_mesh,
            &buffers,
            body_model,
            body_texture,
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
            body_texture,
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

    let dashboard_mesh_index = if synthetic_cockpit && !open_wheel {
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
                [0.0, 0.0, 0.0],
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

    let mut cockpit_covers = Vec::new();
    if has_cockpit_gauges {
        for &(source, center, radius, depth) in cockpit_dials {
            if source == CockpitNeedleSource::Gear {
                continue;
            }
            cockpit_covers.push(CockpitCover {
                mesh_index: push_dial_cover(
                    scene,
                    prefix,
                    center,
                    radius * 1.18,
                    depth - 0.004,
                    body_model,
                )?,
                source,
            });
        }
    }

    let speedometer_face_mesh_index = if has_cockpit_gauges {
        push_dial_face(
            scene,
            prefix,
            CockpitNeedleSource::Speed,
            cockpit_dials,
            body_model,
        )?
    } else {
        None
    };
    let rpm_face_mesh_index = if has_cockpit_gauges {
        push_dial_face(
            scene,
            prefix,
            CockpitNeedleSource::Rpm,
            cockpit_dials,
            body_model,
        )?
    } else {
        None
    };

    let mut cockpit_needles = Vec::new();
    if has_cockpit_gauges {
        for &(source, center, radius, depth) in cockpit_dials {
            if source == CockpitNeedleSource::Gear {
                continue;
            }
            let buffers = needle_buffers(radius);
            let center = Vec3::new(center[0], center[1], center[2] + depth * 0.5 - 0.0035);
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
                [1.0, 0.02, 0.01],
                body_model * Mat4::from_translation(center),
                CarMeshStyle {
                    texture: None,
                    outline: false,
                    alpha: 1.0,
                },
            )?;
            scene.meshes[mesh_index].emissive = [1.0, 0.0, 0.0];
            scene.meshes[mesh_index].casts_static_shadow = false;
            scene.meshes[mesh_index].casts_dynamic_shadow = false;
            cockpit_needles.push(CockpitNeedle {
                mesh_index,
                center,
                source,
            });
        }
    }

    let mut gear_segments = Vec::new();
    if has_cockpit_gauges {
        if let Some((_, center, radius, depth)) = cockpit_dials
            .iter()
            .find(|(source, _, _, _)| *source == CockpitNeedleSource::Gear)
        {
            let z = center[2] + depth * 0.5 - 0.003;
            let half_width = radius * 0.75;
            let half_height = half_width * (512.0 / 3.0) / (844.0 / 4.0);
            let gear_cells = [
                (0, 1, 2),
                (-1, 2, 2),
                (1, 0, 0),
                (2, 1, 0),
                (3, 2, 0),
                (4, 3, 0),
                (5, 0, 1),
                (6, 1, 1),
            ];
            for (gear, column, row) in gear_cells {
                let u0 = column as f32 / 4.0;
                let v0 = row as f32 / 3.0;
                let segment_center = Vec3::new(center[0], center[1], z);
                let buffers = sprite_quad_buffers(
                    half_width,
                    half_height,
                    [1.0, 1.0, 1.0],
                    u0,
                    v0,
                    u0 + 0.25,
                    v0 + 1.0 / 3.0,
                );
                let mesh_ref = MeshRef {
                    name: format!("gear_digit_{}", gear),
                    vertex_offset: 0,
                    vertex_count: 4,
                    index_offset: 0,
                    index_count: 6,
                    material: 0,
                    bounds: Aabb {
                        min: [-half_width, -half_height, 0.0],
                        max: [half_width, half_height, 0.0],
                    },
                    lod: 0,
                };
                let mesh_index = push_car_mesh(
                    scene,
                    &format!("{}_gear_{}", prefix, gear),
                    &mesh_ref,
                    &buffers,
                    [1.0, 1.0, 1.0],
                    body_model * Mat4::from_translation(segment_center),
                    CarMeshStyle {
                        texture: None,
                        outline: false,
                        alpha: 1.0,
                    },
                )?;
                scene.meshes[mesh_index].casts_static_shadow = false;
                scene.meshes[mesh_index].casts_dynamic_shadow = false;
                gear_segments.push(CockpitGearSegment {
                    mesh_index,
                    center: segment_center,
                    gear,
                });
            }
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
        extra_body_meshes,
        glass_mesh_index,
        brake_light_mesh_index,
        reverse_light_mesh_index,
        dashboard_mesh_index,
        dashboard_local: body_local,
        speedometer_face_mesh_index,
        rpm_face_mesh_index,
        cockpit_needles,
        cockpit_covers,
        gear_segments,
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
    let top_color = [0.0, 0.0, 0.0];
    let bulkhead_color = [0.0, 0.0, 0.0];
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
    for (source, center, radius, depth) in COCKPIT_DIALS {
        append_dial(&mut buffers, center, radius, depth);
        if source == CockpitNeedleSource::Gear {
            append_gear_face(&mut buffers, center, radius, depth);
        }
    }
    Some(buffers)
}

fn push_dial_cover(
    scene: &mut Scene,
    prefix: &str,
    center: [f32; 3],
    radius: f32,
    depth: f32,
    body_model: Mat4,
) -> Result<usize, String> {
    let mut buffers = dial_face_buffers(center, radius, depth);
    buffers.colors.fill([0.004, 0.005, 0.006]);
    let mesh_ref = MeshRef {
        name: format!("{}_dial_cover", prefix),
        vertex_offset: 0,
        vertex_count: buffers.positions.len() as u32,
        index_offset: 0,
        index_count: buffers.indices.len() as u32,
        material: 0,
        bounds: Aabb {
            min: [center[0] - radius, center[1] - radius, center[2]],
            max: [center[0] + radius, center[1] + radius, center[2]],
        },
        lod: 0,
    };
    let index = push_car_mesh(
        scene,
        &format!("{}_dial_cover_{}", prefix, scene.meshes.len()),
        &mesh_ref,
        &buffers,
        [0.004, 0.005, 0.006],
        body_model,
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

fn push_dial_face(
    scene: &mut Scene,
    prefix: &str,
    source: CockpitNeedleSource,
    dials: &[CockpitDial],
    body_model: Mat4,
) -> Result<Option<usize>, String> {
    let Some((_, center, radius, depth)) = dials.iter().find(|(dial, _, _, _)| *dial == source)
    else {
        return Ok(None);
    };
    let buffers = dial_face_buffers(*center, *radius, *depth);
    let mesh_ref = MeshRef {
        name: format!("{}_face_{}", prefix, source as u8),
        vertex_offset: 0,
        vertex_count: buffers.positions.len() as u32,
        index_offset: 0,
        index_count: buffers.indices.len() as u32,
        material: 0,
        bounds: Aabb {
            min: [center[0] - radius, center[1] - radius, center[2]],
            max: [center[0] + radius, center[1] + radius, center[2]],
        },
        lod: 0,
    };
    let index = push_car_mesh(
        scene,
        &format!("{}_face_{}", prefix, source as u8),
        &mesh_ref,
        &buffers,
        [1.0, 1.0, 1.0],
        body_model,
        CarMeshStyle {
            texture: None,
            outline: false,
            alpha: 1.0,
        },
    )?;
    scene.meshes[index].casts_static_shadow = false;
    scene.meshes[index].casts_dynamic_shadow = false;
    Ok(Some(index))
}

fn append_dial(buffers: &mut MeshBuffers, center: [f32; 3], radius: f32, depth: f32) {
    let segments = 24;
    let z_front = center[2] + depth * 0.5;
    let z_back = center[2] - depth * 0.5;
    let outer = [0.0, 0.0, 0.0];
    let face = [0.0, 0.0, 0.0];
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
    let white_z = z_front - 0.006;
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

fn dial_face_buffers(center: [f32; 3], radius: f32, depth: f32) -> MeshBuffers {
    let segments = 48;
    let z = center[2] + depth * 0.5 - 0.0045;
    let radius = radius * 0.94;
    let tau = std::f32::consts::TAU;
    let mut positions = Vec::with_capacity(segments + 1);
    let mut normals = Vec::with_capacity(segments + 1);
    let mut uvs = Vec::with_capacity(segments + 1);
    let mut colors = Vec::with_capacity(segments + 1);
    let mut indices = Vec::with_capacity(segments * 3);
    positions.push([center[0], center[1], z]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);
    colors.push([1.0, 1.0, 1.0]);
    for index in 0..segments {
        let angle = tau * index as f32 / segments as f32;
        let (sin, cos) = angle.sin_cos();
        positions.push([center[0] + cos * radius, center[1] + sin * radius, z]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.5 + cos * 0.5, 0.5 - sin * 0.5]);
        colors.push([1.0, 1.0, 1.0]);
    }
    for index in 0..segments {
        let next = (index + 1) % segments;
        indices.extend_from_slice(&[0, index as u32 + 1, next as u32 + 1]);
    }
    MeshBuffers {
        positions,
        normals,
        uvs,
        colors,
        indices,
    }
}

fn append_gear_face(buffers: &mut MeshBuffers, center: [f32; 3], radius: f32, depth: f32) {
    let segments = 24;
    let z = center[2] + depth * 0.5 - 0.004;
    let radius = radius * 0.85;
    let color = [0.004, 0.005, 0.006];
    let tau = std::f32::consts::TAU;
    let center_index = buffers.positions.len() as u32;
    buffers.positions.push([center[0], center[1], z]);
    buffers.normals.push([0.0, 0.0, 1.0]);
    buffers.uvs.push([0.5, 0.5]);
    buffers.colors.push(color);
    let ring = buffers.positions.len() as u32;
    for index in 0..segments {
        let angle = tau * index as f32 / segments as f32;
        let (sin, cos) = angle.sin_cos();
        buffers
            .positions
            .push([center[0] + cos * radius, center[1] + sin * radius, z]);
        buffers.normals.push([0.0, 0.0, 1.0]);
        buffers.uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
        buffers.colors.push(color);
    }
    for index in 0..segments {
        let next = (index + 1) % segments;
        buffers
            .indices
            .extend_from_slice(&[center_index, ring + index as u32, ring + next as u32]);
    }
}

fn sprite_quad_buffers(
    half_width: f32,
    half_height: f32,
    color: [f32; 3],
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
) -> MeshBuffers {
    MeshBuffers {
        positions: vec![
            [-half_width, -half_height, 0.0],
            [half_width, -half_height, 0.0],
            [half_width, half_height, 0.0],
            [-half_width, half_height, 0.0],
        ],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        colors: vec![color; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn needle_buffers(radius: f32) -> MeshBuffers {
    let width = radius * 0.02;
    let length = radius * 0.55;
    MeshBuffers {
        positions: vec![[-width, 0.0, 0.0], [width, 0.0, 0.0], [0.0, length, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
        colors: vec![[1.0, 0.02, 0.01]; 3],
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
        grass_texture: None,
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
        foliage: false,
        billboard: false,
        track_blend: None,
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
    texture: Option<usize>,
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
            texture,
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
    skip_plain_white: bool,
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
        if skip_plain_white
            && triangle.iter().all(|index| {
                buffers
                    .colors
                    .get(*index as usize)
                    .is_some_and(|color| *color == [1.0, 1.0, 1.0])
            })
        {
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
        if skip_plain_white {
            let lateral = triangle
                .iter()
                .filter_map(|index| buffers.positions.get(*index as usize))
                .map(|position| position[0].abs())
                .sum::<f32>()
                / 3.0;
            if lateral < 0.52 {
                push_light_triangle(&mut reverse, buffers, triangle);
            } else {
                push_light_triangle(&mut brake, buffers, triangle);
            }
        } else if red_orange {
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

pub(crate) fn car_transform(metadata: &TrackMetadata) -> Result<Mat4, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cockpit_dials_prefer_open_wheel_over_source_interior() {
        assert_eq!(
            select_cockpit_dials(true, true),
            OPEN_WHEEL_COCKPIT_DIALS.as_slice()
        );
        assert_eq!(
            select_cockpit_dials(true, false),
            OPEN_WHEEL_COCKPIT_DIALS.as_slice()
        );
        assert_eq!(
            select_cockpit_dials(false, true),
            CAMARO_COCKPIT_DIALS.as_slice()
        );
        assert_eq!(select_cockpit_dials(false, false), COCKPIT_DIALS.as_slice());
    }

    #[test]
    fn concrete_line_layout_is_centered_and_constant_spacing() {
        let bay_count = 10;
        let xs = (0..bay_count)
            .map(|index| line_bay_x(index, bay_count))
            .collect::<Vec<_>>();
        assert!((xs[0] + xs[bay_count - 1]).abs() < 1e-4);
        for pair in xs.windows(2) {
            let spacing = pair[1] - pair[0];
            assert!((spacing - BAY_SPACING).abs() < 1e-4);
            assert!((7.5..=8.0).contains(&spacing));
        }
        let spacing = xs[1] - xs[0];
        assert!((7.5..=8.0).contains(&spacing));
    }

    #[test]
    fn concrete_floor_has_upward_winding_and_full_slab() {
        let floor = concrete_floor_mesh(0.0);
        assert_eq!(floor.indices.len() / 3, 2);
        assert!(floor
            .vertices
            .iter()
            .all(|vertex| vertex.normal == [0.0, 1.0, 0.0]));
        let width = floor
            .vertices
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::NEG_INFINITY, f32::max)
            - floor
                .vertices
                .iter()
                .map(|vertex| vertex.position[0])
                .fold(f32::INFINITY, f32::min);
        let depth = floor
            .vertices
            .iter()
            .map(|vertex| vertex.position[2])
            .fold(f32::NEG_INFINITY, f32::max)
            - floor
                .vertices
                .iter()
                .map(|vertex| vertex.position[2])
                .fold(f32::INFINITY, f32::min);
        assert!((width - 2.0 * CONCRETE_FLOOR_HALF_X).abs() < 1e-4);
        assert!((depth - 2.0 * CONCRETE_FLOOR_HALF_Z).abs() < 1e-4);
        let a = Vec3::from_array(floor.vertices[floor.indices[0] as usize].position);
        let b = Vec3::from_array(floor.vertices[floor.indices[1] as usize].position);
        let c = Vec3::from_array(floor.vertices[floor.indices[2] as usize].position);
        assert!((b - a).cross(c - a).y > 0.0);
    }

    #[test]
    fn concrete_texture_is_greyscale_and_tileable_at_seam() {
        let texture = concrete_floor_texture();
        assert_eq!(texture.width, 256);
        assert_eq!(texture.height, 256);
        assert_eq!(texture.rgba8.len(), 256 * 256 * 4);
        let first = texture.rgba8[0];
        let last = texture.rgba8[(255 * 4) as usize];
        assert!(first.abs_diff(last) < 6);
        for start in (0..texture.rgba8.len()).step_by(4) {
            let pixel = &texture.rgba8[start..start + 4];
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
            assert_eq!(pixel[3], 255);
        }
    }

    #[test]
    fn concrete_wheel_contact_offset_plants_wheels_at_zero() {
        let identity = NodeTransform {
            translation: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        };
        let wheel = sim_types::WheelVisual {
            name: "wheel".to_string(),
            mesh: 0,
            transform: NodeTransform {
                translation: [0.0, 0.5, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            radius: 0.34,
            width: 0.2,
            is_front: true,
            is_left: true,
        };
        let visual = CarVisual {
            meshes: Vec::new(),
            mesh_buffers: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            wheels: [wheel.clone(), wheel.clone(), wheel.clone(), wheel],
            body: identity,
            steering_wheel: None,
        };
        let offset = wheel_contact_offset(&visual, Quat::IDENTITY);
        assert!((offset - 0.16).abs() < 1e-5);
        let flipped = wheel_contact_offset(&visual, Quat::from_rotation_y(std::f32::consts::PI));
        assert!((flipped - 0.16).abs() < 1e-5);
    }
}
