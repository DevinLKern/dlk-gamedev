pub(crate) trait Zero {
    fn zero() -> Self;
}

pub(crate) trait One {
    fn one() -> Self;
}

impl Zero for f32 {
    fn zero() -> Self {
        0.0
    }
}

impl Zero for f64 {
    fn zero() -> Self {
        0.0
    }
}

impl One for f32 {
    fn one() -> Self {
        1.0
    }
}

impl One for f64 {
    fn one() -> Self {
        1.0
    }
}
