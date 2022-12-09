use core::ops::*;

#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;
#[cfg(target_arch = "wasm64")]
use core::arch::wasm64::*;
#[cfg(target_arch = "wasm")]
use core::arch::wasm::*;
use std::fmt::Debug;

/// Euclidian vector of 4 `f32` values
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vec4f {
    pub(crate) inner: v128
}

impl Vec4f {
    #[doc = concat!("Creates a new [`Vec4f`]")]
    #[inline]
    pub const fn new (x: f32, y: f32, z: f32, w: f32) -> Self {
        return Self { inner: f32x4(x, y, z, w) }
    }

    #[doc = concat!("Creates a new [`Vec4f`] by expanding `v` into every lane")]
    #[inline]
    pub fn splat (v: f32) -> Self {
        return Self { inner: f32x4_splat(v) }
    }

    #[doc = concat!("Returns the `x` component of the vector")]
    #[inline]
    pub fn x (self) -> f32 {
        return f32x4_extract_lane::<0>(self.inner);
    }

    #[doc = concat!("Returns the `y` component of the vector")]
    #[inline]
    pub fn y (self) -> f32 {
        return f32x4_extract_lane::<1>(self.inner);
    }

    #[doc = concat!("Returns the `z` component of the vector")]
    #[inline]
    pub fn z (self) -> f32 {
        return f32x4_extract_lane::<2>(self.inner);
    }

    #[doc = concat!("Returns the `w` component of the vector")]
    #[inline]
    pub fn w (self) -> f32 {
        return f32x4_extract_lane::<3>(self.inner);
    }

    /// Calculates the dot product between the vectors
    #[inline]
    pub fn dot (self, rhs: Self) -> f32 {
        return self * rhs
    }

    /// Calculates the squared magnitude of the vector
    #[inline]
    pub fn sq_magn (self) -> f32 {
        return self * self
    }

    /// Calculates the magnitude of the vector
    #[inline]
    pub fn magn (self) -> f32 {
        return f32::sqrt(self.sq_magn());
    }

    /// Calculates the unit vector
    #[inline]
    pub fn unit (self) -> Self {
        return self / self.magn()
    }
}

impl Add for Vec4f {
    type Output = Self;

    #[inline]
    fn add (self, rhs: Self) -> Self::Output {
        return Self { inner: f32x4_add(self.inner, rhs.inner) }
    }
}

impl Sub for Vec4f {
    type Output = Self;

    #[inline]
    fn sub (self, rhs: Self) -> Self::Output {
        return Self { inner: f32x4_sub(self.inner, rhs.inner) }
    }
}

impl Mul for Vec4f {
    type Output = f32;

    #[inline]
    fn mul (self, rhs: Self) -> Self::Output {
        let mul = f32x4_mul(self.inner, rhs.inner);
        return f32x4_sum(mul)
    }
}

impl Mul<f32> for Vec4f {
    type Output = Self;

    #[inline]
    fn mul (self, rhs: f32) -> Self::Output {
        return Self { inner: f32x4_mul(self.inner, f32x4_splat(rhs)) }
    }
}

impl Div<f32> for Vec4f {
    type Output = Self;

    #[inline]
    fn div (self, rhs: f32) -> Self::Output {
        return Self { inner: f32x4_div(self.inner, f32x4_splat(rhs)) }
    }
}

impl Mul<Vec4f> for f32 {
    type Output = Vec4f;

    #[inline]
    fn mul(self, rhs: Vec4f) -> Self::Output {
        return Vec4f { inner: f32x4_mul(f32x4_splat(self), rhs.inner) }
    }
}

impl Div<Vec4f> for f32 {
    type Output = Vec4f;

    #[inline]
    fn div(self, rhs: Vec4f) -> Self::Output {
        return Vec4f { inner: f32x4_div(f32x4_splat(self), rhs.inner) }
    }
}

impl PartialEq for Vec4f {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        !v128_any_true(
            v128_not(
                f32x4_eq(self.inner, other.inner)
            )
        )
    }
}

impl Default for Vec4f {
    #[inline]
    fn default() -> Self {
        Self::splat(Default::default())
    }
}

impl Debug for Vec4f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vec4f")
            .field("x", &self.x())
            .field("y", &self.y())
            .field("z", &self.z())
            .field("w", &self.w())
            .finish()
    }
}

