use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use sim_types::{
    Aabb, AssetKind, AssetManifest, AssetStats, CameraSpec, EntryPoints, MaterialRef, MeshBuffers,
    MeshRef, SectorSpec, SpawnPoint, SurfaceSpec, TrackCollision, TrackMetadata, TrackVisual,
    ENGINE_FRAME, ENGINE_UNITS, MAGIC, SCHEMA_VERSION,
};

const SEGMENTS: usize = 120;
const TRACK_SURFACE: u16 = 0;
const GRASS_SURFACE: u16 = 1;
const DIRT_SURFACE: u16 = 2;
const KERB_SURFACE: u16 = 3;
const WALL_SURFACE: u16 = 4;
const TERRAIN_SIZE: f32 = 300.0;
const TERRAIN_CELLS: usize = 128;
const KERB_INNER_RADIUS: f32 = 6.0;
const KERB_OUTER_RADIUS: f32 = 6.8;
const KERB_BOTTOM_OFFSET: f32 = 0.02;
const KERB_SURFACE_OFFSET: f32 = 0.06;
const KERB_TOP_OFFSET: f32 = 0.155;
const OUTER_EDGE_RADIUS: f32 = 12.0;
const ASPHALT_HALF_WIDTH: f32 = 6.0;
const SURFACE_HEIGHT_OFFSET: f32 = 0.06;
const PROP_OFFSET: f32 = 14.0;
const BLOCK_HALF_WIDTH: f32 = 0.3;
const BLOCK_HALF_HEIGHT: f32 = 0.4;
const BLOCK_HALF_LENGTH: f32 = 0.9;
const TYRE_RADIUS: f32 = 0.4;
const TYRE_LAYER_HEIGHT: f32 = 0.3;
const TYRE_LAYERS: usize = 3;
const MIN_TURN_RADIUS: f32 = OUTER_EDGE_RADIUS.max(PROP_OFFSET) + 1.0;
const CURVATURE_RELAXATION: f32 = 0.5;
const CURVATURE_RELAXATION_ITERATIONS: usize = 1024;
const BLOCK_SPACING: f32 = 2.0;
const TYRE_SPACING: f32 = 0.8;
const PROP_LATERAL_JITTER: f32 = 0.03;
const PROP_LONGITUDINAL_JITTER: f32 = 0.03;
const PROP_YAW_JITTER: f32 = 0.025;
const CONTROL_POINTS: [[f32; 3]; 8] = [
    [-30.0, 0.0, -20.0],
    [0.0, 0.0, -35.0],
    [30.0, 0.0, -25.0],
    [40.0, 0.0, 0.0],
    [25.0, 0.0, 25.0],
    [0.0, 0.0, 15.0],
    [-25.0, 0.0, 30.0],
    [-40.0, 0.0, 5.0],
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct KerbSpan {
    pub start: f32,
    pub end: f32,
    pub side: i8,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    Blocks,
    Tyres,
    Off,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct PropLine {
    pub start: f32,
    pub end: f32,
    pub side: i8,
    pub kind: PropKind,
}

pub struct EditorTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub manifest: AssetManifest,
    pub kerb_spans: Vec<KerbSpan>,
    pub prop_lines: Vec<PropLine>,
}

pub fn default_control_points() -> Vec<[f32; 3]> {
    CONTROL_POINTS
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[2]), point[2]])
        .collect()
}

pub fn control_points_with_heights(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    points
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[2]), point[2]])
        .collect()
}

fn default_prop_lines(points: &[Vec3]) -> Vec<PropLine> {
    let area = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.z - next.x * point.z
        })
        .sum::<f32>();
    let outside_side = if area > 0.0 { -1 } else { 1 };
    let end = points.len() as f32;
    vec![
        PropLine {
            start: 0.0,
            end,
            side: outside_side,
            kind: PropKind::Blocks,
        },
        PropLine {
            start: 0.0,
            end,
            side: -outside_side,
            kind: PropKind::Tyres,
        },
    ]
}

fn sanitize_prop_lines(lines: &[PropLine], count: usize) -> Vec<PropLine> {
    if count < 2 {
        return Vec::new();
    }
    let limit = count as f32;
    let mut lines = lines
        .iter()
        .filter_map(|line| {
            let start = line.start.clamp(0.0, limit);
            let end = line.end.clamp(0.0, limit);
            (end - start > 1e-4).then_some(PropLine {
                start,
                end,
                side: if line.side >= 0 { 1 } else { -1 },
                kind: line.kind,
            })
        })
        .collect::<Vec<_>>();
    lines.sort_by(|a, b| {
        a.side
            .cmp(&b.side)
            .then_with(|| a.start.total_cmp(&b.start))
    });

    let mut last_end = [0.0f32; 2];
    let mut sanitized = Vec::with_capacity(lines.len());
    for line in lines {
        let side_index = usize::from(line.side > 0);
        let start = line.start.max(last_end[side_index]);
        if start < line.end {
            last_end[side_index] = line.end;
            sanitized.push(PropLine { start, ..line });
        }
    }
    sanitized
}

pub fn centerline_points(control_points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    sample_track(control_points)
        .iter()
        .map(|point| point.to_array())
        .collect()
}

fn auto_kerb_spans(points: &[Vec3]) -> Vec<KerbSpan> {
    let count = points.len();
    let mut spans = Vec::new();
    let mut current: Option<KerbSpan> = None;
    for index in 0..count {
        let point = points[index];
        let next = points[(index + 1) % count];
        let future = points[(index + 2) % count];
        let tangent = (next - point).normalize_or_zero();
        let future_tangent = (future - next).normalize_or_zero();
        let curvature_y = tangent.cross(future_tangent).y;
        if curvature_y.abs() <= 0.005 {
            if let Some(span) = current.take() {
                spans.push(span);
            }
            continue;
        }

        let side = if curvature_y > 0.0 { 1 } else { -1 };
        let start = index as f32;
        let end = start + 1.0;
        match current.as_mut() {
            Some(span) if span.side == side => span.end = end,
            Some(_) => {
                spans.push(current.take().expect("kerb span exists"));
                current = Some(KerbSpan { start, end, side });
            }
            None => {
                current = Some(KerbSpan { start, end, side });
            }
        }
    }
    if let Some(span) = current {
        spans.push(span);
    }
    spans
}

fn sanitize_kerb_spans(spans: &[KerbSpan], count: usize) -> Vec<KerbSpan> {
    if count < 2 {
        return Vec::new();
    }
    let limit = count as f32;
    let mut spans = spans
        .iter()
        .filter_map(|span| {
            let start = span.start.clamp(0.0, limit);
            let end = span.end.clamp(0.0, limit);
            (end - start > 1e-4).then_some(KerbSpan {
                start,
                end,
                side: if span.side >= 0 { 1 } else { -1 },
            })
        })
        .collect::<Vec<_>>();
    spans.sort_by(|a, b| {
        a.side
            .cmp(&b.side)
            .then_with(|| a.start.total_cmp(&b.start))
    });

    let mut last_end = [0.0f32; 2];
    let mut sanitized: Vec<KerbSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        let side_index = usize::from(span.side > 0);
        let start = span.start.max(last_end[side_index]);
        if start < span.end {
            last_end[side_index] = span.end;
            sanitized.push(KerbSpan { start, ..span });
        }
    }
    sanitized
}

