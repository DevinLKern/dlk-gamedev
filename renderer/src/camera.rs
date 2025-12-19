use math::matrices::{Identity, Mat4};

#[repr(C)]
#[derive(Clone)]
pub struct CameraUBO {
    pub model: Mat4<f32>,
    pub view: Mat4<f32>,
    pub proj: Mat4<f32>,
}

impl CameraUBO {
    pub fn new(rx: f32, ry: f32, rz: f32) -> Self {
        let (sx, cx) = rx.sin_cos();
        let (sy, cy) = ry.sin_cos();
        let (sz, cz) = rz.sin_cos();

        let model = {
            let mut m = math::matrices::Mat4::<f32>::default();
            m[0][0] = cy * cz + sy * sx * sz;
            m[0][1] = cx * sz;
            m[0][2] = -sy * cz + cy * sx * sz;

            m[1][0] = -cy * sz + sy * sx * cz;
            m[1][1] = cx * cz;
            m[1][2] = sy * sz + cy * sx * cz;

            m[2][0] = sy * cx;
            m[2][1] = -sx;
            m[2][2] = cy * cx;

            m[3][2] = -2.0;
            m[3][3] = 1.0;

            m
        };
        let fov: f32 = 90.0;
        let f: f32 = 1.0 / (fov * 0.5).tan();
        let near = 0.1;
        let far = 100.0;
        let aspect = 1.0;

        let proj = {
            let mut m = math::matrices::Mat4::<f32>::default();
            m[0][0] = f / aspect;
            m[1][1] = f;
            m[2][2] = far / (near - far);
            m[2][3] = -1.0;
            m[3][2] = (near * far) / (near - far);

            m
        };
        Self {
            model,
            view: math::matrices::Mat4::<f32>::identity(),
            proj,
        }
    }
}
