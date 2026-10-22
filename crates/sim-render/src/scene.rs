use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use sim_types::{AssetManifest, MeshBuffers, TrackCollision, TrackMetadata, TrackVisual};

use crate::camera::Camera;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub struct MeshData {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub model: Mat4,
}

#[derive(Debug, Clone)]
pub struct SceneWheel {
    pub mesh_index: usize,
    pub base_model: Mat4,
    pub is_front: bool,
}

#[derive(Debug, Clone)]
pub struct RigWheel {
    pub mesh_index: usize,
    pub local_model: Mat4,
    pub is_front: bool,
}

#[derive(Debug, Clone)]
pub struct CarRig {
    pub body_mesh_index: usize,
    pub body_local: Mat4,
    pub wheels: Vec<RigWheel>,
    pub steering_mesh_index: Option<usize>,
    pub steering_local: Mat4,
}

pub struct Scene {
    pub meshes: Vec<MeshData>,
    pub lines: Vec<LineVertex>,
    pub camera: Camera,
    pub debug_info: String,
    pub wheels: Vec<SceneWheel>,
    pub stats: SceneStats,
    pub car_rig: Option<CarRig>,
}

#[derive(Debug, Clone, Copy)]
pub struct SceneStats {
    pub vertices: u32,
    pub triangles: u32,
    pub draw_calls: u32,
    pub textures: u32,
    pub collision_triangles: u32,
}

impl Scene {
    pub fn build(
        visual: &TrackVisual,
        metadata: &TrackMetadata,
        collision: &TrackCollision,
        manifest: &AssetManifest,
    ) -> Result<Scene, String> {
        if visual.meshes.len() != visual.mesh_buffers.len() {
            return Err(format!(
                "track visual has {} mesh refs but {} mesh buffers",
                visual.meshes.len(),
                visual.mesh_buffers.len()
            ));
        }

        let mut meshes = Vec::with_capacity(visual.meshes.len());
        let mut stats = SceneStats {
            vertices: 0,
            triangles: 0,
            draw_calls: 0,
            textures: visual.textures.len() as u32,
            collision_triangles: (collision.indices.len() / 3) as u32,
        };

        for (mesh_ref, buffers) in visual.meshes.iter().zip(visual.mesh_buffers.iter()) {
            if buffers.positions.is_empty() || buffers.indices.is_empty() {
                continue;
            }
            let color = material_color(visual, mesh_ref.material as usize);
            let vertices = build_vertices(buffers, color);
            for index in &buffers.indices {
                if *index as usize >= vertices.len() {
                    return Err(format!(
                        "mesh {} index {} is outside {} vertices",
                        mesh_ref.name,
                        index,
                        vertices.len()
                    ));
                }
            }
            stats.vertices += vertices.len() as u32;
            stats.triangles += (buffers.indices.len() / 3) as u32;
            stats.draw_calls += 1;
            meshes.push(MeshData {
                name: mesh_ref.name.clone(),
                vertices,
                indices: buffers.indices.clone(),
                model: Mat4::IDENTITY,
            });
        }

        let mut lines = Vec::new();
        add_axes(&mut lines);
        add_bounds(&mut lines, manifest);
        add_spawn(&mut lines, metadata);
        add_collision(&mut lines, collision);

        let camera = build_camera(metadata);
        let debug_info = json!({
            "asset": { "id": manifest.id, "kind": "Track" },
            "stats": {
                "vertices": stats.vertices,
                "triangles": stats.triangles,
                "drawCalls": stats.draw_calls,
                "textures": stats.textures,
                "collisionTriangles": stats.collision_triangles,
            },
            "bounds": { "min": manifest.bounds.min, "max": manifest.bounds.max },
            "surfaces": metadata.surfaces.iter().map(|surface| json!({
                "id": surface.id,
                "name": surface.name,
                "friction": surface.friction,
                "roughness": surface.roughness,
            })).collect::<Vec<_>>(),
            "spawns": metadata.spawns.iter().map(|spawn| json!({
                "name": spawn.name,
                "position": spawn.position,
                "rotation": spawn.rotation_xyzw,
            })).collect::<Vec<_>>(),
            "cameras": metadata.cameras.iter().map(|camera| json!({
                "name": camera.name,
                "position": camera.position,
                "target": camera.target,
                "fovY": camera.fov_y,
            })).collect::<Vec<_>>(),
            "meshes": visual.meshes.iter().map(|mesh| json!({
                "name": mesh.name,
                "vertices": mesh.vertex_count,
                "triangles": mesh.index_count / 3,
                "material": mesh.material,
            })).collect::<Vec<_>>(),
            "warnings": manifest.warnings,
        })
        .to_string();

        Ok(Scene {
            meshes,
            lines,
            camera,
            debug_info,
            wheels: Vec::new(),
            stats,
            car_rig: None,
        })
    }
}

