use glam::Vec3;
use sim_types::{TrackCollision, TrackMetadata};

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
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
    surface: u16,
    friction: f32,
}

#[derive(Debug, Clone, Default)]
pub struct TrackCollider {
    triangles: Vec<Triangle>,
}

impl TrackCollider {
    pub fn from_collision(collision: &TrackCollision, metadata: &TrackMetadata) -> Self {
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
                surface,
                friction,
            });
        }
        Self { triangles }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
        let direction = direction.normalize_or_zero();
        if direction.length_squared() < 0.5 {
            return None;
        }

        let mut closest: Option<RayHit> = None;
        for triangle in &self.triangles {
            let edge1 = triangle.b - triangle.a;
            let edge2 = triangle.c - triangle.a;
            let h = direction.cross(edge2);
            let determinant = edge1.dot(h);
            if determinant.abs() < 1e-8 {
                continue;
            }
            let inverse = 1.0 / determinant;
            let s = origin - triangle.a;
            let u = s.dot(h) * inverse;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = s.cross(edge1);
            let v = direction.dot(q) * inverse;
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let distance = edge2.dot(q) * inverse;
            if distance <= 0.0001 || distance > max_distance {
                continue;
            }
            if match closest {
                None => true,
                Some(hit) => distance < hit.distance,
            } {
                let mut normal = triangle.normal;
                if normal.dot(direction) > 0.0 {
                    normal = -normal;
                }
                closest = Some(RayHit {
                    distance,
                    point: origin + direction * distance,
                    normal,
                    surface: triangle.surface,
                    friction: triangle.friction,
                });
            }
        }
        closest
    }

    pub fn wall_contacts(&self, center: Vec3, radius: f32) -> Vec<WallContact> {
        let mut contacts = Vec::new();
        for triangle in &self.triangles {
            if triangle.normal.y.abs() > 0.7 {
                continue;
            }
            let closest = closest_point_on_triangle(center, triangle.a, triangle.b, triangle.c);
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
        contacts
    }
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
    use sim_types::{SurfaceSpec, TrackMetadata};

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

    #[test]
    fn raycasts_ground_plane() {
        let collision = TrackCollision {
            vertices: vec![
                [-10.0, 0.0, -10.0],
                [-10.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 0.0, -10.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            surface_ids: vec![0, 0],
            bounds: sim_types::Aabb {
                min: [-10.0, 0.0, -10.0],
                max: [10.0, 0.0, 10.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
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
            bounds: sim_types::Aabb {
                min: [-1.0, 0.0, -1.0],
                max: [-1.0, 2.0, 1.0],
            },
        };
        let collider = TrackCollider::from_collision(&collision, &metadata());
        let contacts = collider.wall_contacts(Vec3::new(-0.8, 1.0, 0.0), 0.5);
        assert!(!contacts.is_empty());
        assert!(contacts.iter().all(|contact| contact.normal.x > 0.9));
    }
}
