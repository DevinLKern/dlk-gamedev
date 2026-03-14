use core::f32;

use math::{Identity, Mat4, Quat, RigidTransform, Vec3, Vec4, Zero};

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

#[allow(dead_code)]
impl Camera {
    pub fn new(fov_y: f32, aspect_ratio: f32, position: Vec3<f32>, forward: Vec3<f32>) -> Self {
        let forward = forward.normalized();

        let axis = WORLD_FORWARDS.cross(forward);
        let dot = WORLD_FORWARDS.dot(forward).clamp(-1.0, 1.0);
        let angle = dot.acos();

        let rotation = if axis.length_squared() < 1e-6 {
            if dot < 0.0 {
                Quat::unit_from_angle_axis(std::f32::consts::PI, WORLD_UP)
            } else {
                Quat::IDENTITY
            }
        } else {
            Quat::unit_from_angle_axis(angle, axis.normalized())
        };

        let transform = RigidTransform::new(position, rotation);

        Self {
            transform,
            pitch: 0.0,
            fov_y,
            aspect_ratio,
            near: 0.1,
            far: 1000.0,
        }
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
    pub fn look_at(&mut self, target: Vec3<f32>) {
        // TODO: Redo this funciton. It should be agnostic regarding what coordinate system is being used.
        // Also, the math might be wrong. Also, is this even doing anything?
        const LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;

        let dir = target.sub(self.transform.position).normalized();
        let yaw_angle = dir.x().atan2(dir.z());
        let pitch_angle = dir.y().asin().clamp(-LIMIT, LIMIT);

        let yaw = math::Quat::unit_from_angle_axis(yaw_angle, WORLD_UP);
        self.transform.rotate_global(yaw, self.transform.position);

        let pitch = math::Quat::unit_from_angle_axis(pitch_angle, WORLD_RIGHT);
        self.transform.rotate_local(pitch);
    }
    #[inline]
    pub const fn move_global(&mut self, offset: Vec3<f32>) {
        self.transform.translate_global(offset);
    }
    #[inline]
    pub const fn move_local(&mut self, offset: Vec3<f32>) {
        self.transform.translate_local(offset);
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
        const WORLD_TO_VK: Mat4<f32> = {
            use math::Mat3;

            const FROM_WORLD: Mat3<f32> = Mat3::from_rows(WORLD_RIGHT, WORLD_UP, WORLD_FORWARDS);

            const TO_VK: Mat3<f32> = Mat3::from_cols(
                vulkan::VK_DIR_RIGHT,
                vulkan::VK_DIR_UP,
                vulkan::VK_DIR_FORWARDS,
            );

            FROM_WORLD.mul(&TO_VK).into_mat4(1.0)
        };

        let n = self.near;
        let f = self.far;
        const R: f32 = vulkan::VK_VIEW_VOLUME_RIGHT;
        const L: f32 = vulkan::VK_VIEW_VOLUME_LEFT;
        const T: f32 = vulkan::VK_VIEW_VOLUME_TOP;
        const B: f32 = vulkan::VK_VIEW_VOLUME_BOTTOM;

        let half_tan = (self.fov_y.to_radians() / 2.0).tan();

        let p = Mat4::from_cols(
            Vec4::new(1.0 / (self.aspect_ratio * half_tan), 0.0, 0.0, 0.0),
            Vec4::new(0.0, -1.0 / half_tan, 0.0, 0.0),
            Vec4::new((R + L) / (R - L), (T + B) / (T - B), -f / (f - n), -1.0),
            Vec4::new(0.0, 0.0, -f * n / (f - n), 0.0),
        );

        p.mul(&WORLD_TO_VK)
    }
}


#[cfg(test)]
mod test {
    use crate::{Camera, constants::WORLD_FORWARDS, constants::WORLD_RIGHT};
    use math::Vec3;
    fn approx_eq_f32(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }
    fn approx_eq_vec3(a: Vec3<f32>, b: Vec3<f32>) -> bool {
        const EPS: f32 = 0.0001;

        if !approx_eq_f32(a.x(), b.x(), EPS) {
            return false;
        }
        if !approx_eq_f32(a.y(), b.y(), EPS) {
            return false;
        }
        if !approx_eq_f32(a.z(), b.z(), EPS) {
            return false;
        }

        return true;
    }
    
    #[test]
    fn move_local() {
        let mut c = Camera::default();

        c.move_local(WORLD_FORWARDS);

        assert_eq!(c.transform.position, WORLD_FORWARDS);
        
        let mut c = Camera::default();

        c.move_local(WORLD_RIGHT);

        assert_eq!(c.transform.position, WORLD_RIGHT);

        let mut c = Camera::default();

        c.rotate(std::f32::consts::PI, 0.0);
        c.move_local(WORLD_FORWARDS);

        assert_eq!(approx_eq_vec3(c.transform.position, WORLD_FORWARDS.scaled(-1.0)), true);
    }
}