pub fn track_from_control_points(control_points: &[[f32; 3]]) -> Result<EditorTrack, String> {
    track_from_control_points_with_spans(control_points, None, None)
}

pub fn track_from_control_points_with_spans(
    control_points: &[[f32; 3]],
    kerb_spans: Option<&[KerbSpan]>,
    prop_lines: Option<&[PropLine]>,
) -> Result<EditorTrack, String> {
    if control_points.len() < 3 {
        return Err("procedural track needs at least three control points".to_string());
    }
    let points = sample_track(control_points);
    if points.len() < 3 {
        return Err("procedural track needs at least three samples".to_string());
    }
    let kerb_spans = kerb_spans
        .map(|spans| sanitize_kerb_spans(spans, points.len()))
        .unwrap_or_else(|| auto_kerb_spans(&points));
    let prop_lines = prop_lines
        .map(|lines| sanitize_prop_lines(lines, points.len()))
        .unwrap_or_else(|| default_prop_lines(&points));

    let mut visual = TrackVisual {
        meshes: Vec::new(),
        materials: materials(),
        textures: Vec::new(),
        mesh_buffers: Vec::new(),
        instances: Vec::new(),
    };

    let mut collision = TrackCollision {
        vertices: Vec::new(),
        indices: Vec::new(),
        surface_ids: Vec::new(),
        bounds: Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        },
    };
    let TrackRibbon {
        buffers: track_buffers,
        surface_ids: track_surface_ids,
    } = track_ribbon(&points);
    append_collision_surfaces(&mut collision, &track_buffers, |index| {
        track_surface_ids
            .get(index)
            .copied()
            .unwrap_or(TRACK_SURFACE)
    });
    let terrain = terrain_grid(&points);
    push_mesh(&mut visual, "asphalt_dither", 1, track_buffers, false);
    push_mesh(&mut visual, "terrain", 2, terrain, false);

    let mut kerb_red = empty_buffers();
    let mut kerb_white = empty_buffers();
    let mut barriers = empty_buffers();
    let mut tires_blue = empty_buffers();
    let mut tires_white = empty_buffers();
    let mut prop_walls = empty_buffers();

    let count = points.len();
    for span in &kerb_spans {
        let start = span.start.clamp(0.0, count as f32);
        let end = span.end.clamp(0.0, count as f32);
        if end - start <= 1e-4 {
            continue;
        }
        let mut previous_end = None;
        for index in start.floor() as usize..end.ceil() as usize {
            let segment_start = (index as f32).max(start);
            let segment_end = ((index + 1) as f32).min(end);
            if segment_end - segment_start <= 1e-4 {
                continue;
            }
            let (point, binormal) = track_frame(&points, segment_start);
            let (next, next_binormal) = track_frame(&points, segment_end);
            let kerb = if index % 2 == 0 {
                &mut kerb_red
            } else {
                &mut kerb_white
            };
            let (start_edge, end_edge) =
                append_kerb(kerb, point, next, binormal, next_binormal, span.side as f32);
            if let Some(previous_end) = previous_end {
                append_kerb_riser(kerb, previous_end, start_edge, next - point);
            }
            previous_end = Some(end_edge);
        }
    }

    append_prop_lines(
        &points,
        &prop_lines,
        &mut barriers,
        &mut tires_blue,
        &mut tires_white,
    );
    append_prop_walls(&points, &prop_lines, &mut prop_walls);

    push_mesh(&mut visual, "kerb_red", 3, kerb_red, false);
    push_mesh(&mut visual, "kerb_white", 4, kerb_white, false);
    push_mesh(&mut visual, "barriers", 5, barriers, true);
    push_mesh(&mut visual, "tires_blue", 6, tires_blue, true);
    push_mesh(&mut visual, "tires_white", 7, tires_white, true);

    for (index, mesh) in visual.meshes.iter().enumerate() {
        let surface = match mesh.name.as_str() {
            "terrain" => GRASS_SURFACE,
            "kerb_red" | "kerb_white" => KERB_SURFACE,
            _ => continue,
        };
        append_collision(&mut collision, &visual.mesh_buffers[index], surface);
    }
    append_collision(&mut collision, &prop_walls, WALL_SURFACE);
    collision.bounds = collision_bounds(&collision.vertices);

    let tangent = (points[1] - points[0]).normalize_or_zero();
    let rotation = if tangent.length_squared() > 1e-6 {
        Quat::from_rotation_y(PI) * Quat::from_rotation_arc(Vec3::NEG_Z, tangent)
    } else {
        Quat::IDENTITY
    };
    let spawn = SpawnPoint {
        name: "spawn".to_string(),
        position: points[0].to_array(),
        rotation_xyzw: rotation.to_array(),
    };
    let camera_y = terrain_height(0.0, 0.0);
    let camera = CameraSpec {
        name: "overview".to_string(),
        position: [0.0, camera_y + 45.0, 65.0],
        target: [0.0, camera_y, 0.0],
        fov_y: 45.0_f32.to_radians(),
    };
    let length = points
        .iter()
        .enumerate()
        .map(|(index, point)| point.distance(points[(index + 1) % points.len()]))
        .sum::<f32>();
    let metadata = TrackMetadata {
        name: "Procedural Drift Loop".to_string(),
        length,
        spawns: vec![spawn],
        pit_boxes: Vec::new(),
        cameras: vec![camera],
        sectors: vec![SectorSpec {
            name: "loop".to_string(),
            start_fraction: 0.0,
            end_fraction: 1.0,
        }],
        surfaces: vec![
            SurfaceSpec {
                id: TRACK_SURFACE,
                name: "asphalt".to_string(),
                friction: 0.95,
                roughness: 0.8,
            },
            SurfaceSpec {
                id: GRASS_SURFACE,
                name: "grass".to_string(),
                friction: 0.55,
                roughness: 1.0,
            },
            SurfaceSpec {
                id: DIRT_SURFACE,
                name: "dirt".to_string(),
                friction: 0.7,
                roughness: 1.0,
            },
            SurfaceSpec {
                id: KERB_SURFACE,
                name: "kerb".to_string(),
                friction: 0.9,
                roughness: 0.2,
            },
            SurfaceSpec {
                id: WALL_SURFACE,
                name: "wall".to_string(),
                friction: 0.8,
                roughness: 1.0,
            },
        ],
    };

    let bounds = visual_bounds(&visual)?;
    let vertices = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.positions.len() as u32)
        .sum();
    let triangles = visual
        .mesh_buffers
        .iter()
        .map(|buffers| (buffers.indices.len() / 3) as u32)
        .sum();
    let manifest = AssetManifest {
        magic: MAGIC,
        schema_version: SCHEMA_VERSION,
        kind: AssetKind::Track,
        id: "procedural-drift-loop".to_string(),
        source_hash: [0; 32],
        units: ENGINE_UNITS,
        frame: ENGINE_FRAME,
        bounds,
        chunks: Vec::new(),
        entrypoints: EntryPoints {
            physics: None,
            visual: None,
            collision: None,
            metadata: None,
            audio: None,
            texture: None,
        },
        stats: AssetStats {
            vertices,
            triangles,
            materials: visual.materials.len() as u16,
            textures: 0,
            draw_calls: visual.meshes.len() as u16,
        },
        warnings: vec!["procedural track editor preview".to_string()],
    };

    Ok(EditorTrack {
        visual,
        metadata,
        collision,
        manifest,
        kerb_spans,
        prop_lines,
    })
}

