// TODO: add math lib?
type Mat4 = [[f32; 4]; 4];

// TODO: add proper camera class
#[repr(C)]
pub struct CameraUBO {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

impl CameraUBO {
    pub fn new(rx: f32, ry: f32, rz: f32) -> Self {
        let (sx, cx) = rx.sin_cos();
        let (sy, cy) = ry.sin_cos();
        let (sz, cz) = rz.sin_cos();

        let model = [
            [
                cy * cz + sy * sx * sz,
                cx * sz,
                -sy * cz + cy * sx * sz,
                0.0,
            ],
            [
                -cy * sz + sy * sx * cz,
                cx * cz,
                sy * sz + cy * sx * cz,
                0.0,
            ],
            [sy * cx, -sx, cy * cx, 0.0],
            [0.0, 0.0, -2.0, 1.0],
        ];
        let fov: f32 = 90.0;
        let f: f32 = 1.0 / (fov * 0.5).tan();
        let near = 0.1;
        let far = 100.0;
        let aspect = 1.0;

        let proj = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, (near * far) / (near - far), 0.0],
        ];
        Self {
            model,
            view: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            proj,
        }
    }
}

impl Clone for CameraUBO {
    fn clone(&self) -> Self {
        Self {
            model: self.model,
            view: self.view,
            proj: self.proj,
        }
    }
}