/// Euclidian vector of 2 `f64` values
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vec2d {
    pub(crate) inner: v128
}

impl Vec2d {
    #[doc = concat!("Creates a new [`Vec2d`]")]
    #[inline]
    pub const fn new (x: f64, y: f64) -> Self {
        return Self { inner: f64x2(x, y) }
    }

    #[doc = concat!("Creates a new [`Vec2d`] by expanding `v` into every lane")]
    #[inline]
    pub fn splat (v: f64) -> Self {
        return Self { inner: f64x2_splat(v) }
    }

    #[doc = concat!("Returns the `x` component of the vector")]
    #[inline]
    pub fn x (self) -> f64 {
        return f64x2_extract_lane::<0>(self.inner);
    }

    #[doc = concat!("Returns the `y` component of the vector")]
    #[inline]
    pub fn y (self) -> f64 {
        return f64x2_extract_lane::<1>(self.inner);
    }

    /// Calculates the dot product between the vectors
    #[inline]
    pub fn dot (self, rhs: Self) -> f64 {
        return self * rhs
    }

    /// Calculates the squared magnitude of the vector
    #[inline]
    pub fn sq_magn (self) -> f64 {
        return self * self
    }

    /// Calculates the magnitude of the vector
    #[inline]
    pub fn magn (self) -> f64 {
        return f64::sqrt(self.sq_magn());
    }

    /// Calculates the unit vector
    #[inline]
    pub fn unit (self) -> Self {
        return self / self.magn()
    }
}

impl Add for Vec2d {
    type Output = Self;

    #[inline]
    fn add (self, rhs: Self) -> Self::Output {
        return Self { inner: f64x2_add(self.inner, rhs.inner) }
    }
}

impl Sub for Vec2d {
    type Output = Self;

    #[inline]
    fn sub (self, rhs: Self) -> Self::Output {
        return Self { inner: f64x2_sub(self.inner, rhs.inner) }
    }
}

impl Mul for Vec2d {
    type Output = f64;

    #[inline]
    fn mul (self, rhs: Self) -> Self::Output {
        let mul = f64x2_mul(self.inner, rhs.inner);
        return f64x2_sum(mul)
    }
}

impl Mul<f64> for Vec2d {
    type Output = Self;

    #[inline]
    fn mul (self, rhs: f64) -> Self::Output {
        return Self { inner: f64x2_mul(self.inner, f64x2_splat(rhs)) }
    }
}

impl Div<f64> for Vec2d {
    type Output = Self;

    #[inline]
    fn div (self, rhs: f64) -> Self::Output {
        return Self { inner: f64x2_div(self.inner, f64x2_splat(rhs)) }
    }
}

impl Mul<Vec2d> for f64 {
    type Output = Vec2d;

    #[inline]
    fn mul(self, rhs: Vec2d) -> Self::Output {
        return Vec2d { inner: f64x2_mul(f64x2_splat(self), rhs.inner) }
    }
}

impl Div<Vec2d> for f64 {
    type Output = Vec2d;

    #[inline]
    fn div(self, rhs: Vec2d) -> Self::Output {
        return Vec2d { inner: f64x2_div(f64x2_splat(self), rhs.inner) }
    }
}

impl PartialEq for Vec2d {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        !v128_any_true(
            v128_not(
                f64x2_eq(self.inner, other.inner)
            )
        )
    }
}

impl Default for Vec2d {
    #[inline]
    fn default() -> Self {
        Self::splat(Default::default())
    }
}

impl Debug for Vec2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vec2d")
            .field("x", &self.x())
            .field("y", &self.y())
            .finish()
    }
}

#[cfg(target_feature = "simd128")]
#[inline]
fn f32x4_sum (v: v128) -> f32 {
    // v = [ D C | B A ]
    let mut shuf = i32x4_shuffle::<1, 0, 3, 2>(v, v); // [ C D | A B ]
    let mut sums = f32x4_add(v, shuf); // sums = [ D+C C+D | B+A A+B ]
    shuf = i32x4_shuffle::<0, 1, 4, 5>(shuf, sums); //  [ C D | D+C C+D ]
    sums = f32x4_add(sums, shuf);
    return f32x4_extract_lane::<3>(sums);
}

#[cfg(target_feature = "simd128")]
#[inline]
fn f64x2_sum (v: v128) -> f64 {
    return f64x2_extract_lane::<0>(v) + f64x2_extract_lane::<1>(v);
}