pub fn pack_from_track(track: &EditorTrack) -> Result<Vec<u8>, String> {
    let chunks = vec![
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Visual,
            bytes: sim_asset::archive(&track.visual).map_err(|error| error.to_string())?,
        },
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Collision,
            bytes: sim_asset::archive(&track.collision).map_err(|error| error.to_string())?,
        },
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Metadata,
            bytes: sim_asset::archive(&track.metadata).map_err(|error| error.to_string())?,
        },
    ];
    let mut manifest = track.manifest.clone();
    manifest.entrypoints = sim_types::EntryPoints {
        physics: None,
        visual: Some(0),
        collision: Some(1),
        metadata: Some(2),
        audio: None,
        texture: None,
    };
    sim_asset::write_pack(manifest, chunks).map_err(|error| error.to_string())
}

fn terrain_height(x: f32, z: f32) -> f32 {
    2.4 * (x * 0.025).sin() * (z * 0.03 + 0.4).cos() + 0.8 * ((x + z) * 0.04).sin()
}

fn terrain_normal(x: f32, z: f32) -> [f32; 3] {
    let epsilon = 0.5;
    let dx = (terrain_height(x + epsilon, z) - terrain_height(x - epsilon, z)) / (epsilon * 2.0);
    let dz = (terrain_height(x, z + epsilon) - terrain_height(x, z - epsilon)) / (epsilon * 2.0);
    Vec3::new(-dx, 1.0, -dz).normalize_or_zero().to_array()
}

fn materials() -> Vec<MaterialRef> {
    vec![
        material("asphalt", [0.045, 0.045, 0.05, 1.0], false),
        material("dither_shoulder", [0.48, 0.345, 0.208, 1.0], false),
        material("grass", [0.302, 0.431, 0.208, 1.0], false),
        material("kerb_red", [0.702, 0.141, 0.141, 1.0], false),
        material("kerb_white", [0.933, 0.933, 0.933, 1.0], false),
        material("concrete", [0.667, 0.667, 0.667, 1.0], true),
        material("tire_blue", [0.133, 0.333, 0.667, 1.0], true),
        material("tire_white", [0.867, 0.867, 0.867, 1.0], true),
    ]
}

fn material(name: &str, color: [f32; 4], outline: bool) -> MaterialRef {
    MaterialRef {
        name: name.to_string(),
        base_color: color,
        metallic: 0.0,
        roughness: 0.9,
        base_color_texture: None,
        outline,
    }
}

fn sample_track(control_points: &[[f32; 3]]) -> Vec<Vec3> {
    let controls = control_points
        .iter()
        .map(|point| Vec3::new(point[0], 0.0, point[2]))
        .collect::<Vec<_>>();
    let count = controls.len();
    let mut points = (0..SEGMENTS)
        .map(|index| {
            let global = index as f32 / SEGMENTS as f32 * count as f32;
            let segment = (global.floor() as usize) % count;
            let local = global - global.floor();
            catmull_rom(
                controls[(segment + count - 1) % count],
                controls[segment],
                controls[(segment + 1) % count],
                controls[(segment + 2) % count],
                local,
            )
        })
        .collect::<Vec<_>>();

    limit_centerline_curvature(&mut points);

    for point in &mut points {
        point.y = terrain_height(point.x, point.z);
    }
    points
}

fn limit_centerline_curvature(points: &mut [Vec3]) {
    let count = points.len();
    if count < 3 {
        return;
    }

    let mut radii = vec![0.0; count];
    let mut previous = points.to_vec();
    for _ in 0..CURVATURE_RELAXATION_ITERATIONS {
        let mut too_tight = false;
        for index in 0..count {
            radii[index] = curvature_radius(
                points[(index + count - 1) % count],
                points[index],
                points[(index + 1) % count],
            );
            too_tight |= radii[index] < MIN_TURN_RADIUS;
        }
        if !too_tight {
            return;
        }

        previous.copy_from_slice(points);
        for index in 0..count {
            if radii[index] >= MIN_TURN_RADIUS {
                continue;
            }
            let midpoint =
                (previous[(index + count - 1) % count] + previous[(index + 1) % count]) * 0.5;
            points[index] = previous[index].lerp(midpoint, CURVATURE_RELAXATION);
            points[index].y = 0.0;
        }
    }
}

fn curvature_radius(previous: Vec3, current: Vec3, next: Vec3) -> f32 {
    let incoming = current - previous;
    let outgoing = next - current;
    let turn = (incoming.x * outgoing.z - incoming.z * outgoing.x)
        .atan2(incoming.x * outgoing.x + incoming.z * outgoing.z)
        .abs();
    if turn <= 1e-5 {
        f32::INFINITY
    } else {
        ((incoming.length() + outgoing.length()) * 0.5) / turn
    }
}

fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let alpha = 0.5_f32;
    let d01 = p0.distance(p1).powf(alpha);
    let d12 = p1.distance(p2).powf(alpha);
    let d23 = p2.distance(p3).powf(alpha);
    let t0 = 0.0;
    let t1 = t0 + d01.max(1e-5);
    let t2 = t1 + d12.max(1e-5);
    let t3 = t2 + d23.max(1e-5);
    let current = t1 + (t2 - t1) * t.clamp(0.0, 1.0);
    let a1 = p0 * ((t1 - current) / (t1 - t0)) + p1 * ((current - t0) / (t1 - t0));
    let a2 = p1 * ((t2 - current) / (t2 - t1)) + p2 * ((current - t1) / (t2 - t1));
    let a3 = p2 * ((t3 - current) / (t3 - t2)) + p3 * ((current - t2) / (t3 - t2));
    let b1 = a1 * ((t2 - current) / (t2 - t0)) + a2 * ((current - t0) / (t2 - t0));
    let b2 = a2 * ((t3 - current) / (t3 - t1)) + a3 * ((current - t1) / (t3 - t1));
    b1 * ((t2 - current) / (t2 - t1)) + b2 * ((current - t1) / (t2 - t1))
}

struct TrackRibbon {
    buffers: MeshBuffers,
    surface_ids: Vec<u16>,
}

