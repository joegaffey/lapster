use std::path::Path;

use anyhow::{bail, Context, Result};
use glam::{Mat3, Mat4, Vec3};
use gltf::image::Format;
use sim_types::{
    Aabb, AeroSpec, BrakeSpec, CarPhysics, CarVisual, DriveLayout, EngineSpec, MaterialRef,
    MeshBuffers, MeshRef, NodeTransform, PacejkaCoefficients, SteeringSpec, SuspensionSpec,
    TireSpec, TorquePoint, WheelPhysics, WheelVisual,
};

use crate::mesh::{bounds, merge_bounds};

const BODY_PAINT: [f32; 3] = [0.42, 0.015, 0.015];

pub struct ImportedCar {
    pub physics: CarPhysics,
    pub visual: CarVisual,
    pub bounds: Aabb,
    pub warnings: Vec<String>,
}

struct WheelImport {
    source_name: String,
    mesh: MeshBuffers,
    center: Vec3,
}

pub fn import_car(path: &Path) -> Result<ImportedCar> {
    let (document, buffers, images) =
        gltf::import(path).with_context(|| format!("failed to import {}", path.display()))?;
    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .context("GLB has no scene")?;

    let yaw = Mat4::from_rotation_y(std::f32::consts::PI);
    let mut body = empty_mesh();
    let mut glass = empty_mesh();
    let mut wheels = Vec::new();
    let mut warnings = Vec::new();
    let mut removed_logos = 0;
    let mut stack = scene
        .nodes()
        .map(|node| (node, Mat4::IDENTITY))
        .collect::<Vec<_>>();

    while let Some((node, parent)) = stack.pop() {
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = parent * local;
        let car = yaw * world;
        let name = node.name().unwrap_or("").to_string();
        let upper = name.to_ascii_uppercase();

        if let Some(mesh) = node.mesh() {
            if upper.contains("WHEEL") {
                let center = car.transform_point3(Vec3::ZERO);
                let mut wheel_mesh = empty_mesh();
                for primitive in mesh.primitives() {
                    append_primitive(
                        &mut wheel_mesh,
                        &primitive,
                        &buffers,
                        &images,
                        car,
                        Some(center),
                    )?;
                }
                wheels.push(WheelImport {
                    source_name: name,
                    mesh: wheel_mesh,
                    center,
                });
            } else if upper.contains("LOGO") {
                removed_logos += 1;
            } else {
                for primitive in mesh.primitives() {
                    let material = primitive.material();
                    if material.name() == Some("Glass") {
                        append_primitive(&mut glass, &primitive, &buffers, &images, car, None)?;
                    } else {
                        append_primitive(&mut body, &primitive, &buffers, &images, car, None)?;
                    }
                }
            }
        }

        for child in node.children() {
            stack.push((child, world));
        }
    }

    if body.positions.is_empty() || body.indices.is_empty() {
        bail!("GLB has no body mesh");
    }
    if wheels.len() != 4 {
        bail!(
            "GLB must contain four wheel nodes; found {} ({:?})",
            wheels.len(),
            wheels
                .iter()
                .map(|wheel| wheel.source_name.as_str())
                .collect::<Vec<_>>()
        );
    }

    wheels.sort_by(|a, b| {
        let a_front = a.center.z < 0.0;
        let a_left = a.center.x < 0.0;
        let b_front = b.center.z < 0.0;
        let b_left = b.center.x < 0.0;
        (!a_front, !a_left).cmp(&(!b_front, !b_left))
    });

    let body_bounds = bounds(&body);
    let body_ref = mesh_ref("body", &body, 0);
    let mut meshes = vec![body_ref];
    let mut mesh_buffers = vec![body];
    let mut wheel_visuals = Vec::with_capacity(4);
    let mut world_bounds = Some(body_bounds);

    for (index, wheel) in wheels.into_iter().enumerate() {
        let wheel_bounds = bounds(&wheel.mesh);
        let extent = [
            wheel_bounds.max[0] - wheel_bounds.min[0],
            wheel_bounds.max[1] - wheel_bounds.min[1],
            wheel_bounds.max[2] - wheel_bounds.min[2],
        ];
        let width = extent[0];
        let radius = extent[1].max(extent[2]) * 0.5;
        let is_front = wheel.center.z < 0.0;
        let is_left = wheel.center.x < 0.0;
        let label = match (is_front, is_left) {
            (true, true) => "FL",
            (true, false) => "FR",
            (false, true) => "RL",
            (false, false) => "RR",
        };
        let mesh_index = index + 1;
        meshes.push(mesh_ref(&format!("wheel_{label}"), &wheel.mesh, 1));
        mesh_buffers.push(wheel.mesh.clone());
        wheel_visuals.push(WheelVisual {
            name: label.to_string(),
            mesh: mesh_index as u32,
            transform: NodeTransform {
                translation: wheel.center.to_array(),
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            radius,
            width,
            is_front,
            is_left,
        });

        let translated = Aabb {
            min: [
                wheel_bounds.min[0] + wheel.center.x,
                wheel_bounds.min[1] + wheel.center.y,
                wheel_bounds.min[2] + wheel.center.z,
            ],
            max: [
                wheel_bounds.max[0] + wheel.center.x,
                wheel_bounds.max[1] + wheel.center.y,
                wheel_bounds.max[2] + wheel.center.z,
            ],
        };
        world_bounds = Some(merge_bounds(world_bounds, &translated));
    }

    if !glass.positions.is_empty() && !glass.indices.is_empty() {
        meshes.push(mesh_ref("body_glass", &glass, 2));
        mesh_buffers.push(glass);
    }

    let wheels: [WheelVisual; 4] = wheel_visuals
        .try_into()
        .map_err(|_| anyhow::anyhow!("GLB did not produce four wheel visuals"))?;
    let materials = vec![
        MaterialRef {
            name: String::from("imported_body"),
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.6,
            base_color_texture: None,
            outline: true,
        },
        MaterialRef {
            name: String::from("imported_wheel"),
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.8,
            base_color_texture: None,
            outline: true,
        },
        MaterialRef {
            name: String::from("imported_glass"),
            base_color: [0.035, 0.045, 0.055, 0.32],
            metallic: 0.0,
            roughness: 0.1,
            base_color_texture: None,
            outline: false,
        },
    ];

    let visual = CarVisual {
        meshes,
        materials,
        textures: Vec::new(),
        mesh_buffers,
        body: NodeTransform {
            translation: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        },
        wheels,
        steering_wheel: None,
    };
    let physics = physics_from_visual(&visual.wheels)?;

    if removed_logos > 0 {
        warnings.push(format!("removed {removed_logos} logo node(s)"));
    }
    warnings.push("baked source materials and palette texture into vertex colours".to_string());
    warnings.push("body paint defaulted to a demo red".to_string());
    warnings.push("glass split into a semi-transparent mesh".to_string());
    warnings.push("no steering wheel or interior found".to_string());

    Ok(ImportedCar {
        physics,
        visual,
        bounds: world_bounds.unwrap_or(body_bounds),
        warnings,
    })
}

fn empty_mesh() -> MeshBuffers {
    MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    }
}

