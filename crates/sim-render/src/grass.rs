// Grass placement and instance format adapted from Stylized Components grassField (MIT).
// Source: https://github.com/cortiz2894/stylized-components
// See THIRD_PARTY_NOTICES.md.

use std::collections::{HashMap, VecDeque};
use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use sim_core::TrackCollider;
use sim_types::{TrackCollision, TrackMetadata};

const GRASS_CHUNK_SIZE: f32 = 16.0;
const GRASS_ACTIVE_RADIUS: f32 = 90.0;
const GRASS_DROP_RADIUS: f32 = 110.0;
const GRASS_DENSITY: f32 = 40.0;
const GRASS_BLADES_PER_CLUMP: usize = 8;
const GRASS_CLUMP_RADIUS: f32 = 0.28;
const GRASS_MIN_NORMAL_Y: f32 = 0.6;
const GRASS_MOVE_THRESHOLD: f32 = 12.0;
const GRASS_GRID_RES: usize = 16;

pub const GRASS_GROUND_COLOR: [f32; 3] = [0.032, 0.074, 0.004];
pub const GRASS_CLUMPS_PER_FRAME: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GrassChunkKey {
    pub x: i32,
    pub z: i32,
}

impl GrassChunkKey {
    fn center(self) -> Vec3 {
        Vec3::new(
            (self.x as f32 + 0.5) * GRASS_CHUNK_SIZE,
            0.0,
            (self.z as f32 + 0.5) * GRASS_CHUNK_SIZE,
        )
    }

