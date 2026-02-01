use math::{
    Mat4,
    Quat,
    Vec3,
    Vec4,
    RigidTransform,
};

use crate::{WORLD_FORWARDS, WORLD_RIGHT, WORLD_UP};

pub struct Camera {
    pub transform: RigidTransform,
    pitch: f32,
    fov_y: f32,
    aspect_ratio: f32,
    near: f32,
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            transform: RigidTransform::default(),
            pitch: 0.0,
            fov_y: 90.0,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    pub fn new(fov_y: f32, aspect_ratio: f32, position: Vec3<f32>, yaw: f32, pitch: f32) -> Self {
        let transform = RigidTransform::new(
            position,
            Quat::unit_from_angle_axis(std::f32::consts::PI, WORLD_UP)
        );
        let mut r = Self {
            transform,
            pitch: 0.0,
            fov_y,
            aspect_ratio,
            near: 0.1,
            far: 1000.0,
        };

        r.rotate(yaw, pitch);

        r
    }
    pub fn set_aspect_ratio(&mut self, new_aspect_ratio: f32) {
        self.aspect_ratio = new_aspect_ratio;
    }
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        const LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;

        let q_yaw = Quat::unit_from_angle_axis(dx, WORLD_UP);

        let new_pitch = (self.pitch + dy).clamp(-LIMIT, LIMIT);
        let allowed_dy = new_pitch - self.pitch;
        self.pitch = new_pitch;
        let q_pitch = Quat::unit_from_angle_axis(allowed_dy, WORLD_RIGHT);

        self.transform.rotate_global(q_yaw, self.transform.position);
        self.transform.rotate_local(q_pitch);
    }
    #[inline]
    pub const fn move_global(&mut self, offset: Vec3<f32>) {
        self.transform.translate_global(offset);
    }
    #[inline]
    pub const fn move_local(&mut self, offset: Vec3<f32>) {
        // TODO: Why does offset need to be scaled by -1?
        // This seems off. The view matrix is already inverted.
        self.transform.translate_local(offset.scaled(-1.0));
    }
    #[inline]
    pub const fn get_view_matrix(&self) -> Mat4<f32> {
        let inv = self.transform.inv();
        let t = inv.get_translation_matrix();
        let r = inv.get_rotation_matrix();

        r.mul(&t)
    }
    #[inline]
    pub fn get_projection_matrix(&self) -> Mat4<f32> {
        const WORLD_TO_VK: Mat4<f32> = Mat4::from_cols(
            Vec4::new(
                WORLD_RIGHT.dot(vulkan::VK_DIR_RIGHT),
                WORLD_RIGHT.dot(vulkan::VK_DIR_UP),
                WORLD_RIGHT.dot(vulkan::VK_DIR_FORWARDS),
                0.0,
            ),
            Vec4::new(
                WORLD_UP.dot(vulkan::VK_DIR_RIGHT),
                WORLD_UP.dot(vulkan::VK_DIR_UP),
                WORLD_UP.dot(vulkan::VK_DIR_FORWARDS),
                0.0,
            ),
            Vec4::new(
                WORLD_FORWARDS.dot(vulkan::VK_DIR_RIGHT),
                WORLD_FORWARDS.dot(vulkan::VK_DIR_UP),
                WORLD_FORWARDS.dot(vulkan::VK_DIR_FORWARDS),
                0.0,
            ),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        let n = self.near;
        let f = self.far;
        const R: f32 = vulkan::VK_VIEW_VOLUME_RIGHT;
        const L: f32 = vulkan::VK_VIEW_VOLUME_LEFT;
        const T: f32 = vulkan::VK_VIEW_VOLUME_TOP;
        const B: f32 = vulkan::VK_VIEW_VOLUME_BOTTOM;

        let half_tan = (self.fov_y / 2.0).tan();

        let p = Mat4::from_cols(
            Vec4::new(1.0 / (self.aspect_ratio * half_tan), 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0 / half_tan, 0.0, 0.0),
            Vec4::new((R + L) / (R - L), (T + B) / (T - B), f / (f - n), 1.0),
            Vec4::new(0.0, 0.0, -f * n / (f - n), 0.0),
        );

        p.mul(&WORLD_TO_VK)
    }
}
