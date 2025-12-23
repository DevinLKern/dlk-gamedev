use crate::vectors::*;
use crate::matrices::Mat4;

#[allow(dead_code)]
pub struct Quaternion {
    a: f32,
    v: Vec3<f32>,
}

#[allow(dead_code)]
impl Quaternion {
    pub fn unit(angle_rad: f32, axis: Vec3<f32>) -> Self {
        let half = angle_rad * 0.5;
        let (s, c) = half.sin_cos();

        Self {
            a: c,
            v: axis.normalized().scaled(s),
        }
    }

    pub fn calc_rotation_matrix(&self) -> Mat4<f32> {
        todo!()
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            a: self.a * rhs.a - self.v.dot(&rhs.v),
            v: rhs.v.scaled(self.a) + self.v.scaled(rhs.a) + self.v.crossed(&rhs.v),
        }
    }
}

impl std::ops::Add for Quaternion {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            a: self.a + rhs.a,
            v: self.v + rhs.v,
        }
    }
}
