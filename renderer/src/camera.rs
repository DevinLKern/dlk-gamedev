use math::{
    matrices::{Mat4, Rotation, Scale, Translation},
    vectors::Vec3,
};

#[repr(C)]
#[derive(Clone)]
pub struct CameraUBO {
    pub model: Mat4<f32>,
    pub view: Mat4<f32>,
    pub proj: Mat4<f32>,
}

pub struct Camera {
    // up: Vec3<f32>,
    position: Vec3<f32>,
    rotation: Vec3<f32>,
    fov_y: f32,
    aspect_ratio: f32,
    near: f32,
    far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            // up: Vec3::new(0.0, -1.0, 0.0),
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            fov_y: 90.0,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn calculate_ubo(
        &self,
        obj_pos: Vec3<f32>,
        obj_scale: Vec3<f32>,
        obj_rotation: Vec3<f32>,
    ) -> CameraUBO {
        // translates model space into world space
        let model: Mat4<f32> = {
            let t = Mat4::translation(obj_pos);

            let s = Mat4::scale(obj_scale);

            let r = Mat4::rotation(obj_rotation);

            t * r * s
        };

        // translates world space to camera space
        let view = {
            let t = Mat4::translation(Vec3::new(
                -self.position[0],
                -self.position[1],
                -self.position[2],
            ));

            let r = Mat4::rotation(Vec3::new(
                -self.rotation[0],
                -self.rotation[1],
                -self.rotation[2],
            ));

            r * t
        };

        // applies perspective
        let proj = {
            // const VK_NEAR: f32 = 0.0;
            // const VK_FAR: f32 = 1.0;
            // const VK_TOP: f32 = -1.0;
            // const VK_BOTTOM: f32 = 1.0;
            // const VK_LEFT: f32 = -1.0;
            // const VK_RIGHT: f32 = 1.0;

            let f: f32 = 1.0 / (self.fov_y * 0.5).tan();

            let mut m = Mat4::default();
            m[0][0] = f / self.aspect_ratio;
            m[1][1] = f;
            m[2][2] = self.far / (self.near - self.far);
            m[2][3] = -1.0;
            m[3][2] = (self.near * self.far) / (self.near - self.far);

            m
        };

        CameraUBO { model, view, proj }
    }
}