fn track_ribbon(points: &[Vec3]) -> TrackRibbon {
    let mut buffers = empty_buffers();
    let mut surface_ids = Vec::new();
    let count = points.len();
    let radii = [
        -OUTER_EDGE_RADIUS,
        -ASPHALT_HALF_WIDTH,
        ASPHALT_HALF_WIDTH,
        OUTER_EDGE_RADIUS,
    ];
    let surfaces = [DIRT_SURFACE, TRACK_SURFACE, DIRT_SURFACE];
    for index in 0..=count {
        let sample = index % count;
        let point = points[sample];
        let next = points[(sample + 1) % count];
        let tangent = (next - point).normalize_or_zero();
        let binormal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
        let u = index as f32 / count as f32;
        for radius in radii {
            let lateral = point + binormal * radius;
            let y = terrain_height(lateral.x, lateral.z) + SURFACE_HEIGHT_OFFSET;
            buffers.positions.push([lateral.x, y, lateral.z]);
            buffers.normals.push(terrain_normal(lateral.x, lateral.z));
            let v = (radius - radii[0]) / (radii[radii.len() - 1] - radii[0]);
            buffers.uvs.push([u, v]);
        }
        if index < count {
            for (band, surface) in surfaces.iter().enumerate() {
                let base = (index * radii.len() + band) as u32;
                let next_base = ((index + 1) * radii.len() + band) as u32;
                buffers.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    next_base,
                    base + 1,
                    next_base + 1,
                    next_base,
                ]);
                surface_ids.extend_from_slice(&[*surface, *surface]);
            }
        }
    }
    TrackRibbon {
        buffers,
        surface_ids,
    }
}

fn terrain_grid(points: &[Vec3]) -> MeshBuffers {
    let mut buffers = empty_buffers();
    let half = TERRAIN_SIZE * 0.5;
    let step = TERRAIN_SIZE / TERRAIN_CELLS as f32;
    for iz in 0..=TERRAIN_CELLS {
        for ix in 0..=TERRAIN_CELLS {
            let x = -half + ix as f32 * step;
            let z = -half + iz as f32 * step;
            buffers.positions.push([x, terrain_height(x, z), z]);
            buffers.normals.push(terrain_normal(x, z));
            buffers.uvs.push([
                ix as f32 / TERRAIN_CELLS as f32,
                iz as f32 / TERRAIN_CELLS as f32,
            ]);
        }
    }
    let row = TERRAIN_CELLS + 1;
    let hole_radius_squared = 9.5_f32 * 9.5;
    for iz in 0..TERRAIN_CELLS {
        for ix in 0..TERRAIN_CELLS {
            let cell_x = -half + (ix as f32 + 0.5) * step;
            let cell_z = -half + (iz as f32 + 0.5) * step;
            let under_track = points.iter().any(|point| {
                let dx = point.x - cell_x;
                let dz = point.z - cell_z;
                dx * dx + dz * dz < hole_radius_squared
            });
            if under_track {
                continue;
            }
            let base = (iz * row + ix) as u32;
            let next = base + row as u32;
            buffers
                .indices
                .extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
        }
    }
    buffers
}

fn binormal_at(points: &[Vec3], index: usize) -> Vec3 {
    let count = points.len();
    let sample = index % count;
    let incoming = (points[sample] - points[(sample + count - 1) % count]).normalize_or_zero();
    let outgoing = (points[(sample + 1) % count] - points[sample]).normalize_or_zero();
    let tangent = (incoming + outgoing).normalize_or_zero();
    Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero()
}

fn track_frame(points: &[Vec3], index: f32) -> (Vec3, Vec3) {
    let count = points.len();
    if count == 0 {
        return (Vec3::ZERO, Vec3::Z);
    }
    let wrapped = index.clamp(0.0, count as f32) % count as f32;
    let base = wrapped.floor() as usize;
    let t = wrapped - base as f32;
    let sample = base % count;
    let next = (sample + 1) % count;
    let point = points[sample].lerp(points[next], t);
    let binormal = binormal_at(points, sample).lerp(binormal_at(points, next), t);
    (point, binormal.normalize_or_zero())
}

fn offset_path_vertices(points: &[Vec3], start: f32, end: f32, offset: f32) -> Vec<Vec3> {
    let limit = points.len() as f32;
    let start = start.clamp(0.0, limit);
    let end = end.clamp(0.0, limit);
    if end - start <= 1e-4 {
        return Vec::new();
    }
    let mut steps = vec![start];
    let mut next = start.floor() + 1.0;
    while next < end - 1e-4 {
        steps.push(next);
        next += 1.0;
    }
    if (end - steps.last().copied().unwrap_or(start)).abs() > 1e-4 {
        steps.push(end);
    }
    steps
        .into_iter()
        .map(|index| {
            let (point, binormal) = track_frame(points, index);
            point + binormal * offset
        })
        .collect()
}

fn prop_placements(points: &[Vec3], line: &PropLine) -> Vec<(Vec3, Vec3)> {
    let spacing = match line.kind {
        PropKind::Blocks => BLOCK_SPACING,
        PropKind::Tyres => TYRE_SPACING,
        PropKind::Off => return Vec::new(),
    };
    let vertices =
        offset_path_vertices(points, line.start, line.end, PROP_OFFSET * line.side as f32);
    let mut segments = Vec::with_capacity(vertices.len().saturating_sub(1));
    let mut total = 0.0;
    for pair in vertices.windows(2) {
        let length = pair[0].distance(pair[1]);
        if length <= 1e-6 {
            continue;
        }
        segments.push((pair[0], pair[1], length, total));
        total += length;
    }
    if total <= 1e-4 {
        return Vec::new();
    }

    let mut placements = Vec::new();
    let mut distance = 0.0;
    let mut segment = 0;
    while distance < total - 1e-4 {
        while segment + 1 < segments.len() && distance > segments[segment].3 + segments[segment].2 {
            segment += 1;
        }
        let (start, end, length, segment_start) = segments[segment];
        let t = ((distance - segment_start) / length).clamp(0.0, 1.0);
        let position = start.lerp(end, t);
        let tangent = (end - start).normalize_or_zero();
        placements.push((position, tangent));
        distance += spacing;
    }
    placements
}

fn prop_noise(line: &PropLine, index: usize, salt: u32) -> f32 {
    let kind_salt = match line.kind {
        PropKind::Blocks => 0x9e37_79b9,
        PropKind::Tyres => 0x85eb_ca6b,
        PropKind::Off => 0,
    };
    let mut seed = line.start.to_bits().wrapping_mul(0x9e37_79b9)
        ^ line.end.to_bits().wrapping_mul(0x85eb_ca6b)
        ^ ((line.side as i32 as u32) << 8)
        ^ kind_salt
        ^ (index as u32).wrapping_mul(0xc2b2_ae35)
        ^ salt;
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x7feb_352d);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x846c_a68b);
    seed ^= seed >> 16;
    seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn jittered_prop_placement(
    position: Vec3,
    tangent: Vec3,
    line: &PropLine,
    index: usize,
) -> (Vec3, f32) {
    let forward = Vec3::new(tangent.x, 0.0, tangent.z).normalize_or_zero();
    let binormal = Vec3::new(-forward.z, 0.0, forward.x).normalize_or_zero();
    let position = position
        + binormal * (PROP_LATERAL_JITTER * prop_noise(line, index, 0x1))
        + forward * (PROP_LONGITUDINAL_JITTER * prop_noise(line, index, 0x2));
    let heading = forward.x.atan2(forward.z) + PROP_YAW_JITTER * prop_noise(line, index, 0x3);
    (position, heading)
}

