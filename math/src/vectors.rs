use std::ops::{Add, Index, IndexMut, Mul, Sub};

pub trait Dot {
    type Output;
    fn dot(&self, other: &Self) -> Self::Output;
}

pub trait Cross {
    type Output;
    fn crossed(&self, other: &Self) -> Self::Output;
}

pub trait Scale {
    type Factor;
    fn scaled(&self, other: Self::Factor) -> Self;
}

pub trait Normalize {
    fn normalized(&self) -> Self;
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Vec2<T>(pub(crate) [T; 2]);

#[allow(dead_code)]
impl<T> Vec2<T> {
    fn new(a: T, b: T) -> Self {
        Vec2([a, b])
    }
}

impl<T: Default> Default for Vec2<T> {
    fn default() -> Self {
        Self([T::default(), T::default()])
    }
}

impl<T> Index<usize> for Vec2<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for Vec2<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T> Add for Vec2<T>
where
    T: Copy + Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self([self[0] + rhs[0], self[1] + rhs[1]])
    }
}

impl<T: Mul> Dot for Vec2<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = T;
    fn dot(&self, other: &Self) -> Self::Output {
        self[0] * other[0] + self[1] * other[1]
    }
}

impl<T> Cross for Vec2<T>
where
    T: Copy + Mul<Output = T> + Sub<Output = T>,
{
    type Output = T;
    fn crossed(&self, other: &Self) -> Self::Output {
        self[0] * other[1] - self[1] * other[0]
    }
}

impl<T> Scale for Vec2<T>
where
    T: Copy + Mul<Output = T>,
{
    type Factor = T;
    fn scaled(&self, other: Self::Factor) -> Self {
        Self([self[0] * other, self[1] * other])
    }
}

impl Normalize for Vec2<f32> {
    fn normalized(&self) -> Self {
        let s = self[0] * self[0] + self[1] * self[1];
        let s = s.sqrt();

        if s == 0.0 {
            return self.clone();
        }

        Self([self[0] / s, self[1] / s])
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Vec3<T>(pub(crate) [T; 3]);

impl<T> Vec3<T> {
    pub fn new(a: T, b: T, c: T) -> Self {
        Self([a, b, c])
    }
}

impl<T: Default> Default for Vec3<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default()])
    }
}

impl<T> Index<usize> for Vec3<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for Vec3<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T> Add for Vec3<T>
where
    T: Copy + Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self([self[0] + rhs[0], self[1] + rhs[1], self[2] + rhs[1]])
    }
}

impl<T: Mul> Dot for Vec3<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = T;
    fn dot(&self, other: &Self) -> Self::Output {
        self[0] * other[0] + self[1] * other[1] + self[2] * other[2]
    }
}

impl<T> Cross for Vec3<T>
where
    T: Copy + Mul<Output = T> + Sub<Output = T>,
{
    type Output = Self;
    fn crossed(&self, other: &Self) -> Self::Output {
        Self([
            self[1] * other[2] - other[1] * self[2],
            self[2] * other[0] - other[2] * self[0],
            self[0] * other[1] - other[0] * self[1],
        ])
    }
}

impl<T> Scale for Vec3<T>
where
    T: Copy + Mul<Output = T>,
{
    type Factor = T;
    fn scaled(&self, other: Self::Factor) -> Self {
        Self([self[0] * other, self[1] * other, self[2] * other])
    }
}

impl Normalize for Vec3<f32> {
    fn normalized(&self) -> Self {
        let s = self[0] * self[0] + self[1] * self[1] + self[2] * self[2];
        let s = s.sqrt();

        if s == 0.0 {
            return self.clone();
        }

        Self([self[0] / s, self[1] / s, self[2] / s])
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Vec3Std140<T>(pub(crate) [T; 4]);

impl<T: Default> Default for Vec3Std140<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default(), T::default()])
    }
}

impl<T> Index<usize> for Vec3Std140<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for Vec3Std140<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T: Mul> Dot for Vec3Std140<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = T;
    fn dot(&self, other: &Self) -> Self::Output {
        self[0] * other[0] + self[1] * other[1] + self[2] * other[2]
    }
}

impl<T> Cross for Vec3Std140<T>
where
    T: crate::numbers::Zero + Copy + Mul<Output = T> + Sub<Output = T>,
{
    type Output = Self;
    fn crossed(&self, other: &Self) -> Self::Output {
        Self([
            self[1] * other[2] - other[1] * self[2],
            self[2] * other[0] - other[2] * self[0],
            self[0] * other[1] - other[0] * self[1],
            T::zero(),
        ])
    }
}

impl Normalize for Vec3Std140<f32> {
    fn normalized(&self) -> Self {
        let s = self[0] * self[0] + self[1] * self[1] + self[2] * self[2];
        let s = s.sqrt();

        if s == 0.0 {
            return self.clone();
        }

        Self([self[0] / s, self[1] / s, self[2] / s, self[3]])
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Vec4<T>(pub(crate) [T; 4]);

impl<T> Vec4<T> {
    pub fn new(a: T, b: T, c: T, d: T) -> Self {
        Self([a, b, c, d])
    }
}

impl<T: Default> Default for Vec4<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default(), T::default()])
    }
}

impl<T> Index<usize> for Vec4<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for Vec4<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T: Mul> Dot for Vec4<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = T;
    fn dot(&self, other: &Self) -> Self::Output {
        self[0] * other[0] + self[1] * other[1] + self[2] * other[2] + self[3] * other[3]
    }
}
