pub trait Zero {
    const ZERO: Self;
}

pub trait One {
    const ONE: Self;
}

pub trait Identity {
    const IDENTITY: Self;
}

impl Zero for f32 {
    const ZERO: f32 = 0.0;
}

impl Zero for f64 {
    const ZERO: f64 = 0.0;
}

impl One for f32 {
    const ONE: f32 = 1.0;
}

impl One for f64 {
    const ONE: f64 = 1.0;
}