fn mesh_ref(name: &str, mesh: &MeshBuffers, material: u16) -> MeshRef {
    MeshRef {
        name: name.to_string(),
        vertex_offset: 0,
        vertex_count: mesh.positions.len() as u32,
        index_offset: 0,
        index_count: mesh.indices.len() as u32,
        material,
        bounds: bounds(mesh),
        lod: 0,
    }
}

fn append_primitive(
    output: &mut MeshBuffers,
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    matrix: Mat4,
    center: Option<Vec3>,
) -> Result<()> {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        bail!("only triangle primitives are supported");
    }
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
    let positions = reader
        .read_positions()
        .context("primitive has no POSITION attribute")?
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return Ok(());
    }
    let mut indices = reader
        .read_indices()
        .map(|indices| indices.into_u32().collect::<Vec<_>>())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    indices.truncate(indices.len() / 3 * 3);
    if indices.is_empty() {
        return Ok(());
    }

    let mut normals = reader
        .read_normals()
        .map(|normals| normals.collect::<Vec<_>>())
        .unwrap_or_default();
    if normals.len() != positions.len() {
        normals = compute_normals(&positions, &indices);
    }
    let uvs = reader
        .read_tex_coords(0)
        .map(|uvs| uvs.into_f32().collect::<Vec<_>>())
        .unwrap_or_default();
    let normal_matrix = Mat3::from_mat4(matrix).inverse().transpose();
    let base = output.positions.len() as u32;
    let offset = center.unwrap_or(Vec3::ZERO);

    for (index, position) in positions.iter().enumerate() {
        let world = matrix.transform_point3(Vec3::from_array(*position)) - offset;
        let normal = normal_matrix * Vec3::from_array(normals[index]);
        let uv = uvs.get(index).copied().unwrap_or([0.0, 0.0]);
        output.positions.push(world.to_array());
        output.normals.push(normal.normalize_or_zero().to_array());
        output.uvs.push(uv);
        output
            .colors
            .push(material_color(&primitive.material(), images, uv));
    }
    output
        .indices
        .extend(indices.into_iter().map(|index| base + index));
    Ok(())
}

fn material_color(
    material: &gltf::Material<'_>,
    images: &[gltf::image::Data],
    uv: [f32; 2],
) -> [f32; 3] {
    let pbr = material.pbr_metallic_roughness();
    let factor = pbr.base_color_factor();
    let mut color = [factor[0], factor[1], factor[2]];
    if material.name() == Some("Body_paint") {
        color = BODY_PAINT;
    }
    if let Some(info) = pbr.base_color_texture() {
        if let Some(image) = images.get(info.texture().source().index()) {
            let sample = sample_image(image, uv);
            color[0] *= sample[0];
            color[1] *= sample[1];
            color[2] *= sample[2];
        }
    }
    color
}

