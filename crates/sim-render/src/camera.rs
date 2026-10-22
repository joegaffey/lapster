use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub fov_y: f32,
}

impl Camera {
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let eye = Vec3::from_array(self.eye);
        let target = Vec3::from_array(self.target);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);
        let projection = Mat4::perspective_rh(self.fov_y, aspect.max(0.01), 0.1, 5000.0);
        projection * view
    }
}