pub(crate) fn build_vertices(buffers: &MeshBuffers, color: [f32; 3]) -> Vec<MeshVertex> {
    buffers
        .positions
        .iter()
        .enumerate()
        .map(|(index, position)| MeshVertex {
            position: *position,
            normal: buffers
                .normals
                .get(index)
                .copied()
                .unwrap_or([0.0, 1.0, 0.0]),
            color,
        })
        .collect()
}

pub(crate) fn material_color(visual: &TrackVisual, index: usize) -> [f32; 3] {
    visual
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

fn build_camera(metadata: &TrackMetadata) -> Camera {
    if let Some(camera) = metadata
        .cameras
        .iter()
        .find(|camera| camera.name == "start")
        .or_else(|| metadata.cameras.first())
    {
        return Camera {
            eye: camera.position,
            target: camera.target,
            fov_y: camera.fov_y,
        };
    }

    let spawn = metadata
        .spawns
        .first()
        .map(|spawn| spawn.position)
        .unwrap_or([0.0; 3]);
    Camera {
        eye: [spawn[0], spawn[1] + 6.0, spawn[2] + 14.0],
        target: [spawn[0], spawn[1] + 0.5, spawn[2] - 10.0],
        fov_y: 0.9,
    }
}

fn add_axes(lines: &mut Vec<LineVertex>) {
    let origin = [0.0, 0.05, 0.0];
    push_line(lines, origin, [10.0, 0.05, 0.0], [0.9, 0.15, 0.15]);
    push_line(lines, origin, [0.0, 10.0, 0.0], [0.15, 0.9, 0.15]);
    push_line(lines, origin, [0.0, 0.05, -10.0], [0.15, 0.35, 1.0]);
}

fn add_bounds(lines: &mut Vec<LineVertex>, manifest: &AssetManifest) {
    let min = manifest.bounds.min;
    let max = manifest.bounds.max;
    let color = [0.55, 0.6, 0.7];
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
    let edges = [
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
    ];
    for (from, to) in edges {
        push_line(lines, corners[from], corners[to], color);
    }
}

fn add_spawn(lines: &mut Vec<LineVertex>, metadata: &TrackMetadata) {
    let Some(spawn) = metadata.spawns.first() else {
        return;
    };
    let origin = [
        spawn.position[0],
        spawn.position[1] + 0.05,
        spawn.position[2],
    ];
    let color = [1.0, 0.85, 0.2];
    push_line(
        lines,
        [origin[0] - 2.0, origin[1], origin[2]],
        [origin[0] + 2.0, origin[1], origin[2]],
        color,
    );
    push_line(
        lines,
        [origin[0], origin[1], origin[2] - 2.0],
        [origin[0], origin[1], origin[2] + 2.0],
        color,
    );
    push_line(
        lines,
        [origin[0], origin[1], origin[2]],
        [origin[0], origin[1] + 2.0, origin[2]],
        color,
    );

    let rotation = Quat::from_xyzw(
        spawn.rotation_xyzw[0],
        spawn.rotation_xyzw[1],
        spawn.rotation_xyzw[2],
        spawn.rotation_xyzw[3],
    )
    .normalize();
    let forward = rotation * Vec3::new(0.0, 0.0, -5.0);
    push_line(
        lines,
        origin,
        [
            origin[0] + forward.x,
            origin[1] + forward.y,
            origin[2] + forward.z,
        ],
        color,
    );
}

fn add_collision(lines: &mut Vec<LineVertex>, collision: &TrackCollision) {
    let triangle_count = collision.indices.len() / 3;
    for triangle in 0..triangle_count {
        let surface = collision.surface_ids.get(triangle).copied().unwrap_or(0);
        let color = surface_color(surface);
        let base = triangle * 3;
        let points = [
            collision_point(collision, collision.indices[base]),
            collision_point(collision, collision.indices[base + 1]),
            collision_point(collision, collision.indices[base + 2]),
        ];
        push_line(lines, points[0], points[1], color);
        push_line(lines, points[1], points[2], color);
        push_line(lines, points[2], points[0], color);
    }
}

fn collision_point(collision: &TrackCollision, index: u32) -> [f32; 3] {
    let point = collision
        .vertices
        .get(index as usize)
        .copied()
        .unwrap_or([0.0; 3]);
    [point[0], point[1] + 0.02, point[2]]
}

fn surface_color(surface: u16) -> [f32; 3] {
    match surface {
        0 => [0.2, 0.65, 1.0],
        1 => [1.0, 0.3, 0.2],
        2 => [1.0, 0.9, 0.2],
        3 => [0.8, 0.3, 1.0],
        4 => [0.2, 1.0, 0.7],
        _ => [0.9, 0.9, 0.9],
    }
}

pub(crate) fn push_line(
    lines: &mut Vec<LineVertex>,
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 3],
) {
    lines.push(LineVertex {
        position: from,
        color,
    });
    lines.push(LineVertex {
        position: to,
        color,
    });
}