fn append_prop_lines(
    points: &[Vec3],
    lines: &[PropLine],
    barriers: &mut MeshBuffers,
    tires_blue: &mut MeshBuffers,
    tires_white: &mut MeshBuffers,
) {
    for line in lines {
        for (index, (position, tangent)) in prop_placements(points, line).into_iter().enumerate() {
            let (position, heading) = jittered_prop_placement(position, tangent, line, index);
            match line.kind {
                PropKind::Blocks => append_box(
                    barriers,
                    position,
                    Vec3::new(BLOCK_HALF_WIDTH, BLOCK_HALF_HEIGHT, BLOCK_HALF_LENGTH),
                    heading,
                ),
                PropKind::Tyres => {
                    for layer in 0..TYRE_LAYERS {
                        let tire = if layer % 2 == 0 {
                            &mut *tires_blue
                        } else {
                            &mut *tires_white
                        };
                        append_cylinder(
                            tire,
                            position,
                            TYRE_RADIUS,
                            TYRE_LAYER_HEIGHT,
                            8,
                            layer as f32 * TYRE_LAYER_HEIGHT,
                        );
                    }
                }
                PropKind::Off => {}
            }
        }
    }
}

fn prop_wall_dimensions(kind: PropKind) -> Option<(f32, f32)> {
    match kind {
        PropKind::Blocks => Some((BLOCK_HALF_WIDTH, BLOCK_HALF_HEIGHT * 2.0)),
        PropKind::Tyres => Some((TYRE_RADIUS, TYRE_LAYER_HEIGHT * TYRE_LAYERS as f32)),
        PropKind::Off => None,
    }
}

fn prop_wall_vertical(point: Vec3, height: f32) -> (Vec3, Vec3) {
    let base = terrain_height(point.x, point.z);
    (
        Vec3::new(point.x, base, point.z),
        Vec3::new(point.x, base + height, point.z),
    )
}

fn push_wall_quad(buffers: &mut MeshBuffers, corners: [Vec3; 4], hint: Vec3) {
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let mut normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.dot(hint) < 0.0 {
        normal = -normal;
    }
    push_quad(
        buffers,
        corners.map(|corner| corner.to_array()),
        normal.to_array(),
    );
}

fn append_prop_walls(points: &[Vec3], lines: &[PropLine], walls: &mut MeshBuffers) {
    for line in lines {
        let Some((half_width, height)) = prop_wall_dimensions(line.kind) else {
            continue;
        };
        let side = line.side as f32;
        let inner = offset_path_vertices(
            points,
            line.start,
            line.end,
            (PROP_OFFSET - half_width) * side,
        );
        let outer = offset_path_vertices(
            points,
            line.start,
            line.end,
            (PROP_OFFSET + half_width) * side,
        );
        if inner.len() < 2 || inner.len() != outer.len() {
            continue;
        }
        for (inner_pair, outer_pair) in inner.windows(2).zip(outer.windows(2)) {
            let (inner_start_bottom, inner_start_top) = prop_wall_vertical(inner_pair[0], height);
            let (inner_end_bottom, inner_end_top) = prop_wall_vertical(inner_pair[1], height);
            let (outer_start_bottom, outer_start_top) = prop_wall_vertical(outer_pair[0], height);
            let (outer_end_bottom, outer_end_top) = prop_wall_vertical(outer_pair[1], height);
            let tangent = (inner_pair[1] - inner_pair[0]).normalize_or_zero();
            let binormal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_start_bottom,
                    inner_start_top,
                    inner_end_top,
                    inner_end_bottom,
                ],
                -binormal * side,
            );
            push_wall_quad(
                walls,
                [
                    outer_start_bottom,
                    outer_start_top,
                    outer_end_top,
                    outer_end_bottom,
                ],
                binormal * side,
            );
            push_wall_quad(
                walls,
                [
                    inner_start_top,
                    outer_start_top,
                    outer_end_top,
                    inner_end_top,
                ],
                Vec3::Y,
            );
        }

        let closed = line.start <= 0.0 && line.end >= points.len() as f32;
        if !closed {
            let (inner_start_bottom, inner_start_top) = prop_wall_vertical(inner[0], height);
            let (outer_start_bottom, outer_start_top) = prop_wall_vertical(outer[0], height);
            let start_tangent = (inner[1] - inner[0]).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_start_bottom,
                    outer_start_bottom,
                    outer_start_top,
                    inner_start_top,
                ],
                -start_tangent,
            );

            let (inner_end_bottom, inner_end_top) =
                prop_wall_vertical(inner[inner.len() - 1], height);
            let (outer_end_bottom, outer_end_top) =
                prop_wall_vertical(outer[outer.len() - 1], height);
            let end_tangent = (inner[inner.len() - 1] - inner[inner.len() - 2]).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_end_bottom,
                    outer_end_bottom,
                    outer_end_top,
                    inner_end_top,
                ],
                end_tangent,
            );
        }
    }
}

fn append_kerb(
    buffers: &mut MeshBuffers,
    start: Vec3,
    end: Vec3,
    start_binormal: Vec3,
    end_binormal: Vec3,
    side: f32,
) -> ([Vec3; 2], [Vec3; 2]) {
    let inner_start = start + start_binormal * (KERB_INNER_RADIUS * side);
    let inner_end = end + end_binormal * (KERB_INNER_RADIUS * side);
    let outer_start = start + start_binormal * (KERB_OUTER_RADIUS * side);
    let outer_end = end + end_binormal * (KERB_OUTER_RADIUS * side);
    let inner_base_start = terrain_height(inner_start.x, inner_start.z);
    let inner_base_end = terrain_height(inner_end.x, inner_end.z);
    let outer_base_start = terrain_height(outer_start.x, outer_start.z);
    let outer_base_end = terrain_height(outer_end.x, outer_end.z);
    let inner_top_start = Vec3::new(
        inner_start.x,
        inner_base_start + KERB_SURFACE_OFFSET,
        inner_start.z,
    );
    let inner_top_end = Vec3::new(
        inner_end.x,
        inner_base_end + KERB_SURFACE_OFFSET,
        inner_end.z,
    );
    let outer_top_start = Vec3::new(
        outer_start.x,
        outer_base_start + KERB_SURFACE_OFFSET,
        outer_start.z,
    );
    let outer_top_end = Vec3::new(outer_end.x, outer_base_end + KERB_TOP_OFFSET, outer_end.z);
    let outer_bottom_start = Vec3::new(
        outer_start.x,
        outer_base_start + KERB_BOTTOM_OFFSET,
        outer_start.z,
    );
    let outer_bottom_end = Vec3::new(
        outer_end.x,
        outer_base_end + KERB_BOTTOM_OFFSET,
        outer_end.z,
    );
    let side_normal = (start_binormal + end_binormal).normalize_or_zero() * side;
    let along = (end - start).normalize_or_zero();
    let across = (outer_top_start - inner_top_start).normalize_or_zero();
    let mut top_normal = across.cross(along).normalize_or_zero();
    if top_normal.y < 0.0 {
        top_normal = -top_normal;
    }
    push_quad(
        buffers,
        [
            inner_top_start.to_array(),
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            inner_top_end.to_array(),
        ],
        top_normal.to_array(),
    );
    push_quad(
        buffers,
        [
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            outer_bottom_end.to_array(),
            outer_bottom_start.to_array(),
        ],
        side_normal.to_array(),
    );
    (
        [inner_top_start, outer_top_start],
        [inner_top_end, outer_top_end],
    )
}