    fn min(self) -> Vec3 {
        Vec3::new(
            self.x as f32 * GRASS_CHUNK_SIZE,
            0.0,
            self.z as f32 * GRASS_CHUNK_SIZE,
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GrassInstance {
    pub base: [f32; 4],
    pub shape: [f32; 4],
}

pub struct GrassField {
    collider: Option<TrackCollider>,
    grass_surfaces: Vec<u16>,
    center: Vec3,
    chunks: HashMap<GrassChunkKey, Vec<GrassInstance>>,
    pending: VecDeque<GrassChunkKey>,
    builder: Option<ChunkBuilder>,
}

impl GrassField {
    pub fn new(collision: &TrackCollision, metadata: &TrackMetadata) -> Self {
        let grass_surfaces = metadata
            .surfaces
            .iter()
            .filter(|surface| surface.name.to_ascii_uppercase().contains("GRASS"))
            .map(|surface| surface.id)
            .collect::<Vec<_>>();
        let Some(spawn) = metadata.spawns.first() else {
            return Self::empty();
        };
        if grass_surfaces.is_empty() {
            return Self::empty();
        }

        let center = Vec3::from_array(spawn.position);
        let mut field = Self {
            collider: Some(TrackCollider::from_collision(collision, metadata)),
            grass_surfaces,
            center,
            chunks: HashMap::new(),
            pending: VecDeque::new(),
            builder: None,
        };
        field.refresh_chunks();
        while field.process_pending(usize::MAX) > 0 {}
        field
    }

    fn empty() -> Self {
        Self {
            collider: None,
            grass_surfaces: Vec::new(),
            center: Vec3::ZERO,
            chunks: HashMap::new(),
            pending: VecDeque::new(),
            builder: None,
        }
    }

    pub fn chunks(&self) -> &HashMap<GrassChunkKey, Vec<GrassInstance>> {
        &self.chunks
    }

    pub fn len(&self) -> usize {
        self.chunks.values().map(Vec::len).sum()
    }

    pub fn update(&mut self, center: Vec3) -> bool {
        if self.collider.is_none() {
            return false;
        }
        let dx = center.x - self.center.x;
        let dz = center.z - self.center.z;
        if dx * dx + dz * dz < GRASS_MOVE_THRESHOLD * GRASS_MOVE_THRESHOLD {
            return false;
        }
        self.center = center;
        self.refresh_chunks();
        true
    }

    pub fn process_pending(&mut self, clump_budget: usize) -> usize {
        let Some(collider) = &self.collider else {
            return 0;
        };

        if self.builder.is_none() {
            let Some(key) = self.pending.pop_front() else {
                return 0;
            };
            if self.chunks.contains_key(&key) {
                return 0;
            }
            self.builder = Some(ChunkBuilder::new(
                key,
                collider,
                &self.grass_surfaces,
                self.center.y + 200.0,
            ));
        }

        let builder = self.builder.as_mut().expect("grass builder");
        builder.process(clump_budget);

        if builder.done {
            let builder = self.builder.take().expect("completed grass builder");
            self.chunks.insert(builder.key, builder.blades);
            return 1;
        }
        0
    }

    fn refresh_chunks(&mut self) {
        let center = self.center;
        self.chunks
            .retain(|key, _| distance_squared(key.center(), center) <= GRASS_DROP_RADIUS.powi(2));

        self.pending.clear();
        let mut keys = chunk_keys_around(center);
        keys.sort_by(|a, b| {
            distance_squared(a.center(), center)
                .partial_cmp(&distance_squared(b.center(), center))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for key in keys {
            if !self.chunks.contains_key(&key) {
                self.pending.push_back(key);
            }
        }
    }
}

struct ChunkBuilder {
    key: GrassChunkKey,
    blades: Vec<GrassInstance>,
    grass_points: Vec<Vec3>,
    rng: Lcg,
    target: usize,
    clumps_left: usize,
    done: bool,
}

impl ChunkBuilder {
    fn new(
        key: GrassChunkKey,
        collider: &TrackCollider,
        grass_surfaces: &[u16],
        origin_y: f32,
    ) -> Self {
        let min = key.min();
        let step = GRASS_CHUNK_SIZE / GRASS_GRID_RES as f32;
        let mut grass_points = Vec::with_capacity(GRASS_GRID_RES * GRASS_GRID_RES);
        for iz in 0..GRASS_GRID_RES {
            for ix in 0..GRASS_GRID_RES {
                let x = min.x + (ix as f32 + 0.5) * step;
                let z = min.z + (iz as f32 + 0.5) * step;
                let Some(hit) = collider.raycast(Vec3::new(x, origin_y, z), Vec3::NEG_Y, 500.0)
                else {
                    continue;
                };
                if grass_surfaces.contains(&hit.surface) && hit.normal.y >= GRASS_MIN_NORMAL_Y {
                    grass_points.push(Vec3::new(x, hit.point.y, z));
                }
            }
        }

        let grass_fraction = grass_points.len() as f32 / (GRASS_GRID_RES * GRASS_GRID_RES) as f32;
        let target =
            (GRASS_CHUNK_SIZE * GRASS_CHUNK_SIZE * GRASS_DENSITY * grass_fraction) as usize;
        let clumps = target.div_ceil(GRASS_BLADES_PER_CLUMP);
        let mut rng = Lcg::new(
            0x9e37_79b9
                ^ (key.x as u32).wrapping_mul(0x85eb_ca6b)
                ^ (key.z as u32).wrapping_mul(0xc2b2_ae35),
        );
        rng.next_f32();

        Self {
            key,
            blades: Vec::with_capacity(target),
            grass_points,
            rng,
            target,
            clumps_left: clumps,
            done: clumps == 0,
        }
    }

    fn process(&mut self, budget: usize) {
        let step = GRASS_CHUNK_SIZE / GRASS_GRID_RES as f32;
        let mut spent = 0;
        while spent < budget && !self.done {
            if self.clumps_left == 0 || self.blades.len() >= self.target {
                self.done = true;
                break;
            }
            self.clumps_left -= 1;
            spent += 1;

            let point = self.grass_points[self.rng.next_index(self.grass_points.len())];
            let x = point.x + (self.rng.next_f32() - 0.5) * step;
            let z = point.z + (self.rng.next_f32() - 0.5) * step;

            for _ in 0..GRASS_BLADES_PER_CLUMP {
                if self.blades.len() >= self.target {
                    break;
                }
                let jitter_angle = self.rng.next_f32() * TAU;
                let jitter_radius = self.rng.next_f32().sqrt() * GRASS_CLUMP_RADIUS;
                let base_x = x + jitter_angle.cos() * jitter_radius;
                let base_z = z + jitter_angle.sin() * jitter_radius;
                let yaw = self.rng.next_f32() * TAU;
                let width = 0.045 + self.rng.next_f32() * 0.045;
                let height = 0.12 + self.rng.next_f32() * 0.18;
                let tilt = (self.rng.next_f32() - 0.5) * 0.55;
                let phase = self.rng.next_f32();
                self.blades.push(GrassInstance {
                    base: [base_x, point.y + 0.005, base_z, yaw],
                    shape: [width, height, tilt, phase],
                });
            }
        }
    }
}

fn chunk_keys_around(center: Vec3) -> Vec<GrassChunkKey> {
    let half = (GRASS_ACTIVE_RADIUS / GRASS_CHUNK_SIZE).ceil() as i32;
    let origin_x = (center.x / GRASS_CHUNK_SIZE).floor() as i32;
    let origin_z = (center.z / GRASS_CHUNK_SIZE).floor() as i32;
    let mut keys = Vec::new();
    for dz in -half..=half {
        for dx in -half..=half {
            let key = GrassChunkKey {
                x: origin_x + dx,
                z: origin_z + dz,
            };
            if distance_squared(key.center(), center) <= GRASS_ACTIVE_RADIUS.powi(2) {
                keys.push(key);
            }
        }
    }
    keys
}

fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    dx * dx + dz * dz
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