fn sample_image(image: &gltf::image::Data, uv: [f32; 2]) -> [f32; 3] {
    if image.width == 0 || image.height == 0 {
        return [1.0; 3];
    }
    let x = ((uv[0].clamp(0.0, 0.999_999) * image.width as f32) as u32).min(image.width - 1);
    let y = ((uv[1].clamp(0.0, 0.999_999) * image.height as f32) as u32).min(image.height - 1);
    let pixel = (y * image.width + x) as usize;
    let rgb = match image.format {
        Format::R8G8B8A8 => {
            let index = pixel * 4;
            [
                image.pixels[index],
                image.pixels[index + 1],
                image.pixels[index + 2],
            ]
        }
        Format::R8G8B8 => {
            let index = pixel * 3;
            [
                image.pixels[index],
                image.pixels[index + 1],
                image.pixels[index + 2],
            ]
        }
        Format::R8G8 => {
            let index = pixel * 2;
            [image.pixels[index], image.pixels[index + 1], 0]
        }
        Format::R8 => [image.pixels[pixel]; 3],
        _ => [255; 3],
    };
    [
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
    ]
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn compute_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks(3) {
        let a = Vec3::from_array(positions[triangle[0] as usize]);
        let b = Vec3::from_array(positions[triangle[1] as usize]);
        let c = Vec3::from_array(positions[triangle[2] as usize]);
        let normal = (b - a).cross(c - a);
        for index in triangle {
            normals[*index as usize] += normal;
        }
    }
    normals
        .into_iter()
        .map(|normal| normal.normalize_or_zero().to_array())
        .collect()
}

fn physics_from_visual(wheels: &[WheelVisual; 4]) -> Result<CarPhysics> {
    let front_z = (wheels[0].transform.translation[2] + wheels[1].transform.translation[2]) * 0.5;
    let rear_z = (wheels[2].transform.translation[2] + wheels[3].transform.translation[2]) * 0.5;
    let wheelbase = (rear_z - front_z).abs();
    let track_width = [
        (wheels[0].transform.translation[0] - wheels[1].transform.translation[0]).abs(),
        (wheels[2].transform.translation[0] - wheels[3].transform.translation[0]).abs(),
    ];
    if wheelbase <= 0.1 {
        bail!("imported wheelbase is invalid: {wheelbase}");
    }

    let mut physics_wheels = Vec::with_capacity(4);
    for (index, wheel) in wheels.iter().enumerate() {
        let label = ["FL", "FR", "RL", "RR"][index];
        physics_wheels.push(WheelPhysics {
            name: label.to_string(),
            is_front: wheel.is_front,
            is_left: wheel.is_left,
            position: wheel.transform.translation,
            radius: wheel.radius,
            width: wheel.width,
            steerable: wheel.is_front,
            driven: !wheel.is_front,
            suspension: SuspensionSpec {
                travel: 0.18,
                spring_rate: 200_000.0,
                damper_bump: 12_000.0,
                damper_rebound: 15_000.0,
                anti_roll: 8_000.0,
                ride_height: wheel.transform.translation[1],
            },
            tire: default_tire(),
        });
    }

    Ok(CarPhysics {
        mass: 1_250.0,
        inertia: [500.0, 650.0, 350.0],
        center_of_gravity: [0.0, 0.35, 0.0],
        wheelbase,
        track_width,
        wheels: physics_wheels
            .try_into()
            .map_err(|_| anyhow::anyhow!("imported car did not produce four physics wheels"))?,
        engine: EngineSpec {
            idle_rpm: 900.0,
            redline_rpm: 7_500.0,
            torque_curve: vec![
                TorquePoint {
                    rpm: 1_000.0,
                    torque_nm: 190.0,
                },
                TorquePoint {
                    rpm: 2_000.0,
                    torque_nm: 270.0,
                },
                TorquePoint {
                    rpm: 3_500.0,
                    torque_nm: 330.0,
                },
                TorquePoint {
                    rpm: 5_000.0,
                    torque_nm: 310.0,
                },
                TorquePoint {
                    rpm: 6_500.0,
                    torque_nm: 270.0,
                },
            ],
        },
        drivetrain: sim_types::DrivetrainSpec {
            gear_ratios: vec![3.2, 2.1, 1.5, 1.2, 1.0, 0.8],
            final_drive: 3.7,
            clutch_max_torque: 400.0,
            drive_layout: DriveLayout::Rear,
        },
        brakes: BrakeSpec {
            max_torque: [1_800.0, 1_800.0, 1_200.0, 1_200.0],
            bias: 0.6,
        },
        steering: SteeringSpec {
            max_angle: 0.6,
            ratio: 14.0,
            caster: 0.09,
            kingpin_inclination: 0.12,
        },
        aero: AeroSpec {
            drag_coefficient: 0.32,
            frontal_area: 2.0,
            downforce_coefficient: 0.0,
        },
    })
}

fn default_tire() -> TireSpec {
    TireSpec {
        lateral: PacejkaCoefficients {
            b: 10.0,
            c: 1.9,
            d: 1.0,
            e: 0.97,
        },
        longitudinal: PacejkaCoefficients {
            b: 12.0,
            c: 1.65,
            d: 1.0,
            e: 0.9,
        },
        relaxation_length: 0.1,
        peak_slip_ratio: 0.12,
        peak_slip_angle: 0.12,
        friction_scale: 1.0,
    }
}
