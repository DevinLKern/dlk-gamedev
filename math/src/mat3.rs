use crate::traits::{Identity, One, Zero};
use crate::vec3::Vec3;
use crate::vec4::Vec4;

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Mat3<T>([Vec3<T>; 3]);

impl<T> Mat3<T>
where
    T: Copy,
{
    #[inline]
    pub const fn from_rows(r0: Vec3<T>, r1: Vec3<T>, r2: Vec3<T>) -> Self {
        Self([
            Vec3::new(r0.x(), r1.x(), r2.x()),
            Vec3::new(r0.y(), r1.y(), r2.y()),
            Vec3::new(r0.z(), r1.z(), r2.z()),
        ])
    }
}

impl<T> Mat3<T> {
    #[inline]
    pub const fn from_cols(c0: Vec3<T>, c1: Vec3<T>, c2: Vec3<T>) -> Self {
        Self([c0, c1, c2])
    }
}

// This function is problomatic.
// impl Mat3<f32> {
//     #[inline]
//     pub fn rotation_euler_xyz(r: Vec3<f32>) -> Self {
//         let (sx, cx) = r.x().sin_cos();
//         let (sy, cy) = r.y().sin_cos();
//         let (sz, cz) = r.z().sin_cos();
//         Self::from_rows(
//             Vec3::new(cy * cx, cy * sx, -sy),
//             Vec3::new(sz * sy * cx - cz * sx, sz * sy * sx + cz * cx, sz * cy),
//             Vec3::new(cz * sy * cx + sz * sx, cz * sy * sx - sz * cx, cz * cy),
//         )
//     }
// }

#[allow(dead_code)]
impl<T: Zero + Copy> Mat3<T> {
    fn scaling(s: Vec3<T>) -> Self {
        Self::from_cols(
            Vec3::new(s.x(), T::ZERO, T::ZERO),
            Vec3::new(T::ZERO, s.y(), T::ZERO),
            Vec3::new(T::ZERO, T::ZERO, s.z()),
        )
    }
}

impl<T: Zero> Zero for Mat3<T> {
    const ZERO: Self = Self::from_cols(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
}

impl<T: Zero + One> Identity for Mat3<T> {
    const IDENTITY: Self = Self::from_cols(
        Vec3::new(T::ONE, T::ZERO, T::ZERO),
        Vec3::new(T::ZERO, T::ONE, T::ZERO),
        Vec3::new(T::ZERO, T::ZERO, T::ONE),
    );
}

impl<T> Mat3<T>
where
    T: Copy,
{
    #[inline]
    pub const fn c0(&self) -> Vec3<T> {
        self.0[0]
    }
    #[inline]
    pub const fn c1(&self) -> Vec3<T> {
        self.0[1]
    }
    #[inline]
    pub const fn c2(&self) -> Vec3<T> {
        self.0[2]
    }
    #[inline]
    pub const fn r0(&self) -> Vec3<T> {
        Vec3::new(self.c0().x(), self.c1().x(), self.c2().x())
    }
    #[inline]
    pub const fn r1(&self) -> Vec3<T> {
        Vec3::new(self.c0().y(), self.c1().y(), self.c2().y())
    }
    #[inline]
    pub const fn r2(&self) -> Vec3<T> {
        Vec3::new(self.c0().z(), self.c1().z(), self.c2().z())
    }
}

impl<T> Mat3<T> {
    #[inline]
    pub const fn c0_mut(&mut self) -> &mut Vec3<T> {
        &mut self.0[0]
    }
    #[inline]
    pub const fn c1_mut(&mut self) -> &mut Vec3<T> {
        &mut self.0[1]
    }
    #[inline]
    pub const fn c2_mut(&mut self) -> &mut Vec3<T> {
        &mut self.0[2]
    }
}

impl Mat3<f32> {
    #[inline]
    pub const fn mul(&self, rhs: &Self) -> Mat3<f32> {
        let (r0, r1, r2) = (self.r0(), self.r1(), self.r2());

        Self::from_cols(
            Vec3::new(r0.dot(rhs.c0()), r1.dot(rhs.c0()), r2.dot(rhs.c0())),
            Vec3::new(r0.dot(rhs.c1()), r1.dot(rhs.c1()), r2.dot(rhs.c1())),
            Vec3::new(r0.dot(rhs.c2()), r1.dot(rhs.c2()), r2.dot(rhs.c2())),
        )
    }
    #[inline]
    pub const fn mul_vec(&self, v: Vec3<f32>) -> Vec3<f32> {
        self.c0().scaled(v.x())
            .add(self.c1().scaled(v.y()))
            .add(self.c2().scaled(v.z()))
    }
    #[inline]
    pub const fn transposed(&self) -> Self {
        Self::from_rows(self.c0(), self.c1(), self.c2())
    }
}

impl<T: std::fmt::Display + Copy> std::fmt::Display for Mat3<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        write!(f, "{}", self.c0())?;
        write!(f, "{}", self.c1())?;
        write!(f, "{}", self.c2())?;
        write!(f, "]")
    }
}

impl<T: PartialEq + Copy> PartialEq for Mat3<T> {
    fn eq(&self, other: &Self) -> bool {
        self.c0() == other.c0() && self.c1() == other.c1() && self.c2() == other.c2()
    }
    fn ne(&self, other: &Self) -> bool {
        self.c0() != other.c0() || self.c1() != other.c1() || self.c2() != other.c2()
    }
}

impl<T> Mat3<T>
where
    T: Zero + One + Copy,
{
    pub const fn into_mat4(self) -> crate::mat4::Mat4<T> {
        crate::mat4::Mat4::from_rows(
            self.r0().into_vec4(),
            self.r1().into_vec4(),
            self.r2().into_vec4(),
            Vec4::ZERO,
        )
    }
}

#[cfg(test)]
mod test {
    use super::Mat3;
    use super::Vec3;

    #[test]
    fn multiplication_scaling() {
        let s = Mat3::scaling(Vec3::new(2.0, 3.0, 4.0));

        let v = Mat3::from_cols(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
        );

        let result = v.mul(&s);

        let expected = Mat3::from_cols(
            Vec3::new(2.0, 4.0, 6.0),
            Vec3::new(12.0, 15.0, 18.0),
            Vec3::new(28.0, 32.0, 36.0),
        );

        assert_eq!(result, expected);
    }

    #[test]
    fn multiplication_chained() {
        let a = Mat3::from_rows(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
        );
        let b = Mat3::from_rows(
            Vec3::new(10.0, 11.0, 12.0),
            Vec3::new(13.0, 14.0, 15.0),
            Vec3::new(16.0, 17.0, 18.0),
        );
        let c = Mat3::from_rows(
            Vec3::new(19.0, 20.0, 21.0),
            Vec3::new(22.0, 23.0, 24.0),
            Vec3::new(25.0, 26.0, 27.0),
        );

        let result = a.mul(&b).mul(&c);

        // this is equivalent to a * b * c
        let expected = Mat3::from_rows(
            Vec3::new(5976.0, 6246.0, 6516.0),
            Vec3::new(14346.0, 14994.0, 15642.0),
            Vec3::new(22716.0, 23742.0, 24768.0),
        );

        assert_eq!(result, expected);
    }
}
