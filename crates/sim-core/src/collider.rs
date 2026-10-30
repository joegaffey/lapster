use glam::Vec3;
use sim_types::{TrackCollision, TrackMetadata};

const LEAF_SIZE: usize = 4;
const STACK_SIZE: usize = 128;

#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub surface: u16,
    pub friction: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct WallContact {
    pub normal: Vec3,
    pub penetration: f32,
    pub point: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    fn from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self {
            min: a.min(b).min(c),
            max: a.max(b).max(c),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn extend_point(self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    fn extent(self) -> Vec3 {
        self.max - self.min
    }

    fn ray_intersects(self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
        let mut t_min = 0.0f32;
        let mut t_max = max_distance;
        for axis in 0..3 {
            let origin = origin[axis];
            let direction = direction[axis];
            if direction.abs() < 1e-8 {
                if origin < self.min[axis] || origin > self.max[axis] {
                    return false;
                }
                continue;
            }
            let inverse = 1.0 / direction;
            let mut near = (self.min[axis] - origin) * inverse;
            let mut far = (self.max[axis] - origin) * inverse;
            if near > far {
                core::mem::swap(&mut near, &mut far);
            }
            t_min = t_min.max(near);
            t_max = t_max.min(far);
            if t_max < t_min {
                return false;
            }
        }
        true
    }

    fn intersects_sphere_aabb(self, center: Vec3, radius: f32) -> bool {
        let closest = Vec3::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
            center.z.clamp(self.min.z, self.max.z),
        );
        closest.distance_squared(center) <= radius * radius
    }
}

#[derive(Debug, Clone, Copy)]
struct BvhNode {
    bounds: Bounds,
    left: u32,
    right: u32,
    start: u32,
    count: u32,
}

impl BvhNode {
    fn is_leaf(self) -> bool {
        self.count > 0
    }
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
    bounds: Bounds,
    centroid: Vec3,
    surface: u16,
    friction: f32,
    wall: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TrackCollider {
    triangles: Vec<Triangle>,
    nodes: Vec<BvhNode>,
    order: Vec<u32>,
}

impl TrackCollider {
    pub fn from_collision(collision: &TrackCollision, metadata: &TrackMetadata) -> Self {
        let wall_surfaces = metadata
            .surfaces
            .iter()
            .filter(|spec| spec.name.eq_ignore_ascii_case("wall"))
            .map(|spec| spec.id)
            .collect::<Vec<_>>();
        let mut triangles = Vec::with_capacity(collision.indices.len() / 3);
        for triangle_index in 0..collision.indices.len() / 3 {
            let base = triangle_index * 3;
            let Some(a) = collision.vertices.get(collision.indices[base] as usize) else {
                continue;
            };
            let Some(b) = collision.vertices.get(collision.indices[base + 1] as usize) else {
                continue;
            };
            let Some(c) = collision.vertices.get(collision.indices[base + 2] as usize) else {
                continue;
            };
            let a = Vec3::from_array(*a);
            let b = Vec3::from_array(*b);
            let c = Vec3::from_array(*c);
            let normal = (b - a).cross(c - a);
            if normal.length_squared() < 1e-10 {
                continue;
            }
            let normal = normal.normalize();
            let surface = collision
                .surface_ids
                .get(triangle_index)
                .copied()
                .unwrap_or(0);
            let friction = metadata
                .surfaces
                .iter()
                .find(|spec| spec.id == surface)
                .map(|spec| spec.friction)
                .unwrap_or(1.0);
            triangles.push(Triangle {
                a,
                b,
                c,
                normal,
                bounds: Bounds::from_triangle(a, b, c),
                centroid: (a + b + c) / 3.0,
                surface,
                friction,
                wall: wall_surfaces.contains(&surface),
            });
        }
        let (nodes, order) = build_bvh(&triangles);
        Self {
            triangles,
            nodes,
            order,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
        let direction = direction.normalize_or_zero();
        if direction.length_squared() < 0.5 || self.nodes.is_empty() {
            return None;
        }

        let mut closest = max_distance;
        let mut hit: Option<RayHit> = None;
        let mut stack = [0u32; STACK_SIZE];
        let mut stack_len = 1;
        stack[0] = 0;

        while stack_len > 0 {
            stack_len -= 1;
            let node = self.nodes[stack[stack_len] as usize];
            if !node.bounds.ray_intersects(origin, direction, closest) {
                continue;
            }
            if node.is_leaf() {
                let start = node.start as usize;
                let end = start + node.count as usize;
                for index in &self.order[start..end] {
                    let triangle = self.triangles[*index as usize];
                    if triangle.wall {
                        continue;
                    }
                    if let Some(candidate) = ray_triangle(origin, direction, closest, &triangle) {
                        closest = candidate.distance;
                        hit = Some(candidate);
                    }
                }
            } else if stack_len + 2 <= STACK_SIZE {
                stack[stack_len] = node.left;
                stack[stack_len + 1] = node.right;
                stack_len += 2;
            }
        }
        hit
    }

    pub fn wall_contacts(&self, center: Vec3, radius: f32) -> Vec<WallContact> {
        let mut contacts = Vec::new();
        if self.nodes.is_empty() {
            return contacts;
        }

        let mut stack = [0u32; STACK_SIZE];
        let mut stack_len = 1;
        stack[0] = 0;

        while stack_len > 0 {
            stack_len -= 1;
            let node = self.nodes[stack[stack_len] as usize];
            if !node.bounds.intersects_sphere_aabb(center, radius) {
                continue;
            }
            if node.is_leaf() {
                let start = node.start as usize;
                let end = start + node.count as usize;
                for index in &self.order[start..end] {
                    let triangle = self.triangles[*index as usize];
                    if triangle.normal.y.abs() > 0.7 {
                        continue;
                    }
                    let closest =
                        closest_point_on_triangle(center, triangle.a, triangle.b, triangle.c);
                    let delta = center - closest;
                    let distance_squared = delta.length_squared();
                    if distance_squared >= radius * radius {
                        continue;
                    }
                    let distance = distance_squared.sqrt();
                    let normal = if distance > 1e-5 {
                        delta / distance
                    } else {
                        triangle.normal
                    };
                    contacts.push(WallContact {
                        normal,
                        penetration: radius - distance,
                        point: closest,
                    });
                }
            } else if stack_len + 2 <= STACK_SIZE {
                stack[stack_len] = node.left;
                stack[stack_len + 1] = node.right;
                stack_len += 2;
            }
        }
        contacts.sort_by(|a, b| b.penetration.total_cmp(&a.penetration));
        contacts
    }
}

fn build_bvh(triangles: &[Triangle]) -> (Vec<BvhNode>, Vec<u32>) {
    if triangles.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut order = (0..triangles.len() as u32).collect::<Vec<_>>();
    let mut nodes = Vec::with_capacity(triangles.len().saturating_mul(2));
    build_node(triangles, &mut order, &mut nodes, 0, triangles.len());
    (nodes, order)
}

fn build_node(
    triangles: &[Triangle],
    order: &mut [u32],
    nodes: &mut Vec<BvhNode>,
    start: usize,
    end: usize,
) -> u32 {
    let node_index = nodes.len() as u32;
    nodes.push(BvhNode {
        bounds: Bounds::empty(),
        left: 0,
        right: 0,
        start: 0,
        count: 0,
    });

    let mut bounds = Bounds::empty();
    let mut centroid_bounds = Bounds::empty();
    for index in &order[start..end] {
        let triangle = triangles[*index as usize];
        bounds = bounds.merge(triangle.bounds);
        centroid_bounds = centroid_bounds.extend_point(triangle.centroid);
    }

    let count = end - start;
    if count <= LEAF_SIZE {
        nodes[node_index as usize] = BvhNode {
            bounds,
            left: 0,
            right: 0,
            start: start as u32,
            count: count as u32,
        };
        return node_index;
    }

    let extent = centroid_bounds.extent();
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    if extent[axis] < 1e-6 {
        nodes[node_index as usize] = BvhNode {
            bounds,
            left: 0,
            right: 0,
            start: start as u32,
            count: count as u32,
        };
        return node_index;
    }

    order[start..end].sort_by(|a, b| {
        triangles[*a as usize].centroid[axis].total_cmp(&triangles[*b as usize].centroid[axis])
    });
    let middle = start + count / 2;
    let left = build_node(triangles, order, nodes, start, middle);
    let right = build_node(triangles, order, nodes, middle, end);
    nodes[node_index as usize] = BvhNode {
        bounds,
        left,
        right,
        start: 0,
        count: 0,
    };
    node_index
}

fn ray_triangle(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    triangle: &Triangle,
) -> Option<RayHit> {
    let edge1 = triangle.b - triangle.a;
    let edge2 = triangle.c - triangle.a;
    let h = direction.cross(edge2);
    let determinant = edge1.dot(h);
    if determinant.abs() < 1e-8 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let s = origin - triangle.a;
    let u = s.dot(h) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(q) * inverse;
    if distance <= 0.0001 || distance > max_distance {
        return None;
    }
    let mut normal = triangle.normal;
    if normal.dot(direction) > 0.0 {
        normal = -normal;
    }
    Some(RayHit {
        distance,
        point: origin + direction * distance,
        normal,
        surface: triangle.surface,
        friction: triangle.friction,
    })
}

fn closest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }

    let denominator = 1.0 / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    a + ab * v + ac * w
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_types::{Aabb, SurfaceSpec, TrackMetadata};

    fn metadata() -> TrackMetadata {
        TrackMetadata {
            name: String::from("test"),
            length: 10.0,
            spawns: Vec::new(),
            pit_boxes: Vec::new(),
            cameras: Vec::new(),
            sectors: Vec::new(),
            surfaces: vec![
                SurfaceSpec {
                    id: 0,
                    name: String::from("asphalt"),
                    friction: 1.0,
                    roughness: 0.0,
                },
                SurfaceSpec {
                    id: 1,
                    name: String::from("wall"),
                    friction: 0.8,
                    roughness: 0.0,
                },
            ],
        }
    }

    fn ground_collision() -> TrackCollision {
        TrackCollision {
            vertices: vec![
                [-10.0, 0.0, -10.0],
                [-10.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 0.0, -10.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: Aabb {
                min: [-10.0, 0.0, -10.0],
                max: [10.0, 0.0, 10.0],
            },
        }
    }

    #[test]
    fn raycasts_ground_plane() {
        let collider = TrackCollider::from_collision(&ground_collision(), &metadata());
        let hit = collider
            .raycast(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Y, 3.0)
            .expect("expected ground hit");
        assert!((hit.distance - 1.0).abs() < 1e-4);
        assert!(hit.normal.y > 0.9);
        assert!((hit.friction - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn detects_wall_contact() {
        let collision = TrackCollision {
            vertices: vec![
                [-1.0, 0.0, -1.0],
                [-1.0, 2.0, -1.0],
                [-1.0, 2.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![1, 1],
            bounds: Aabb {
                min: [-1.0, 0.0, -1.0],
                max: [-1.0, 2.0, 1.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
        let contacts = collider.wall_contacts(Vec3::new(-0.8, 1.0, 0.0), 0.5);
        assert!(!contacts.is_empty());
        assert!(contacts.iter().all(|contact| contact.normal.x > 0.9));
    }

    #[test]
    fn detects_contact_along_wall() {
        let collision = TrackCollision {
            vertices: vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 500.0],
                [500.0, 0.0, -500.0],
                [-50.0, 0.0, -5.0],
                [50.0, 0.0, -5.0],
                [50.0, 2.0, -5.0],
                [-50.0, 2.0, -5.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7],
            surface_ids: vec![0, 0, 1, 1],
            bounds: Aabb {
                min: [-500.0, 0.0, -500.0],
                max: [500.0, 2.0, 500.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
        let contacts = collider.wall_contacts(Vec3::new(-9.595604, 0.34650669, -4.4141498), 0.8);
        assert!(!contacts.is_empty(), "expected contact along wall");
    }

    #[test]
    fn raycasts_ignore_wall_surfaces() {
        let collision = TrackCollision {
            vertices: vec![
                [-10.0, 0.0, -10.0],
                [-10.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 0.0, -10.0],
                [-2.0, 2.0, -2.0],
                [-2.0, 2.0, 2.0],
                [2.0, 2.0, 2.0],
                [2.0, 2.0, -2.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7],
            surface_ids: vec![0, 0, 1, 1],
            bounds: Aabb {
                min: [-10.0, 0.0, -10.0],
                max: [10.0, 2.0, 10.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
        let hit = collider
            .raycast(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0)
            .expect("expected ground hit");
        assert_eq!(hit.surface, 0);
        assert!(hit.point.y.abs() < 1e-4);
    }

    #[test]
    fn bvh_raycast_matches_bruteforce() {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut surface_ids = Vec::new();
        let cells = 20;
        for x in 0..cells {
            for z in 0..cells {
                let x0 = x as f32 * 2.0 - 20.0;
                let z0 = z as f32 * 2.0 - 20.0;
                let x1 = x0 + 2.0;
                let z1 = z0 + 2.0;
                let base = vertices.len() as u32;
                vertices.extend_from_slice(&[
                    [x0, 0.0, z0],
                    [x0, 0.0, z1],
                    [x1, 0.0, z1],
                    [x1, 0.0, z0],
                ]);
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                surface_ids.extend_from_slice(&[0, 0]);
            }
        }
        let collision = TrackCollision {
            vertices,
            indices,
            surface_ids,
            bounds: Aabb {
                min: [-20.0, 0.0, -20.0],
                max: [20.0, 0.0, 20.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
        let rays = [
            (Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y),
            (Vec3::new(-15.0, 3.0, -7.0), Vec3::new(0.2, -1.0, 0.1)),
            (Vec3::new(12.0, 2.5, 14.0), Vec3::new(-0.1, -1.0, -0.2)),
        ];
        for (origin, direction) in rays {
            let direction = direction.normalize();
            let bvh = collider.raycast(origin, direction, 50.0);
            let brute = brute_raycast(&collider, origin, direction, 50.0);
            match (bvh, brute) {
                (Some(bvh), Some(brute)) => {
                    assert!((bvh.distance - brute.distance).abs() < 1e-3);
                    assert_eq!(bvh.surface, brute.surface);
                }
                (None, None) => {}
                _ => panic!("BVH and brute force disagreed"),
            }
        }
    }

    fn brute_raycast(
        collider: &TrackCollider,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<RayHit> {
        let mut best = max_distance;
        let mut hit = None;
        for triangle in &collider.triangles {
            if let Some(candidate) = ray_triangle(origin, direction, best, triangle) {
                best = candidate.distance;
                hit = Some(candidate);
            }
        }
        hit
    }
}
