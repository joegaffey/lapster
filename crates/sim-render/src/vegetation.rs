use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use sim_core::TrackCollider;
use sim_types::{TrackCollision, TrackMetadata, TrackVisual};

use crate::scene::{MeshData, MeshVertex, SceneTexture};

const MAX_TREES: usize = 800;
const MAX_BUSHES: usize = 900;
const TREE_GROUP_CAPS: [usize; 3] = [320, 240, 240];
const BUSH_GROUP_CAPS: [usize; 3] = [360, 270, 270];
const TREE_SPACING_SQ: f32 = 4.6 * 4.6;
const BUSH_SPACING_SQ: f32 = 2.4 * 2.4;
const TRACK_RADIUS: f32 = 800.0;
const MAX_VERGE_DISTANCE: f32 = 35.0;
const GRASS_GRID_STEP: f32 = 5.0;
const MASS_LAYERS: usize = 64;
const MASS_TREES_PER_LAYER: usize = 300;
const MASS_BUSHES_PER_LAYER: usize = 300;
const MASS_JITTER: f32 = 1.0;

const TREE_TINTS: [[f32; 3]; 5] = [
    [0.42, 0.62, 0.22],
    [0.34, 0.58, 0.18],
    [0.50, 0.66, 0.24],
    [0.30, 0.52, 0.20],
    [0.58, 0.68, 0.28],
];

const BUSH_TINTS: [[f32; 3]; 4] = [
    [0.28, 0.48, 0.16],
    [0.36, 0.55, 0.18],
    [0.24, 0.42, 0.14],
    [0.44, 0.58, 0.22],
];

