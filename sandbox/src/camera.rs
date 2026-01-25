use math::{
    mat4::Mat4, quat::Quat, traits::{Identity, Zero}, vec3::Vec3, vec4::Vec4
};

use crate::constants::{WORLD_FORWARDS, WORLD_RIGHT, WORLD_UP};

use renderer::render_context::CameraUBO;

pub struct Camera {
    position: Vec3<f32>,
    // yaw: f32,
    pitch: f32,
    orientation: Quat,
    fov_y: f32,
    aspect_ratio: f32,
    near: f32,
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            // yaw: 0.0,
            pitch: 0.0,
            orientation: Quat::IDENTITY,
            fov_y: 90.0,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    pub fn new(fov_y: f32, aspect_ratio: f32, position: Vec3<f32>, yaw: f32, pitch: f32) -> Self {
        let mut r = Self {
            position,
            // yaw: 0.0,
            pitch: 0.0,
            orientation: Quat::unit_from_angle_axis(std::f32::consts::PI, WORLD_UP),
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

        // yaw
        let q_yaw = Quat::unit_from_angle_axis(dx, WORLD_UP);
        self.orientation = q_yaw.mul(self.orientation);

        // pitch
        let old_pitch = self.pitch;
        let new_pitch = (old_pitch + dy).clamp(-LIMIT, LIMIT);
        let allowed_dy = new_pitch - old_pitch;
        self.pitch = new_pitch;

        let q_pitch = Quat::unit_from_angle_axis(allowed_dy, WORLD_RIGHT);

        self.orientation = self.orientation.mul(q_pitch);

        self.orientation = self.orientation.normalized();
    }
    #[inline]
    pub const fn move_global(&mut self, offset: Vec3<f32>) {
        self.position.add_assign(offset);
    }
    #[inline]
    pub const fn move_local(&mut self, offset: Vec3<f32>) {
        // Why does offset need to be scaled by -1? This seems off.
        let offset = self.orientation.rotate_vec(offset).scaled(-1.0);
        self.position.add_assign(offset);
    }
    #[inline]
    pub fn calculate_ubo(
        &self,
        obj_pos: Vec3<f32>,
        obj_scale: Vec3<f32>,
        _obj_rotation: Vec3<f32>,
    ) -> CameraUBO {
        // translates model space into world space
        let model: Mat4<f32> = {
            let t = Mat4::translation(obj_pos);

            let obj_scale = {
                let mut s = obj_scale.into_vec4();
                *s.w_mut() = 1.0;
                s
            };
            let s = Mat4::scaling(obj_scale);

            let r = Mat4::IDENTITY;

            t.mul(&r).mul(&s)
        };

        // translates world space to camera space
        let view = {
            let t = Mat4::translation(self.position.scaled(-1.0));

            let r = self.orientation.conjugate().as_mat4();

            r.mul(&t)
        };

        // applies perspective
        #[allow(unused)]
        let proj = {
            const WORLD_TO_VK: Mat4<f32> = Mat4::from_cols(
                Vec4::new(
                    WORLD_RIGHT.dot(vulkan::constants::VK_DIR_RIGHT),
                    WORLD_RIGHT.dot(vulkan::constants::VK_DIR_UP),
                    WORLD_RIGHT.dot(vulkan::constants::VK_DIR_FORWARDS),
                    0.0,
                ),
                Vec4::new(
                    WORLD_UP.dot(vulkan::constants::VK_DIR_RIGHT),
                    WORLD_UP.dot(vulkan::constants::VK_DIR_UP),
                    WORLD_UP.dot(vulkan::constants::VK_DIR_FORWARDS),
                    0.0,
                ),
                Vec4::new(
                    WORLD_FORWARDS.dot(vulkan::constants::VK_DIR_RIGHT),
                    WORLD_FORWARDS.dot(vulkan::constants::VK_DIR_UP),
                    WORLD_FORWARDS.dot(vulkan::constants::VK_DIR_FORWARDS),
                    0.0,
                ),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
            );

            let n = self.near;
            let f = self.far;
            const R: f32 = vulkan::constants::VK_VIEW_VOLUME_RIGHT;
            const L: f32 = vulkan::constants::VK_VIEW_VOLUME_LEFT;
            const T: f32 = vulkan::constants::VK_VIEW_VOLUME_TOP;
            const B: f32 = vulkan::constants::VK_VIEW_VOLUME_BOTTOM;

            let half_tan = (self.fov_y / 2.0).tan();

            let p = Mat4::from_cols(
                Vec4::new(1.0 / (self.aspect_ratio * half_tan), 0.0, 0.0, 0.0),
                Vec4::new(
                    0.0,
                    1.0 / half_tan,
                    0.0,
                    0.0,
                ),
                Vec4::new((R + L) / (R - L), (T + B) / (T - B), f / (f - n), 1.0),
                Vec4::new(0.0, 0.0, -f * n / (f - n), 0.0),
            );

            p.mul(&WORLD_TO_VK)
        };

        CameraUBO { model, view, proj }
    }
}
