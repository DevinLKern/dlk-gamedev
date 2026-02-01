use crate::{Vec3, Vec4, Mat4, Quat};

#[allow(dead_code)]
pub struct AffineTransform {
    pub position: Vec3<f32>,
    pub orientation: Quat,
    pub scalar: Vec3<f32>,
}

impl AffineTransform {
    #[inline]
    pub const fn move_global(&mut self, offset: Vec3<f32>) {
        self.position.add_assign(offset);
    }
    #[inline]
    pub const fn move_local(&mut self, offset: Vec3<f32>) {
        let offset = self.orientation.rotate_vec(offset);
        self.position.add_assign(offset);
    }
    pub const fn rotate_global(&mut self, rotation: Quat, pivot: Vec3<f32>) {
        self.orientation = rotation.mul(self.orientation);
        self.position.sub_assign(pivot);
        self.position = rotation.rotate_vec(self.position);
        self.position.add_assign(pivot);
    }
    #[inline]
    pub const fn rotate_local(&mut self, rotation: Quat) {
        self.orientation.mul_assign(rotation);
    }
    #[inline]
    pub const fn get_translation_matrix(&self) -> Mat4<f32> {
        Mat4::translation(self.position)
    }
    #[inline]
    pub const fn get_rotation_matrix(&self) -> Mat4<f32> {
        self.orientation.as_mat4()
    }
    #[inline]
    pub const fn scale_uniform(&mut self, s: f32) {
        self.scalar.scale_assign(s);
    }
    #[inline]
    pub const fn scale_nonuniform(&mut self, s: Vec3<f32>) {
        self.scalar.scale_assign_nonuniform(s);
    }
    #[inline]
    pub const fn get_scaling_matrix(&self) -> Mat4<f32> {
        Mat4::scaling(Vec4::from_vec3(self.scalar, 1.0))
    }
    pub const fn as_mat4(&self) -> Mat4<f32> {
        let t = self.get_translation_matrix();
        let r = self.get_rotation_matrix();
        let s = self.get_scaling_matrix();

        r.mul(&t).mul(&s)
    }
}