pub struct BuiltVegetation {
    pub texture: SceneTexture,
    pub meshes: Vec<MeshData>,
    pub tiers: [Vec<usize>; 3],
    pub trees: usize,
    pub bushes: usize,
    pub candidates: usize,
    pub collision_candidates: usize,
    pub visual_candidates: usize,
    pub near_candidates: usize,
    pub mid_candidates: usize,
    pub far_candidates: usize,
    pub min_candidate_distance: f32,
    pub sample_candidate: [f32; 3],
    pub mass: MassFoliage,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MassFoliageInstance {
    pub center: [f32; 4],
    pub shape: [f32; 4],
    pub tint: [f32; 4],
    pub cluster: [f32; 4],
}

pub struct MassFoliage {
    pub tree_instances: Vec<MassFoliageInstance>,
    pub bush_instances: Vec<MassFoliageInstance>,
    pub tree_layer_ends: Vec<u32>,
    pub bush_layer_ends: Vec<u32>,
    pub texture: usize,
}

#[derive(Default)]
struct MeshBuilder {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn push_mesh(&self, name: &str, texture: Option<usize>, foliage: bool) -> MeshData {
        MeshData {
            name: name.to_string(),
            vertices: self.vertices.clone(),
            indices: self.indices.clone(),
            model: glam::Mat4::IDENTITY,
            texture,
            grass_texture: None,
            outline: None,
            alpha: 1.0,
            emissive: [0.0; 3],
            casts_static_shadow: !foliage,
            casts_dynamic_shadow: false,
            is_decal: false,
            foliage,
            billboard: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn card(
        &mut self,
        center: Vec3,
        width: f32,
        height: f32,
        yaw: f32,
        tilt: f32,
        cluster_center: Vec3,
        tint: [f32; 3],
        phase: f32,
        wind_weight: f32,
    ) {
        let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let up = (Vec3::Y * tilt.cos() + Vec3::new(yaw.sin(), 0.0, yaw.cos()) * tilt.sin())
            .normalize_or(Vec3::Y);
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let base = self.vertices.len() as u32;
        let corners = [
            (center - right * half_w - up * half_h, [0.0, 0.0], 0.0),
            (center + right * half_w - up * half_h, [1.0, 0.0], 0.0),
            (center + right * half_w + up * half_h, [1.0, 1.0], 1.0),
            (center - right * half_w + up * half_h, [0.0, 1.0], 1.0),
        ];
        for (position, uv, top) in corners {
            let normal = (position - cluster_center + Vec3::Y * 0.28).normalize_or(Vec3::Y);
            let shade = 0.76 + top * 0.24;
            self.vertices.push(MeshVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                color: [tint[0] * shade, tint[1] * shade, tint[2] * shade],
                uv,
                wind: [wind_weight * (0.45 + top * 0.55), phase],
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn trunk(
        &mut self,
        base: Vec3,
        height: f32,
        radius: f32,
        lean: Vec3,
        phase: f32,
        tint: [f32; 3],
    ) {
        const SIDES: usize = 6;
        const RINGS: usize = 3;
        let first = self.vertices.len() as u32;
        for ring in 0..=RINGS {
            let t = ring as f32 / RINGS as f32;
            let center = base + lean * t + Vec3::Y * (height * t);
            let ring_radius = radius * (1.0 - 0.58 * t);
            for side in 0..SIDES {
                let angle = side as f32 / SIDES as f32 * TAU;
                let normal = Vec3::new(angle.cos(), 0.0, angle.sin());
                let position = center + normal * ring_radius;
                let shade = 0.72 + 0.28 * t;
                self.vertices.push(MeshVertex {
                    position: position.to_array(),
                    normal: normal.to_array(),
                    color: [
                        tint[0] * 0.22 * shade,
                        tint[1] * 0.20 * shade,
                        tint[2] * 0.16 * shade,
                    ],
                    uv: [0.0, 0.0],
                    wind: [0.0, phase],
                });
            }
        }
        for ring in 0..RINGS {
            let start = first + (ring * SIDES) as u32;
            let next = start + SIDES as u32;
            for side in 0..SIDES as u32 {
                let a = start + side;
                let b = start + (side + 1) % SIDES as u32;
                let c = next + side;
                let d = next + (side + 1) % SIDES as u32;
                self.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }
}

#[derive(Default)]
struct GroupBuilder {
    foliage: MeshBuilder,
    trunks: MeshBuilder,
}

pub fn build(
    visual: &TrackVisual,
    metadata: &TrackMetadata,
    collision: &TrackCollision,
    seed: u64,
) -> BuiltVegetation {
    let texture = leaf_texture();
    let collider = TrackCollider::from_collision(collision, metadata);
    let grass_surfaces = metadata
        .surfaces
        .iter()
        .filter(|surface| surface.name.to_ascii_uppercase().contains("GRASS"))
        .map(|surface| surface.id)
        .collect::<Vec<_>>();
    let spawn = metadata
        .spawns
        .first()
        .map(|spawn| Vec3::from_array(spawn.position))
        .unwrap_or(Vec3::ZERO);

    let mut groups = (0..3).map(|_| GroupBuilder::default()).collect::<Vec<_>>();
    let mut tree_points: Vec<Vec3> = Vec::new();
    let mut bush_points: Vec<Vec3> = Vec::new();
    let mut trees = 0usize;
    let mut bushes = 0usize;
    let mut rng = Lcg::new(seed as u32);
    let road = RoadMask::from_visual(visual);
    let mut candidates = grid_grass_candidates(&collider, &grass_surfaces, spawn, &road);
    let collision_candidate_count = candidates.len();
    let require_grass_surface = !candidates.is_empty();
    let visual_candidates = if candidates.is_empty() {
        visual_grass_candidates(visual, spawn, &mut rng)
    } else {
        Vec::new()
    };
    let visual_candidate_count = visual_candidates.len();
    candidates.extend(visual_candidates);
    for index in (1..candidates.len()).rev() {
        let swap = rng.next_index(index + 1);
        candidates.swap(index, swap);
    }

    let candidate_count = candidates.len();
    let sample_candidate = candidates
        .first()
        .map(|candidate| candidate.to_array())
        .unwrap_or([0.0; 3]);
    let mut near_candidates = 0usize;
    let mut mid_candidates = 0usize;
    let mut far_candidates = 0usize;
    let mut min_candidate_distance = f32::INFINITY;
    for candidate in &candidates {
        let distance = ((candidate.x - spawn.x).powi(2) + (candidate.z - spawn.z).powi(2)).sqrt();
        min_candidate_distance = min_candidate_distance.min(distance);
        if distance < 30.0 {
            near_candidates += 1;
        } else if distance < 52.0 {
            mid_candidates += 1;
        } else {
            far_candidates += 1;
        }
    }
    let mass_candidates = candidates.clone();
    let mut tree_groups = [0usize; 3];
    let mut bush_groups = [0usize; 3];
    for candidate in candidates {
        let Some(hit) = collider.raycast(
            Vec3::new(candidate.x, candidate.y + 80.0, candidate.z),
            Vec3::NEG_Y,
            200.0,
        ) else {
            continue;
        };
        if hit.normal.y < 0.6
            || road.near(hit.point, 1.0)
            || (require_grass_surface && !grass_surfaces.contains(&hit.surface))
        {
            continue;
        }
        let distance = ((hit.point.x - spawn.x).powi(2) + (hit.point.z - spawn.z).powi(2)).sqrt();
        if distance < 7.0 {
            continue;
        }
        let group = if distance < 30.0 {
            0
        } else if distance < 52.0 {
            1
        } else {
            2
        };
        let roll = rng.next_f32();
        if roll < 0.16
            && tree_groups[group] < TREE_GROUP_CAPS[group]
            && tree_points
                .iter()
                .all(|point| point.distance_squared(hit.point) > TREE_SPACING_SQ)
        {
            tree_points.push(hit.point);
            trees += 1;
            tree_groups[group] += 1;
            add_tree(&mut groups[group], hit.point, &mut rng);
            if trees >= MAX_TREES && bushes >= MAX_BUSHES {
                break;
            }
            continue;
        }
        if roll < 0.34
            && bush_groups[group] < BUSH_GROUP_CAPS[group]
            && bush_points
                .iter()
                .all(|point| point.distance_squared(hit.point) > BUSH_SPACING_SQ)
        {
            bush_points.push(hit.point);
            bushes += 1;
            bush_groups[group] += 1;
            add_bush(&mut groups[group], hit.point, &mut rng);
            if trees >= MAX_TREES && bushes >= MAX_BUSHES {
                break;
            }
        }
    }

    let mass = build_mass(
        &mass_candidates,
        &road,
        &collider,
        &grass_surfaces,
        require_grass_surface,
        &mut rng,
    );
    let mut meshes = Vec::new();
    let mut tiers = [Vec::new(), Vec::new(), Vec::new()];
    for (tier, group) in groups.into_iter().enumerate() {
        if !group.foliage.vertices.is_empty() {
            tiers[tier].push(meshes.len());
            meshes.push(
                group
                    .foliage
                    .push_mesh(&format!("style-foliage-{tier}"), Some(0), true),
            );
        }
        if !group.trunks.vertices.is_empty() {
            tiers[tier].push(meshes.len());
            meshes.push(
                group
                    .trunks
                    .push_mesh(&format!("style-trunks-{tier}"), None, false),
            );
        }
    }

    BuiltVegetation {
        texture,
        meshes,
        tiers,
        trees,
        bushes,
        candidates: candidate_count,
        collision_candidates: collision_candidate_count,
        visual_candidates: visual_candidate_count,
        near_candidates,
        mid_candidates,
        far_candidates,
        min_candidate_distance,
        sample_candidate,
        mass,
    }
}

fn build_mass(
    candidates: &[Vec3],
    road: &RoadMask,
    collider: &TrackCollider,
    grass_surfaces: &[u16],
    require_grass_surface: bool,
    rng: &mut Lcg,
) -> MassFoliage {
    let mut tree_instances = Vec::with_capacity(MASS_LAYERS * MASS_TREES_PER_LAYER * 14);
    let mut bush_instances = Vec::with_capacity(MASS_LAYERS * MASS_BUSHES_PER_LAYER * 5);
    let mut tree_layer_ends = Vec::with_capacity(MASS_LAYERS);
    let mut bush_layer_ends = Vec::with_capacity(MASS_LAYERS);
    if !candidates.is_empty() {
        for _ in 0..MASS_LAYERS {
            for _ in 0..MASS_TREES_PER_LAYER {
                let candidate = candidates[rng.next_index(candidates.len())];
                let point = jitter_point(candidate, rng);
                let Some(hit) = collider.raycast(
                    Vec3::new(point.x, point.y + 80.0, point.z),
                    Vec3::NEG_Y,
                    200.0,
                ) else {
                    continue;
                };
                if hit.normal.y < 0.6
                    || road.near(hit.point, 1.0)
                    || (require_grass_surface && !grass_surfaces.contains(&hit.surface))
                {
                    continue;
                }
                add_mass_tree(&mut tree_instances, hit.point, rng);
            }
            for _ in 0..MASS_BUSHES_PER_LAYER {
                let candidate = candidates[rng.next_index(candidates.len())];
                let point = jitter_point(candidate, rng);
                let Some(hit) = collider.raycast(
                    Vec3::new(point.x, point.y + 80.0, point.z),
                    Vec3::NEG_Y,
                    200.0,
                ) else {
                    continue;
                };
                if hit.normal.y < 0.6
                    || road.near(hit.point, 1.0)
                    || (require_grass_surface && !grass_surfaces.contains(&hit.surface))
                {
                    continue;
                }
                add_mass_bush(&mut bush_instances, hit.point, rng);
            }
            tree_layer_ends.push(tree_instances.len() as u32);
            bush_layer_ends.push(bush_instances.len() as u32);
        }
    }
    MassFoliage {
        tree_instances,
        bush_instances,
        tree_layer_ends,
        bush_layer_ends,
        texture: 0,
    }
}

fn jitter_point(point: Vec3, rng: &mut Lcg) -> Vec3 {
    Vec3::new(
        point.x + (rng.next_f32() - 0.5) * MASS_JITTER * 2.0,
        point.y,
        point.z + (rng.next_f32() - 0.5) * MASS_JITTER * 2.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn push_mass_card(
    instances: &mut Vec<MassFoliageInstance>,
    center: Vec3,
    width: f32,
    height: f32,
    yaw: f32,
    tilt: f32,
    cluster_center: Vec3,
    tint: [f32; 3],
    phase: f32,
    wind_weight: f32,
    kind: f32,
) {
    instances.push(MassFoliageInstance {
        center: [center.x, center.y, center.z, kind],
        shape: [width, height, yaw, tilt],
        tint: [tint[0], tint[1], tint[2], phase],
        cluster: [
            cluster_center.x,
            cluster_center.y,
            cluster_center.z,
            wind_weight,
        ],
    });
}

fn add_mass_tree(instances: &mut Vec<MassFoliageInstance>, base: Vec3, rng: &mut Lcg) {
    let height = 2.6 + rng.next_f32() * 1.9;
    let radius = 1.0 + rng.next_f32() * 0.9;
    let tint = TREE_TINTS[rng.next_index(TREE_TINTS.len())];
    let phase = rng.next_f32() * TAU;
    let lean = Vec3::new(
        (rng.next_f32() - 0.5) * 0.55,
        0.0,
        (rng.next_f32() - 0.5) * 0.55,
    );
    let trunk_tint = [0.12, 0.075, 0.04];
    for index in 0..2 {
        push_mass_card(
            instances,
            base + lean * 0.5 + Vec3::Y * (height * 0.5),
            0.22,
            height,
            phase + index as f32 * FRAC_PI_2,
            0.0,
            base,
            trunk_tint,
            phase,
            0.0,
            1.0,
        );
    }

    let cluster_count = 2 + (rng.next_f32() * 2.0) as usize;
    for cluster in 0..cluster_count {
        let offset = if cluster == 0 {
            Vec3::ZERO
        } else {
            Vec3::new(
                (rng.next_f32() - 0.5) * radius * 1.8,
                (rng.next_f32() - 0.35) * height * 0.26,
                (rng.next_f32() - 0.5) * radius * 1.8,
            )
        };
        let cluster_center = base + lean + Vec3::Y * (height * 0.84) + offset;
        let cluster_radius = radius * (0.85 + rng.next_f32() * 0.35);
        let cards = 4 + (rng.next_f32() * 3.0) as usize;
        for card in 0..cards {
            let yaw = rng.next_f32() * TAU + card as f32 * TAU / cards as f32;
            let tilt = (rng.next_f32() - 0.5) * 0.75;
            let size = cluster_radius * (0.82 + rng.next_f32() * 0.45);
            let center = cluster_center
                + Vec3::new(
                    (rng.next_f32() - 0.5) * cluster_radius * 0.8,
                    (rng.next_f32() - 0.5) * cluster_radius * 0.55,
                    (rng.next_f32() - 0.5) * cluster_radius * 0.8,
                );
            let jitter = 0.82 + rng.next_f32() * 0.36;
            push_mass_card(
                instances,
                center,
                size * 1.12,
                size,
                yaw,
                tilt,
                cluster_center,
                [tint[0] * jitter, tint[1] * jitter, tint[2] * jitter],
                phase + rng.next_f32() * 0.7,
                0.92,
                0.0,
            );
        }
    }
}

fn add_mass_bush(instances: &mut Vec<MassFoliageInstance>, base: Vec3, rng: &mut Lcg) {
    let center = base + Vec3::Y * (0.42 + rng.next_f32() * 0.32);
    let radius = 0.72 + rng.next_f32() * 0.7;
    let tint = BUSH_TINTS[rng.next_index(BUSH_TINTS.len())];
    let phase = rng.next_f32() * TAU;
    let cards = 4 + (rng.next_f32() * 2.0) as usize;
    for card in 0..cards {
        let yaw = rng.next_f32() * TAU + card as f32 * TAU / cards as f32;
        let tilt = (rng.next_f32() - 0.5) * 0.55;
        let size = radius * (0.82 + rng.next_f32() * 0.42);
        let jitter = 0.86 + rng.next_f32() * 0.28;
        push_mass_card(
            instances,
            center,
            size * 1.12,
            size,
            yaw,
            tilt,
            center,
            [tint[0] * jitter, tint[1] * jitter, tint[2] * jitter],
            phase + rng.next_f32() * 0.5,
            0.78,
            0.0,
        );
    }
}

struct RoadMask {
    cell: f32,
    cells: HashMap<(i32, i32), Vec<[Vec3; 3]>>,
    large: Vec<[Vec3; 3]>,
}

impl RoadMask {
    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn from_visual(visual: &TrackVisual) -> Self {
        let cell = 16.0;
        let mut cells: HashMap<(i32, i32), Vec<[Vec3; 3]>> = HashMap::new();
        let mut large = Vec::new();
        for (mesh, buffers) in visual.meshes.iter().zip(visual.mesh_buffers.iter()) {
            let material = visual.materials.get(mesh.material as usize);
            let label = format!(
                "{} {}",
                mesh.name,
                material
                    .map(|material| material.name.as_str())
                    .unwrap_or("")
            )
            .to_ascii_lowercase();
            let is_road = !label.contains("grass")
                && (label.contains("asphalt")
                    || label.contains("asp")
                    || label.contains("tarmac")
                    || label.contains("drift track")
                    || label.contains("track")
                    || label.contains("kerb")
                    || label.contains("road")
                    || label.contains("anello")
                    || label.contains("aspo")
                    || label.contains("pista")
                    || label.contains("circuit")
                    || label.contains("runway")
                    || label.contains("concrete"));
            if !is_road {
                continue;
            }
            for triangle in buffers.indices.chunks_exact(3) {
                let Some(a) = buffers
                    .positions
                    .get(triangle[0] as usize)
                    .map(|position| Vec3::from_array(*position))
                else {
                    continue;
                };
                let Some(b) = buffers
                    .positions
                    .get(triangle[1] as usize)
                    .map(|position| Vec3::from_array(*position))
                else {
                    continue;
                };
                let Some(c) = buffers
                    .positions
                    .get(triangle[2] as usize)
                    .map(|position| Vec3::from_array(*position))
                else {
                    continue;
                };
                let min_x = a.x.min(b.x).min(c.x);
                let max_x = a.x.max(b.x).max(c.x);
                let min_z = a.z.min(b.z).min(c.z);
                let max_z = a.z.max(b.z).max(c.z);
                let min_cell_x = (min_x / cell).floor() as i32;
                let max_cell_x = (max_x / cell).floor() as i32;
                let min_cell_z = (min_z / cell).floor() as i32;
                let max_cell_z = (max_z / cell).floor() as i32;
                if (max_cell_x - min_cell_x + 1) * (max_cell_z - min_cell_z + 1) > 256 {
                    large.push([a, b, c]);
                    continue;
                }
                for cell_z in min_cell_z..=max_cell_z {
                    for cell_x in min_cell_x..=max_cell_x {
                        cells.entry((cell_x, cell_z)).or_default().push([a, b, c]);
                    }
                }
            }
        }
        Self { cell, cells, large }
    }

    fn near(&self, point: Vec3, radius: f32) -> bool {
        let reach = (radius / self.cell).ceil() as i32;
        let center_x = (point.x / self.cell).floor() as i32;
        let center_z = (point.z / self.cell).floor() as i32;
        let radius_sq = radius * radius;
        for dz in -reach..=reach {
            for dx in -reach..=reach {
                let Some(triangles) = self.cells.get(&(center_x + dx, center_z + dz)) else {
                    continue;
                };
                for triangle in triangles {
                    if point_triangle_distance_sq_xz(point, triangle) <= radius_sq {
                        return true;
                    }
                }
            }
        }
        self.large
            .iter()
            .any(|triangle| point_triangle_distance_sq_xz(point, triangle) <= radius_sq)
    }
}

fn point_triangle_distance_sq_xz(point: Vec3, triangle: &[Vec3; 3]) -> f32 {
    let p = Vec2::new(point.x, point.z);
    let a = Vec2::new(triangle[0].x, triangle[0].z);
    let b = Vec2::new(triangle[1].x, triangle[1].z);
    let c = Vec2::new(triangle[2].x, triangle[2].z);
    let ab = b - a;
    let bc = c - b;
    let ca = a - c;
    let ap = p - a;
    let bp = p - b;
    let cp = p - c;
    let d1 = cross2(ab, ap);
    let d2 = cross2(bc, bp);
    let d3 = cross2(ca, cp);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    if !(has_neg && has_pos) {
        return 0.0;
    }
    segment_distance_sq(p, a, b)
        .min(segment_distance_sq(p, b, c))
        .min(segment_distance_sq(p, c, a))
}

fn segment_distance_sq(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let length_sq = ab.length_squared();
    if length_sq <= 1e-8 {
        return (point - a).length_squared();
    }
    let t = ((point - a).dot(ab) / length_sq).clamp(0.0, 1.0);
    (point - (a + ab * t)).length_squared()
}

fn cross2(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn grid_grass_candidates(
    collider: &TrackCollider,
    grass_surfaces: &[u16],
    spawn: Vec3,
    road: &RoadMask,
) -> Vec<Vec3> {
    let step = GRASS_GRID_STEP;
    let half = TRACK_RADIUS;
    let steps = (half / step).ceil() as i32;
    let mut candidates = Vec::new();
    for dz in -steps..=steps {
        for dx in -steps..=steps {
            let x = spawn.x + dx as f32 * step;
            let z = spawn.z + dz as f32 * step;
            if (x - spawn.x).powi(2) + (z - spawn.z).powi(2) > half * half {
                continue;
            }
            let Some(hit) = collider.raycast(Vec3::new(x, 500.0, z), Vec3::NEG_Y, 1000.0) else {
                continue;
            };
            if grass_surfaces.contains(&hit.surface) && road.near(hit.point, MAX_VERGE_DISTANCE) {
                candidates.push(hit.point);
            }
        }
    }
    candidates
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn visual_grass_candidates(visual: &TrackVisual, spawn: Vec3, rng: &mut Lcg) -> Vec<Vec3> {
    let mut candidates = Vec::new();
    for (mesh, buffers) in visual.meshes.iter().zip(visual.mesh_buffers.iter()) {
        let material = visual.materials.get(mesh.material as usize);
        let label = format!(
            "{} {}",
            mesh.name,
            material
                .map(|material| material.name.as_str())
                .unwrap_or("")
        )
        .to_ascii_lowercase();
        if !label.contains("grass") || label.contains("0grassext") || buffers.indices.len() < 3 {
            continue;
        }
        let mut triangles = 0usize;
        for triangle in buffers.indices.chunks_exact(3) {
            let Some(a) = buffers
                .positions
                .get(triangle[0] as usize)
                .map(|position| Vec3::from_array(*position))
            else {
                continue;
            };
            let Some(b) = buffers
                .positions
                .get(triangle[1] as usize)
                .map(|position| Vec3::from_array(*position))
            else {
                continue;
            };
            let Some(c) = buffers
                .positions
                .get(triangle[2] as usize)
                .map(|position| Vec3::from_array(*position))
            else {
                continue;
            };
            for _ in 0..12 {
                let mut u = rng.next_f32();
                let mut v = rng.next_f32();
                if u + v > 1.0 {
                    u = 1.0 - u;
                    v = 1.0 - v;
                }
                let point = a + (b - a) * u + (c - a) * v;
                let distance = ((point.x - spawn.x).powi(2) + (point.z - spawn.z).powi(2)).sqrt();
                if distance <= TRACK_RADIUS {
                    candidates.push(point);
                }
            }
            triangles += 1;
            if triangles >= 5000 || candidates.len() >= 12_000 {
                break;
            }
        }
        if candidates.len() >= 12_000 {
            break;
        }
    }
    for index in (1..candidates.len()).rev() {
        let swap = rng.next_index(index + 1);
        candidates.swap(index, swap);
    }
    candidates
}

fn add_tree(builder: &mut GroupBuilder, base: Vec3, rng: &mut Lcg) {
    let height = 2.6 + rng.next_f32() * 1.9;
    let radius = 1.0 + rng.next_f32() * 0.9;
    let lean = Vec3::new(
        (rng.next_f32() - 0.5) * 0.55,
        0.0,
        (rng.next_f32() - 0.5) * 0.55,
    );
    let tint = TREE_TINTS[rng.next_index(TREE_TINTS.len())];
    let phase = rng.next_f32() * TAU;

    builder.trunks.trunk(
        base,
        height,
        0.10 + rng.next_f32() * 0.07,
        lean,
        phase,
        tint,
    );

    let cluster_count = 2 + (rng.next_f32() * 2.0) as usize;
    for cluster in 0..cluster_count {
        let offset = if cluster == 0 {
            Vec3::ZERO
        } else {
            Vec3::new(
                (rng.next_f32() - 0.5) * radius * 1.8,
                (rng.next_f32() - 0.35) * height * 0.26,
                (rng.next_f32() - 0.5) * radius * 1.8,
            )
        };
        let cluster_center = base + lean + Vec3::Y * (height * 0.84) + offset;
        let cluster_radius = radius * (0.85 + rng.next_f32() * 0.35);
        let cards = 4 + (rng.next_f32() * 3.0) as usize;
        for card in 0..cards {
            let yaw = rng.next_f32() * TAU + card as f32 * TAU / cards as f32;
            let tilt = (rng.next_f32() - 0.5) * 0.75;
            let size = cluster_radius * (0.82 + rng.next_f32() * 0.45);
            let center = cluster_center
                + Vec3::new(
                    (rng.next_f32() - 0.5) * cluster_radius * 0.8,
                    (rng.next_f32() - 0.5) * cluster_radius * 0.55,
                    (rng.next_f32() - 0.5) * cluster_radius * 0.8,
                );
            let jitter = 0.82 + rng.next_f32() * 0.36;
            builder.foliage.card(
                center,
                size * 1.12,
                size,
                yaw,
                tilt,
                cluster_center,
                [tint[0] * jitter, tint[1] * jitter, tint[2] * jitter],
                phase + rng.next_f32() * 0.7,
                0.92,
            );
        }
    }
}

fn add_bush(builder: &mut GroupBuilder, base: Vec3, rng: &mut Lcg) {
    let center = base + Vec3::Y * (0.42 + rng.next_f32() * 0.32);
    let radius = 0.72 + rng.next_f32() * 0.7;
    let tint = BUSH_TINTS[rng.next_index(BUSH_TINTS.len())];
    let phase = rng.next_f32() * TAU;
    let cards = 4 + (rng.next_f32() * 2.0) as usize;
    for card in 0..cards {
        let yaw = rng.next_f32() * TAU + card as f32 * TAU / cards as f32;
        let tilt = (rng.next_f32() - 0.5) * 0.55;
        let size = radius * (0.82 + rng.next_f32() * 0.42);
        let jitter = 0.86 + rng.next_f32() * 0.28;
        builder.foliage.card(
            center,
            size * 1.12,
            size,
            yaw,
            tilt,
            center,
            [tint[0] * jitter, tint[1] * jitter, tint[2] * jitter],
            phase + rng.next_f32() * 0.5,
            0.78,
        );
    }
}

fn leaf_texture() -> SceneTexture {
    const SIZE: usize = 128;
    let mut rng = Lcg::new(0x51f1_5eed);
    let mut leaves = Vec::new();
    for _ in 0..30 {
        let angle = rng.next_f32() * TAU;
        let radius = 0.16 + rng.next_f32() * 0.64;
        let cx = radius * angle.cos();
        let cy = radius * angle.sin() * 0.92;
        let length = 0.13 + rng.next_f32() * 0.20;
        let width = length * (0.30 + rng.next_f32() * 0.18);
        let rotation = angle + (rng.next_f32() - 0.5) * 1.4;
        leaves.push((cx, cy, length, width, rotation, rng.next_f32()));
    }
    for _ in 0..10 {
        let angle = rng.next_f32() * TAU;
        let radius = rng.next_f32() * 0.32;
        let cx = radius * angle.cos();
        let cy = radius * angle.sin();
        let length = 0.10 + rng.next_f32() * 0.13;
        let width = length * (0.34 + rng.next_f32() * 0.16);
        let rotation = angle + (rng.next_f32() - 0.5) * 1.8;
        leaves.push((cx, cy, length, width, rotation, rng.next_f32()));
    }

    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let mut coverage = 0.0f32;
            let mut leaf_shade = 0.5f32;
            let mut vein = 0.0f32;
            for (cx, cy, length, width, rotation, variation) in &leaves {
                let cos = rotation.cos();
                let sin = rotation.sin();
                let dx = u - cx;
                let dy = v - cy;
                let rotated_x = dx * cos + dy * sin;
                let rotated_y = -dx * sin + dy * cos;
                let distance = ((rotated_x / length).powi(2) + (rotated_y / width).powi(2)).sqrt();
                let edge = ((1.0 - distance) / 0.14).clamp(0.0, 1.0);
                let leaf_coverage = edge * edge * (3.0 - 2.0 * edge);
                if leaf_coverage > coverage {
                    coverage = leaf_coverage;
                    leaf_shade = *variation;
                    vein = if rotated_y.abs() < width * 0.16 {
                        1.0
                    } else {
                        0.0
                    };
                }
            }
            if coverage <= 0.0 {
                continue;
            }
            let noise = hash2(x as f32 * 0.27, y as f32 * 0.27);
            let height_shade = 0.56 + (0.5 - v * 0.5) * 0.30;
            let shade = (height_shade + leaf_shade * 0.22 + (noise - 0.5) * 0.12 - vein * 0.16)
                .clamp(0.20, 1.0);
            let index = (y * SIZE + x) * 4;
            rgba[index] = (34.0 + shade * 58.0) as u8;
            rgba[index + 1] = (82.0 + shade * 84.0) as u8;
            rgba[index + 2] = (24.0 + shade * 28.0) as u8;
            rgba[index + 3] = (coverage * 255.0 + 0.5) as u8;
        }
    }
    SceneTexture {
        width: SIZE as u32,
        height: SIZE as u32,
        rgba8: rgba,
    }
}

fn hash2(x: f32, y: f32) -> f32 {
    let mut value = x.sin() * 43758.547 + y.cos() * 22578.145;
    value = (value - value.floor()).abs();
    value
}

struct Lcg(u32);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / 16_777_216.0
    }

    fn next_index(&mut self, len: usize) -> usize {
        ((self.next_f32() * len as f32) as usize).min(len.saturating_sub(1))
    }
}
