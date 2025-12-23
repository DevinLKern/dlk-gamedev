use crate::numbers::{One, Zero};
use crate::vectors::*;

pub trait Identity {
    fn identity() -> Self;
}

pub trait Scale {
    type Input;
    fn scale(input: Self::Input) -> Self;
}

pub trait Translation {
    type Input;
    fn translation(input: Self::Input) -> Self;
}

pub trait Rotation {
    type Input;
    fn rotation(input: Self::Input) -> Self;
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Mat2<T>([Vec2<T>; 2]);

impl<T: Default> Default for Mat2<T> {
    fn default() -> Self {
        Self([Vec2::<T>::default(), Vec2::<T>::default()])
    }
}

impl<T: Zero + One> Identity for Mat2<T> {
    fn identity() -> Self {
        Self([Vec2([T::one(), T::zero()]), Vec2([T::zero(), T::one()])])
    }
}

impl<T> std::ops::Index<usize> for Mat2<T> {
    type Output = Vec2<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Mat2<T> {
    fn index_mut(&mut self, index: usize) -> &mut Vec2<T> {
        &mut self.0[index]
    }
}

impl<T> std::ops::Mul for Mat2<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T>,
{
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Mat2([
            Vec2([
                self[0][0] * rhs[0][0] + self[1][0] * rhs[0][1],
                self[0][1] * rhs[0][0] + self[1][1] * rhs[0][1],
            ]),
            Vec2([
                self[0][0] * rhs[1][0] + self[1][0] * rhs[1][1],
                self[0][1] * rhs[1][0] + self[1][1] * rhs[1][1],
            ]),
        ])
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Mat3<T>([Vec3<T>; 3]);

impl<T: Default> Default for Mat3<T> {
    fn default() -> Self {
        Self([
            Vec3::<T>::default(),
            Vec3::<T>::default(),
            Vec3::<T>::default(),
        ])
    }
}

impl<T: Zero + One> Identity for Mat3<T> {
    fn identity() -> Self {
        Self([
            Vec3([T::one(), T::zero(), T::zero()]),
            Vec3([T::zero(), T::one(), T::zero()]),
            Vec3([T::zero(), T::zero(), T::one()]),
        ])
    }
}

impl<T> std::ops::Index<usize> for Mat3<T> {
    type Output = Vec3<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Mat3<T> {
    fn index_mut(&mut self, index: usize) -> &mut Vec3<T> {
        &mut self.0[index]
    }
}

impl<T> std::ops::Mul for Mat3<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T>,
{
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Mat3([
            Vec3([
                self[0][0] * rhs[0][0] + self[1][0] * rhs[0][1] + self[2][0] * rhs[0][2],
                self[0][1] * rhs[0][0] + self[1][1] * rhs[0][1] + self[2][1] * rhs[0][2],
                self[0][2] * rhs[0][0] + self[1][2] * rhs[0][1] + self[2][2] * rhs[0][2],
            ]),
            Vec3([
                self[0][0] * rhs[1][0] + self[1][0] * rhs[1][1] + self[2][0] * rhs[1][2],
                self[0][1] * rhs[1][0] + self[1][1] * rhs[1][1] + self[2][1] * rhs[1][2],
                self[0][2] * rhs[1][0] + self[1][2] * rhs[1][1] + self[2][2] * rhs[1][2],
            ]),
            Vec3([
                self[0][0] * rhs[2][0] + self[1][0] * rhs[2][1] + self[2][0] * rhs[2][2],
                self[0][1] * rhs[2][0] + self[1][1] * rhs[2][1] + self[2][1] * rhs[2][2],
                self[0][2] * rhs[2][0] + self[1][2] * rhs[2][1] + self[2][2] * rhs[2][2],
            ]),
        ])
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Mat3Std140<T>([Vec3Std140<T>; 3]);

impl<T: Default> Default for Mat3Std140<T> {
    fn default() -> Self {
        Self([
            Vec3Std140::<T>::default(),
            Vec3Std140::<T>::default(),
            Vec3Std140::<T>::default(),
        ])
    }
}

impl<T: Zero + One> Identity for Mat3Std140<T> {
    fn identity() -> Self {
        Self([
            Vec3Std140([T::one(), T::zero(), T::zero(), T::zero()]),
            Vec3Std140([T::zero(), T::one(), T::zero(), T::zero()]),
            Vec3Std140([T::zero(), T::zero(), T::one(), T::zero()]),
        ])
    }
}

impl<T> std::ops::Index<usize> for Mat3Std140<T> {
    type Output = Vec3Std140<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Mat3Std140<T> {
    fn index_mut(&mut self, index: usize) -> &mut Vec3Std140<T> {
        &mut self.0[index]
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Mat4<T>([Vec4<T>; 4]);

impl<T: Default> Default for Mat4<T> {
    fn default() -> Self {
        Self([
            Vec4::<T>::default(),
            Vec4::<T>::default(),
            Vec4::<T>::default(),
            Vec4::<T>::default(),
        ])
    }
}

impl<T: Zero + One> Identity for Mat4<T> {
    fn identity() -> Self {
        Self([
            Vec4([T::one(), T::zero(), T::zero(), T::zero()]),
            Vec4([T::zero(), T::one(), T::zero(), T::zero()]),
            Vec4([T::zero(), T::zero(), T::one(), T::zero()]),
            Vec4([T::zero(), T::zero(), T::zero(), T::one()]),
        ])
    }
}

impl<T: One + Zero + Copy> Scale for Mat4<T> {
    type Input = Vec3<T>;
    fn scale(input: Self::Input) -> Self {
        Self([
            Vec4([input[0], T::zero(), T::zero(), T::zero()]),
            Vec4([T::zero(), input[1], T::zero(), T::zero()]),
            Vec4([T::zero(), T::zero(), input[2], T::zero()]),
            Vec4([T::zero(), T::zero(), T::zero(), T::one()]),
        ])
    }
}

impl<T: Zero + One + Copy> Translation for Mat4<T> {
    type Input = Vec3<T>;
    fn translation(input: Self::Input) -> Self {
        Self([
            Vec4([T::one(), T::zero(), T::zero(), T::zero()]),
            Vec4([T::zero(), T::one(), T::zero(), T::zero()]),
            Vec4([T::zero(), T::zero(), T::one(), T::zero()]),
            Vec4([input[0], input[1], input[2], T::one()]),
        ])
    }
}

impl Rotation for Mat4<f32> {
    type Input = Vec3<f32>;
    fn rotation(input: Self::Input) -> Self {
        let (sx, cx) = input[0].sin_cos();
        let (sy, cy) = input[1].sin_cos();
        let (sz, cz) = input[2].sin_cos();
        Self([
            Vec4([cy * cx, cy * sx, -sy, 0.0]),
            Vec4([sz * sy * cx - cz * sx, sz * sy * sx + cz * cx, sz * cy, 0.0]),
            Vec4([cz * sy * cx + sz * sx, cz * sy * sx - sz * cx, cz * cy, 0.0]),
            Vec4([0.0, 0.0, 0.0, 1.0]),
        ])
    }
}

impl<T> std::ops::Index<usize> for Mat4<T> {
    type Output = Vec4<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Mat4<T> {
    fn index_mut(&mut self, index: usize) -> &mut Vec4<T> {
        &mut self.0[index]
    }
}

impl<T> std::ops::Mul for Mat4<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T>,
{
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Mat4([
            Vec4([
                self[0][0] * rhs[0][0]
                    + self[1][0] * rhs[0][1]
                    + self[2][0] * rhs[0][2]
                    + self[3][0] * rhs[0][3],
                self[0][1] * rhs[0][0]
                    + self[1][1] * rhs[0][1]
                    + self[2][1] * rhs[0][2]
                    + self[3][1] * rhs[0][3],
                self[0][2] * rhs[0][0]
                    + self[1][2] * rhs[0][1]
                    + self[2][2] * rhs[0][2]
                    + self[3][2] * rhs[0][3],
                self[0][3] * rhs[0][0]
                    + self[1][3] * rhs[0][1]
                    + self[2][3] * rhs[0][2]
                    + self[3][3] * rhs[0][3],
            ]),
            Vec4([
                self[0][0] * rhs[1][0]
                    + self[1][0] * rhs[1][1]
                    + self[2][0] * rhs[1][2]
                    + self[3][0] * rhs[1][3],
                self[0][1] * rhs[1][0]
                    + self[1][1] * rhs[1][1]
                    + self[2][1] * rhs[1][2]
                    + self[3][1] * rhs[1][3],
                self[0][2] * rhs[1][0]
                    + self[1][2] * rhs[1][1]
                    + self[2][2] * rhs[1][2]
                    + self[3][2] * rhs[1][3],
                self[0][3] * rhs[1][0]
                    + self[1][3] * rhs[1][1]
                    + self[2][3] * rhs[1][2]
                    + self[3][3] * rhs[1][3],
            ]),
            Vec4([
                self[0][0] * rhs[2][0]
                    + self[1][0] * rhs[2][1]
                    + self[2][0] * rhs[2][2]
                    + self[3][0] * rhs[2][3],
                self[0][1] * rhs[2][0]
                    + self[1][1] * rhs[2][1]
                    + self[2][1] * rhs[2][2]
                    + self[3][1] * rhs[2][3],
                self[0][2] * rhs[2][0]
                    + self[1][2] * rhs[2][1]
                    + self[2][2] * rhs[2][2]
                    + self[3][2] * rhs[2][3],
                self[0][3] * rhs[2][0]
                    + self[1][3] * rhs[2][1]
                    + self[2][3] * rhs[2][2]
                    + self[3][3] * rhs[2][3],
            ]),
            Vec4([
                self[0][0] * rhs[3][0]
                    + self[1][0] * rhs[3][1]
                    + self[2][0] * rhs[3][2]
                    + self[3][0] * rhs[3][3],
                self[0][1] * rhs[3][0]
                    + self[1][1] * rhs[3][1]
                    + self[2][1] * rhs[3][2]
                    + self[3][1] * rhs[3][3],
                self[0][2] * rhs[3][0]
                    + self[1][2] * rhs[3][1]
                    + self[2][2] * rhs[3][2]
                    + self[3][2] * rhs[3][3],
                self[0][3] * rhs[3][0]
                    + self[1][3] * rhs[3][1]
                    + self[2][3] * rhs[3][2]
                    + self[3][3] * rhs[3][3],
            ]),
        ])
    }
}
