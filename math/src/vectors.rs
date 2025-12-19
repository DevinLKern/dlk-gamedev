#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Vec2<T>(pub(crate) [T; 2]);

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

impl<T> std::ops::Index<usize> for Vec2<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Vec2<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Vec3<T>(pub(crate) [T; 3]);

impl<T: Default> Default for Vec3<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default()])
    }
}

impl<T> std::ops::Index<usize> for Vec3<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Vec3<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Vec3Std140<T>(pub(crate) [T; 4]);

impl<T: Default> Default for Vec3Std140<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default(), T::default()])
    }
}

impl<T> std::ops::Index<usize> for Vec3Std140<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Vec3Std140<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[allow(dead_code)]
#[repr(transparent)]
#[derive(Clone)]
pub struct Vec4<T>(pub(crate) [T; 4]);

impl<T: Default> Default for Vec4<T> {
    fn default() -> Self {
        Self([T::default(), T::default(), T::default(), T::default()])
    }
}

impl<T> std::ops::Index<usize> for Vec4<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Vec4<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
