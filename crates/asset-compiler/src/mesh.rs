use anyhow::{bail, Result};
use sim_types::{Aabb, MeshBuffers};

pub fn build_box(size: [f32; 3], position: [f32; 3]) -> MeshBuffers {
    let half = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];
    let min = [
        position[0] - half[0],
        position[1] - half[1],
        position[2] - half[2],
    ];
    let max = [
        position[0] + half[0],
        position[1] + half[1],
        position[2] + half[2],
    ];

    let mut mesh = MeshBuffers {
        positions: Vec::with_capacity(24),
        normals: Vec::with_capacity(24),
        uvs: Vec::with_capacity(24),
        indices: Vec::with_capacity(36),
    };

    add_quad(
        &mut mesh,
        [
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
            [max[0], min[1], max[2]],
        ],
        [1.0, 0.0, 0.0],
    );
    add_quad(
        &mut mesh,
        [
            [min[0], min[1], max[2]],
            [min[0], max[1], max[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        [-1.0, 0.0, 0.0],
    );
    add_quad(
        &mut mesh,
        [
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], max[1], min[2]],
        ],
        [0.0, 1.0, 0.0],
    );
    add_quad(
        &mut mesh,
        [
            [min[0], min[1], max[2]],
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], min[1], max[2]],
        ],
        [0.0, -1.0, 0.0],
    );
    add_quad(
        &mut mesh,
        [
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ],
        [0.0, 0.0, 1.0],
    );
    add_quad(
        &mut mesh,
        [
            [max[0], min[1], min[2]],
            [min[0], min[1], min[2]],
            [min[0], max[1], min[2]],
            [max[0], max[1], min[2]],
        ],
        [0.0, 0.0, -1.0],
    );

    mesh
}

pub fn build_plane(size: [f32; 2], position: [f32; 3]) -> MeshBuffers {
    let half_x = size[0] * 0.5;
    let half_z = size[1] * 0.5;
    let y = position[1];
    let mut mesh = MeshBuffers {
        positions: Vec::with_capacity(4),
        normals: Vec::with_capacity(4),
        uvs: Vec::with_capacity(4),
        indices: vec![0, 1, 2, 0, 2, 3],
    };

    let points = [
        [position[0] - half_x, y, position[2] - half_z],
        [position[0] - half_x, y, position[2] + half_z],
        [position[0] + half_x, y, position[2] + half_z],
        [position[0] + half_x, y, position[2] - half_z],
    ];
    for point in points {
        mesh.positions.push(point);
        mesh.normals.push([0.0, 1.0, 0.0]);
    }
    mesh.uvs
        .extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
    mesh
}

pub fn build_cylinder(
    radius: f32,
    width: f32,
    axis: &str,
    segments: u32,
    position: [f32; 3],
) -> Result<MeshBuffers> {
    if axis != "x" {
        bail!("cylinder axis {axis} is not supported yet; use x");
    }
    if segments < 3 {
        bail!("cylinder segments must be at least 3");
    }
    if radius <= 0.0 || width <= 0.0 {
        bail!("cylinder radius and width must be positive");
    }

    let segments = segments as usize;
    let half_width = width * 0.5;
    let mut mesh = MeshBuffers {
        positions: Vec::with_capacity(segments * 4 + 2),
        normals: Vec::with_capacity(segments * 4 + 2),
        uvs: Vec::with_capacity(segments * 4 + 2),
        indices: Vec::with_capacity(segments * 12),
    };

    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * core::f32::consts::TAU;
        let y = angle.cos();
        let z = angle.sin();
        let normal = [0.0, y, z];
        let u = index as f32 / segments as f32;

        mesh.positions.push([
            position[0] - half_width,
            position[1] + y * radius,
            position[2] + z * radius,
        ]);
        mesh.normals.push(normal);
        mesh.uvs.push([u, 0.0]);

        mesh.positions.push([
            position[0] + half_width,
            position[1] + y * radius,
            position[2] + z * radius,
        ]);
        mesh.normals.push(normal);
        mesh.uvs.push([u, 1.0]);
    }

    for index in 0..segments {
        let base = (index * 2) as u32;
        let next = base + 2;
        mesh.indices
            .extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
    }

    let left_center = mesh.positions.len() as u32;
    mesh.positions
        .push([position[0] - half_width, position[1], position[2]]);
    mesh.normals.push([-1.0, 0.0, 0.0]);
    mesh.uvs.push([0.5, 0.5]);

    let left_ring = mesh.positions.len() as u32;
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * core::f32::consts::TAU;
        let y = angle.cos();
        let z = angle.sin();
        mesh.positions.push([
            position[0] - half_width,
            position[1] + y * radius,
            position[2] + z * radius,
        ]);
        mesh.normals.push([-1.0, 0.0, 0.0]);
        mesh.uvs.push([0.5 + 0.5 * y, 0.5 + 0.5 * z]);
    }
    for index in 0..segments {
        let index = index as u32;
        mesh.indices
            .extend_from_slice(&[left_center, left_ring + index, left_ring + index + 1]);
    }

    let right_center = mesh.positions.len() as u32;
    mesh.positions
        .push([position[0] + half_width, position[1], position[2]]);
    mesh.normals.push([1.0, 0.0, 0.0]);
    mesh.uvs.push([0.5, 0.5]);

    let right_ring = mesh.positions.len() as u32;
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * core::f32::consts::TAU;
        let y = angle.cos();
        let z = angle.sin();
        mesh.positions.push([
            position[0] + half_width,
            position[1] + y * radius,
            position[2] + z * radius,
        ]);
        mesh.normals.push([1.0, 0.0, 0.0]);
        mesh.uvs.push([0.5 + 0.5 * y, 0.5 + 0.5 * z]);
    }
    for index in 0..segments {
        let index = index as u32;
        mesh.indices
            .extend_from_slice(&[right_center, right_ring + index + 1, right_ring + index]);
    }

    Ok(mesh)
}

pub fn bounds(mesh: &MeshBuffers) -> Aabb {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &mesh.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    if !min[0].is_finite() {
        return Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        };
    }
    Aabb { min, max }
}

pub fn merge_bounds(current: Option<Aabb>, next: &Aabb) -> Aabb {
    match current {
        None => *next,
        Some(current) => Aabb {
            min: [
                current.min[0].min(next.min[0]),
                current.min[1].min(next.min[1]),
                current.min[2].min(next.min[2]),
            ],
            max: [
                current.max[0].max(next.max[0]),
                current.max[1].max(next.max[1]),
                current.max[2].max(next.max[2]),
            ],
        },
    }
}

fn add_quad(mesh: &mut MeshBuffers, points: [[f32; 3]; 4], normal: [f32; 3]) {
    let base = mesh.positions.len() as u32;
    mesh.positions.extend_from_slice(&points);
    mesh.normals.extend_from_slice(&[normal; 4]);
    mesh.uvs
        .extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