fn append_kerb_riser(
    buffers: &mut MeshBuffers,
    previous_end: [Vec3; 2],
    current_start: [Vec3; 2],
    along: Vec3,
) {
    let corners = [previous_end[0], previous_end[1], current_start[1]];
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let mut normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.dot(-along) < 0.0 {
        normal = -normal;
    }
    push_triangle(
        buffers,
        corners.map(|corner| corner.to_array()),
        normal.to_array(),
    );
}

fn append_box(buffers: &mut MeshBuffers, center: Vec3, half: Vec3, yaw: f32) {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let rotate = |local: Vec3| {
        let x = center.x + local.x * cos_yaw + local.z * sin_yaw;
        let z = center.z - local.x * sin_yaw + local.z * cos_yaw;
        let y = terrain_height(x, z) + local.y + half.y;
        Vec3::new(x, y, z)
    };
    let faces = [
        (
            Vec3::X,
            [
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(half.x, half.y, -half.z),
                Vec3::new(half.x, half.y, half.z),
            ],
        ),
        (
            Vec3::NEG_X,
            [
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(-half.x, -half.y, half.z),
                Vec3::new(-half.x, half.y, half.z),
                Vec3::new(-half.x, half.y, -half.z),
            ],
        ),
        (
            Vec3::Y,
            [
                Vec3::new(-half.x, half.y, half.z),
                Vec3::new(half.x, half.y, half.z),
                Vec3::new(half.x, half.y, -half.z),
                Vec3::new(-half.x, half.y, -half.z),
            ],
        ),
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(-half.x, -half.y, half.z),
            ],
        ),
        (
            Vec3::Z,
            [
                Vec3::new(-half.x, -half.y, half.z),
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(half.x, half.y, half.z),
                Vec3::new(-half.x, half.y, half.z),
            ],
        ),
        (
            Vec3::NEG_Z,
            [
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(-half.x, half.y, -half.z),
                Vec3::new(half.x, half.y, -half.z),
            ],
        ),
    ];
    for (normal, corners) in faces {
        let rotated_normal = if normal.y.abs() > 0.5 {
            Vec3::from_array(terrain_normal(center.x, center.z)) * normal.y.signum()
        } else {
            Vec3::new(
                normal.x * cos_yaw + normal.z * sin_yaw,
                normal.y,
                -normal.x * sin_yaw + normal.z * cos_yaw,
            )
        };
        push_quad(
            buffers,
            corners.map(|corner| rotate(corner).to_array()),
            rotated_normal.to_array(),
        );
    }
}

fn append_cylinder(
    buffers: &mut MeshBuffers,
    center: Vec3,
    radius: f32,
    height: f32,
    segments: usize,
    base_offset: f32,
) {
    let top_normal = Vec3::from_array(terrain_normal(center.x, center.z));
    let top_center = Vec3::new(
        center.x,
        terrain_height(center.x, center.z) + base_offset + height,
        center.z,
    );
    let bottom_center = Vec3::new(
        center.x,
        terrain_height(center.x, center.z) + base_offset,
        center.z,
    );
    for index in 0..segments {
        let angle0 = index as f32 / segments as f32 * TAU;
        let angle1 = (index + 1) as f32 / segments as f32 * TAU;
        let x0 = center.x + angle0.cos() * radius;
        let z0 = center.z + angle0.sin() * radius;
        let x1 = center.x + angle1.cos() * radius;
        let z1 = center.z + angle1.sin() * radius;
        let ground0 = terrain_height(x0, z0) + base_offset;
        let ground1 = terrain_height(x1, z1) + base_offset;
        let bottom0 = Vec3::new(x0, ground0, z0);
        let bottom1 = Vec3::new(x1, ground1, z1);
        let top0 = Vec3::new(x0, ground0 + height, z0);
        let top1 = Vec3::new(x1, ground1 + height, z1);
        let middle = (angle0 + angle1) * 0.5;
        push_quad(
            buffers,
            [
                bottom0.to_array(),
                top0.to_array(),
                top1.to_array(),
                bottom1.to_array(),
            ],
            [middle.cos(), 0.0, middle.sin()],
        );
        push_triangle(
            buffers,
            [top_center.to_array(), top1.to_array(), top0.to_array()],
            top_normal.to_array(),
        );
        push_triangle(
            buffers,
            [
                bottom_center.to_array(),
                bottom0.to_array(),
                bottom1.to_array(),
            ],
            (-top_normal).to_array(),
        );
    }
}

fn push_quad(buffers: &mut MeshBuffers, corners: [[f32; 3]; 4], normal: [f32; 3]) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_triangle(buffers: &mut MeshBuffers, corners: [[f32; 3]; 3], normal: [f32; 3]) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2]);
}

fn append_collision_surfaces(
    collision: &mut TrackCollision,
    buffers: &MeshBuffers,
    surface_at: impl Fn(usize) -> u16,
) {
    let base = collision.vertices.len() as u32;
    collision.vertices.extend_from_slice(&buffers.positions);
    for (triangle, [a, b, c]) in buffers.indices.as_chunks::<3>().0.iter().enumerate() {
        let a = base + *a;
        let b = base + *b;
        let c = base + *c;
        let pa = Vec3::from_array(collision.vertices[a as usize]);
        let pb = Vec3::from_array(collision.vertices[b as usize]);
        let pc = Vec3::from_array(collision.vertices[c as usize]);
        let normal = (pb - pa).cross(pc - pa);
        if normal.y < 0.0 {
            collision.indices.extend_from_slice(&[a, c, b]);
        } else {
            collision.indices.extend_from_slice(&[a, b, c]);
        }
        collision.surface_ids.push(surface_at(triangle));
    }
}

fn append_collision(collision: &mut TrackCollision, buffers: &MeshBuffers, surface: u16) {
    append_collision_surfaces(collision, buffers, |_| surface);
}

fn collision_bounds(vertices: &[[f32; 3]]) -> Aabb {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        let point = Vec3::from_array(*vertex);
        min = min.min(point);
        max = max.max(point);
    }
    if !min.is_finite() || !max.is_finite() {
        return Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        };
    }
    Aabb {
        min: min.to_array(),
        max: max.to_array(),
    }
}

