use std::ops::{Add, Mul};

use hotham::glam::DVec3;

/// Linear interpolator.
#[inline]
pub fn lerp<T>(a: T, b: T, t: f64) -> <<f64 as Mul<T>>::Output as std::ops::Add>::Output
where
    T: Mul<f64>,
    f64: Mul<T>,
    <T as Mul<f64>>::Output: Add<<f64 as Mul<T>>::Output>,
    <f64 as Mul<T>>::Output: Add,
{
    (1.0 - t) * a + t * b
}

/// Linear interpolator.
#[inline]
pub fn lerp32<T>(a: T, b: T, t: f32) -> <<f32 as Mul<T>>::Output as std::ops::Add>::Output
where
    T: Mul<f32>,
    f32: Mul<T>,
    <T as Mul<f32>>::Output: Add<<f32 as Mul<T>>::Output>,
    <f32 as Mul<T>>::Output: Add,
{
    (1.0 - t) * a + t * b
}

/// Linearly interpolates from `a` through `b` in `n` steps, returning the intermediate result at
/// each step.
#[inline]
pub fn linspace<T>(
    a: T,
    b: T,
    n: usize,
) -> impl Iterator<Item = <<f64 as Mul<T>>::Output as std::ops::Add>::Output>
where
    T: Copy + Mul<f64>,
    f64: Mul<T>,
    <T as Mul<f64>>::Output: Add<<f64 as Mul<T>>::Output>,
    <f64 as Mul<T>>::Output: Add,
{
    (0..n).map(move |t| lerp(a, b, t as f64 / (n - 1).max(1) as f64))
}

/// Given two 3D vectors `from` and `to`, linearly interpolates between them in `n` steps along
/// the three axes, returning the intermediate result at each step.
pub fn grid(
    from: DVec3,
    to: DVec3,
    nx: usize,
    ny: usize,
    nz: usize,
) -> impl Iterator<Item = DVec3> {
    linspace(from.z, to.z, nz).flat_map(move |z| {
        linspace(from.y, to.y, ny)
            .flat_map(move |y| linspace(from.x, to.x, nx).map(move |x| (x, y, z).into()))
    })
}
