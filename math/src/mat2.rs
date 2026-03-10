use crate::traits::{Identity, One, Zero};
use crate::vec2::Vec2;
use crate::vec3::Vec3;

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Mat2<T>([Vec2<T>; 2]);

impl<T> Mat2<T>
where
    T: Copy,
{
    #[inline]
    pub const fn from_rows(r0: Vec2<T>, r1: Vec2<T>) -> Self {
        Self([
            Vec2::new(r0.x(), r1.x()),
            Vec2::new(r0.y(), r1.y()),
        ])
    }
}

impl<T> Mat2<T> {
    #[inline]
    pub const fn from_cols(c0: Vec2<T>, c1: Vec2<T>) -> Self {
        Self([c0, c1])
    }
}

#[allow(dead_code)]
impl<T: Zero + Copy> Mat2<T> {
    fn scaling(s: Vec2<T>) -> Self {
        Self::from_cols(
            Vec2::new(s.x(), T::ZERO),
            Vec2::new(T::ZERO, s.y()),
        )
    }
}

impl<T: Zero> Zero for Mat2<T> {
    const ZERO: Self = Self::from_cols(Vec2::ZERO, Vec2::ZERO);
}

impl<T: Zero + One> Identity for Mat2<T> {
    const IDENTITY: Self = Self::from_cols(
        Vec2::new(T::ONE, T::ZERO),
        Vec2::new(T::ZERO, T::ONE),
    );
}

impl<T> Mat2<T>
where
    T: Copy,
{
    #[inline]
    pub const fn c0(&self) -> Vec2<T> {
        self.0[0]
    }
    #[inline]
    pub const fn c1(&self) -> Vec2<T> {
        self.0[1]
    }
    #[inline]
    pub const fn r0(&self) -> Vec2<T> {
        Vec2::new(self.c0().x(), self.c1().x())
    }
    #[inline]
    pub const fn r1(&self) -> Vec2<T> {
        Vec2::new(self.c0().y(), self.c1().y())
    }
}

impl<T> Mat2<T> {
    #[inline]
    pub const fn c0_mut(&mut self) -> &mut Vec2<T> {
        &mut self.0[0]
    }
    #[inline]
    pub const fn c1_mut(&mut self) -> &mut Vec2<T> {
        &mut self.0[1]
    }
}

impl Mat2<f32> {
    #[inline]
    pub const fn mul(&self, rhs: &Self) -> Mat2<f32> {
        let (r0, r1) = (self.r0(), self.r1());

        Self::from_cols(
            Vec2::new(r0.dot(rhs.c0()), r1.dot(rhs.c0())),
            Vec2::new(r0.dot(rhs.c1()), r1.dot(rhs.c1())),
        )
    }
    #[inline]
    pub const fn mul_vec(&self, v: Vec2<f32>) -> Vec2<f32> {
        self.c0()
            .scaled(v.x())
            .add(self.c1().scaled(v.y()))
    }
    #[inline]
    pub const fn transposed(&self) -> Self {
        Self::from_rows(self.c0(), self.c1())
    }
    #[inline]
    pub const fn determinant(&self) -> f32 {
        self.c0().x() * self.c1().x() - self.c0().y() * self.c1().y()
    }
}