fn visual_bounds(visual: &TrackVisual) -> Result<Aabb, String> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for buffers in &visual.mesh_buffers {
        for vertex in &buffers.positions {
            let point = Vec3::from_array(*vertex);
            min = min.min(point);
            max = max.max(point);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return Err("procedural track has no finite geometry".to_string());
    }
    Ok(Aabb {
        min: min.to_array(),
        max: max.to_array(),
    })
}

fn empty_buffers() -> MeshBuffers {
    MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    }
}

fn push_mesh(
    visual: &mut TrackVisual,
    name: &str,
    material: u16,
    buffers: MeshBuffers,
    outline: bool,
) {
    if buffers.positions.is_empty() || buffers.indices.is_empty() {
        return;
    }
    let vertex_offset = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.positions.len() as u32)
        .sum();
    let index_offset = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.indices.len() as u32)
        .sum();
    let bounds = collision_bounds(&buffers.positions);
    visual.meshes.push(MeshRef {
        name: name.to_string(),
        vertex_offset,
        vertex_count: buffers.positions.len() as u32,
        index_offset,
        index_count: buffers.indices.len() as u32,
        material,
        bounds,
        lod: 0,
    });
    if let Some(slot) = visual.materials.get_mut(material as usize) {
        slot.outline = outline;
    }
    visual.mesh_buffers.push(buffers);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIGHT_CONTROL_POINTS: [[f32; 3]; 10] = [
        [-30.0, 0.0, -20.0],
        [0.0, 0.0, -35.0],
        [30.0, 0.0, -25.0],
        [40.0, 0.0, 0.0],
        [25.0, 0.0, 25.0],
        [20.0, 0.0, 15.0],
        [13.0, 0.0, 17.0],
        [8.0, 0.0, 13.0],
        [2.0, 0.0, 15.0],
        [-3.0, 0.0, 13.0],
    ];

    fn minimum_radius(points: &[Vec3]) -> f32 {
        let count = points.len();
        (0..count)
            .map(|index| {
                curvature_radius(
                    points[(index + count - 1) % count],
                    points[index],
                    points[(index + 1) % count],
                )
            })
            .fold(f32::INFINITY, f32::min)
    }

    fn segment_crosses(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> bool {
        let orientation =
            |a: Vec3, b: Vec3, c: Vec3| (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x);
        orientation(a0, a1, b0) * orientation(a0, a1, b1) < 0.0
            && orientation(b0, b1, a0) * orientation(b0, b1, a1) < 0.0
    }

    fn offset_crossings(points: &[Vec3], offset: f32) -> usize {
        let count = points.len();
        let offset_points = (0..count)
            .map(|index| {
                let tangent = (points[(index + 1) % count] - points[index]).normalize_or_zero();
                points[index] + Vec3::new(-tangent.z, 0.0, tangent.x) * offset
            })
            .collect::<Vec<_>>();
        let mut crossings = 0;
        for first in 0..count {
            for second in (first + 2)..count {
                if first == 0 && second == count - 1 {
                    continue;
                }
                if segment_crosses(
                    offset_points[first],
                    offset_points[(first + 1) % count],
                    offset_points[second],
                    offset_points[(second + 1) % count],
                ) {
                    crossings += 1;
                }
            }
        }
        crossings
    }

    #[test]
    fn kerb_spans_can_overlap_across_sides() {
        let spans = sanitize_kerb_spans(
            &[
                KerbSpan {
                    start: 0.0,
                    end: 20.0,
                    side: -1,
                },
                KerbSpan {
                    start: 0.0,
                    end: 20.0,
                    side: 1,
                },
            ],
            120,
        );
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn spawn_faces_increasing_track_direction_after_drive_flip() {
        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let spawn = &track.metadata.spawns[0];
        let rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let drive_rotation = Quat::from_rotation_y(PI) * rotation;
        let forward = drive_rotation * Vec3::NEG_Z;
        let points = sample_track(&CONTROL_POINTS);
        let tangent = (points[1] - points[0]).normalize_or_zero();
        assert!(forward.dot(tangent) > 0.99);
    }

    #[test]
    fn track_surface_uses_one_continuous_ribbon() {
        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let surface_meshes = track
            .visual
            .meshes
            .iter()
            .filter(|mesh| mesh.name == "asphalt_dither")
            .count();
        assert_eq!(surface_meshes, 1);
        assert!(!track
            .visual
            .meshes
            .iter()
            .any(|mesh| mesh.name.starts_with("dither_shoulder")));
        assert!(track.collision.surface_ids.contains(&TRACK_SURFACE));
        assert!(track.collision.surface_ids.contains(&DIRT_SURFACE));
    }

    #[test]
    fn editor_shoulders_collide_as_dirt() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let (point, binormal) = track_frame(&points, 10.0);
        for lateral in [7.0, 8.0, 9.0, 10.0, 11.0] {
            for side in [-1.0, 1.0] {
                let sample = point + binormal * (lateral * side);
                let hit = collider
                    .raycast(
                        Vec3::new(sample.x, sample.y + 20.0, sample.z),
                        Vec3::NEG_Y,
                        100.0,
                    )
                    .unwrap_or_else(|| panic!("no hit at lateral {lateral} side {side}"));
                assert_eq!(
                    hit.surface, DIRT_SURFACE,
                    "lateral {lateral} side {side} hit surface {}",
                    hit.surface
                );
            }
        }
    }

    #[test]
    fn editor_track_exports_a_valid_pack() {
        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let bytes = pack_from_track(&track).expect("export pack");
        let pack = sim_asset::Pack::from_bytes(&bytes).expect("parse pack");
        assert_eq!(pack.kind(), sim_types::AssetKind::Track);
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Visual).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Collision).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Metadata).is_ok());
        let metadata = pack.track_metadata_owned().expect("metadata");
        assert!(metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "kerb"));
        assert!(metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "wall"));
        let collision = pack.track_collision_owned().expect("collision");
        assert!(collision.surface_ids.contains(&KERB_SURFACE));
        assert!(collision.surface_ids.contains(&WALL_SURFACE));
        assert_eq!(pack.track_visual_owned().expect("visual"), track.visual);
    }

    #[test]
    fn prop_lines_use_simplified_wall_collision() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let lines = default_prop_lines(&points);
        assert!(lines.iter().any(|line| line.kind != PropKind::Off));
        for line in lines.into_iter().filter(|line| line.kind != PropKind::Off) {
            let (half_width, _) = prop_wall_dimensions(line.kind).expect("prop wall");
            let side = line.side as f32;
            let (point, binormal) = track_frame(&points, (line.start + line.end) * 0.5);
            let center = point + binormal * (PROP_OFFSET * side);
            let contact = |position: Vec3, radius: f32| {
                collider.wall_contacts(
                    Vec3::new(
                        position.x,
                        terrain_height(position.x, position.z) + 0.4,
                        position.z,
                    ),
                    radius,
                )
            };
            assert!(contact(center, 0.2).is_empty());
            let inner_face = center - binormal * (half_width * side);
            assert!(!contact(inner_face - binormal * (side * 0.1), 0.2).is_empty());
            assert!(contact(inner_face - binormal * (side * 0.8), 0.2).is_empty());
        }
    }

    #[test]
    fn kerb_top_is_driveable_collision() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let spans = [KerbSpan {
            start: 10.25,
            end: 10.75,
            side: 1,
        }];
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, Some(&spans), Some(&[]))
            .expect("editor track");
        assert!(track
            .metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "kerb"));
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);

        let (point, binormal) = track_frame(&points, 10.5);
        let sample = point + binormal * 6.4;
        let hit = collider
            .raycast(
                Vec3::new(sample.x, sample.y + 20.0, sample.z),
                Vec3::NEG_Y,
                50.0,
            )
            .expect("kerb raycast");
        let kerb = track
            .metadata
            .surfaces
            .iter()
            .find(|surface| surface.name == "kerb")
            .expect("kerb surface");
        assert_eq!(hit.surface, KERB_SURFACE);
        assert!((hit.friction - kerb.friction).abs() < 1e-5);
        assert!(hit.point.y > terrain_height(hit.point.x, hit.point.z) + 0.07);
    }

    #[test]
    fn jagged_kerb_segments_are_closed_by_risers() {
        let points = sample_track(&CONTROL_POINTS);
        let index = 10.0;
        let (start, start_binormal) = track_frame(&points, index);
        let (middle, middle_binormal) = track_frame(&points, index + 1.0);
        let (end, end_binormal) = track_frame(&points, index + 2.0);
        let mut buffers = empty_buffers();
        let (_, first_end) = append_kerb(
            &mut buffers,
            start,
            middle,
            start_binormal,
            middle_binormal,
            1.0,
        );
        let (second_start, _) = append_kerb(
            &mut buffers,
            middle,
            end,
            middle_binormal,
            end_binormal,
            1.0,
        );
        assert!(first_end[1].y > second_start[1].y + 0.01);
        let before = buffers.positions.len();
        append_kerb_riser(&mut buffers, first_end, second_start, end - middle);
        assert_eq!(buffers.positions.len(), before + 3);
        let riser = &buffers.positions[before..];
        assert!(riser
            .iter()
            .any(|position| Vec3::from_array(*position).distance(first_end[1]) < 1e-5));
        assert!(riser
            .iter()
            .any(|position| Vec3::from_array(*position).distance(second_start[1]) < 1e-5));
    }

    #[test]
    fn low_speed_turn_can_cross_the_kerb() {
        use sim_core::{Controls, Vehicle};

        let track = track_from_control_points(&CONTROL_POINTS).expect("editor track");
        let collider = sim_core::TrackCollider::from_collision(&track.collision, &track.metadata);
        let car_bytes = include_bytes!("../../../fixtures/cars/mazda-rx7/mazda-rx7.sima").to_vec();
        let pack = sim_asset::Pack::from_bytes(&car_bytes).expect("car pack");
        let physics = pack.car_physics_owned().expect("car physics");
        let mut spawn = track.metadata.spawns[0].clone();
        let spawn_rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        spawn.rotation_xyzw = (Quat::from_rotation_y(PI) * spawn_rotation).to_array();
        let mut vehicle = Vehicle::new(physics, collider, &spawn);
        for _ in 0..500 {
            vehicle.step(Controls::default());
        }
        let accelerate = Controls {
            throttle: 0.35,
            gear: 1,
            ..Controls::default()
        };
        for _ in 0..1000 {
            vehicle.step(accelerate);
        }
        let turn = Controls {
            throttle: 0.35,
            steering: 0.9,
            gear: 1,
            ..Controls::default()
        };
        let mut touched_kerb = false;
        for _ in 0..2500 {
            vehicle.step(turn);
            touched_kerb |= vehicle
                .state
                .wheels
                .iter()
                .any(|wheel| wheel.surface_id == KERB_SURFACE);
        }
        assert!(touched_kerb);
        assert!(
            vehicle.state.velocity.length() * 3.6 > 10.0,
            "car stalled on the kerb: {} kph",
            vehicle.state.velocity.length() * 3.6
        );
    }

    #[test]
    fn tight_control_points_are_smoothed_to_the_minimum_radius() {
        let points = sample_track(&TIGHT_CONTROL_POINTS);
        assert!(minimum_radius(&points) >= MIN_TURN_RADIUS - 0.01);
        assert_eq!(offset_crossings(&points, OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, -OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_OFFSET), 0);
    }

    #[test]
    fn prop_spacing_is_uniform_along_offset_paths() {
        let points = sample_track(&TIGHT_CONTROL_POINTS);
        for line in default_prop_lines(&points) {
            let spacing = match line.kind {
                PropKind::Blocks => BLOCK_SPACING,
                PropKind::Tyres => TYRE_SPACING,
                PropKind::Off => continue,
            };
            let placements = prop_placements(&points, &line);
            assert!(placements.len() > 2);
            for pair in placements.windows(2) {
                let distance = pair[0].0.distance(pair[1].0);
                assert!(
                    (distance - spacing).abs() <= spacing * 0.03,
                    "prop spacing drifted: expected {spacing}, got {distance}"
                );
            }
        }
    }

    #[test]
    fn fractional_prop_range_follows_the_offset_path() {
        let points = sample_track(&CONTROL_POINTS);
        let line = PropLine {
            start: 10.25,
            end: 12.75,
            side: 1,
            kind: PropKind::Blocks,
        };
        let vertices = offset_path_vertices(
            &points,
            line.start,
            line.end,
            PROP_OFFSET * line.side as f32,
        );
        assert_eq!(vertices.len(), 4);
        let (start, start_binormal) = track_frame(&points, line.start);
        let (end, end_binormal) = track_frame(&points, line.end);
        assert!(vertices[0].distance(start + start_binormal * PROP_OFFSET) < 1e-3);
        assert!(
            vertices
                .last()
                .expect("end vertex")
                .distance(end + end_binormal * PROP_OFFSET)
                < 1e-3
        );
        assert!(!prop_placements(&points, &line).is_empty());
    }

    #[test]
    fn prop_jitter_is_tiny_and_deterministic() {
        let points = sample_track(&CONTROL_POINTS);
        let mut moved = false;
        for line in default_prop_lines(&points) {
            for (index, (position, tangent)) in
                prop_placements(&points, &line).into_iter().enumerate()
            {
                let (jittered, heading) = jittered_prop_placement(position, tangent, &line, index);
                let (again, again_heading) =
                    jittered_prop_placement(position, tangent, &line, index);
                let offset = jittered.distance(position);
                moved |= offset > 1e-4;
                assert!(offset <= 0.05);
                assert_eq!(jittered, again);
                assert_eq!(heading, again_heading);
            }
        }
        assert!(moved);
    }

    #[test]
    fn default_track_meets_the_minimum_radius() {
        let points = sample_track(&CONTROL_POINTS);
        assert!(minimum_radius(&points) >= MIN_TURN_RADIUS - 0.01);
        assert_eq!(offset_crossings(&points, OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, -OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_OFFSET), 0);
    }
}
