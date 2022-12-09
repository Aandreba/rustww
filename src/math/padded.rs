use rand::{distributions::*, prelude::*};
use super::full::*;
use core::ops::*;

#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;
#[cfg(target_arch = "wasm64")]
use core::arch::wasm64::*;
#[cfg(target_arch = "wasm")]
use core::arch::wasm::*;

macro_rules! impl_padded {
    (
        $(
            $v:vis struct $name:ident => $parent:ident as ($($field:ident),+): [$ty:ty; $len:literal] + ($($zero:literal),+)
        ),+
    ) => {
        $(
            #[doc = concat!("Euclidian vector of ", stringify!($len), " `", stringify!($ty), "` values")]
            #[derive(Clone, Copy, PartialEq, Default)]
            #[repr(transparent)]
            $v struct $name {
                inner: $parent
            }

            impl $name {
                #[doc = concat!("Creates a new [`", stringify!($name), "`]")]
                #[inline]
                pub const fn new ($($field: $ty),+) -> Self {
                    return Self { inner: <$parent>::new($($field,)+ $($zero),+) };
                }

                $(
                    #[doc = concat!("Returns the `", stringify!($field), "` component of the vector")]
                    #[inline]
                    pub fn $field (self) -> $ty {
                        return self.inner.$field()
                    }
                )+

                /// Calculates the dot product between the vectors
                #[inline]
                pub fn dot (self, rhs: Self) -> $ty {
                    return self * rhs
                }

                /// Calculates the squared magnitude of the vector
                #[inline]
                pub fn sq_magn (self) -> $ty {
                    return self.inner.sq_magn()
                }

                /// Calculates the magnitude of the vector
                #[inline]
                pub fn magn (self) -> $ty {
                    return self.inner.magn()
                }

                /// Calculates the unit vector
                #[inline]
                pub fn unit (self) -> Self {
                    return self / self.magn()
                }
            }

            impl Add for $name {
                type Output = Self;

                #[inline]
                fn add (self, rhs: Self) -> Self::Output {
                    return Self { inner: self.inner + rhs.inner }
                }
            }

            impl Sub for $name {
                type Output = Self;

                #[inline]
                fn sub (self, rhs: Self) -> Self::Output {
                    return Self { inner: self.inner - rhs.inner }
                }
            }

            impl Mul for $name {
                type Output = $ty;

                #[inline]
                fn mul (self, rhs: Self) -> Self::Output {
                    return self.inner * rhs.inner
                }
            }

            impl Mul<$ty> for $name {
                type Output = Self;

                #[inline]
                fn mul (self, rhs: $ty) -> Self::Output {
                    return Self { inner: self.inner * rhs }
                }
            }

            impl Div<$ty> for $name {
                type Output = Self;

                #[inline]
                fn div (self, rhs: $ty) -> Self::Output {
                    return Self { inner: self.inner / rhs }
                }
            }

            impl Mul<$name> for $ty {
                type Output = $name;

                #[inline]
                fn mul (self, rhs: $name) -> Self::Output {
                    return $name { inner: self * rhs.inner }
                }
            }

            impl Div<$name> for $ty {
                type Output = $name;

                #[inline]
                fn div (self, rhs: $name) -> Self::Output {
                    let mut inner: $parent = self / rhs.inner;
                    inner.inner = v128_and(inner.inner, <$name>::DIV_MASK);
                    return $name { inner }
                }
            }

            impl Distribution<$name> for Standard {
                #[inline]
                fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $name {
                    let mut inner: $parent = <Self as Distribution<$parent>>::sample(self, rng);
                    inner.inner = v128_and(inner.inner, <$name>::DIV_MASK);
                    return $name { inner }
                }
            }

            impl core::fmt::Debug for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!($name))
                        $(
                            .field(stringify!($field), &self.$field())
                        )+
                        .finish()
                }
            }
        )+
    }
}

impl_padded! {
    pub struct Vec2f => Vec4f as (x, y): [f32; 2] + (0., 0.),
    pub struct Vec3f => Vec4f as (x, y, z): [f32; 3] + (0.)
}

impl Vec2f {
    const DIV_MASK: v128 = unsafe {
        core::mem::transmute(((1u128 << (2 * u32::BITS)) - 1))
    };

    #[doc = concat!("Creates a new [`Vec2f`] by expanding `v` into every lane")]
    #[inline]
    pub fn splat (v: f32) -> Self {
        return Self::new(v, v);
    }
}

impl Vec3f {
    const DIV_MASK: v128 = unsafe {
        core::mem::transmute(((1u128 << (3 * u32::BITS)) - 1))
    };

    #[doc = concat!("Creates a new [`Vec3f`] by expanding `v` into every lane")]
    #[inline]
    pub fn splat (v: f32) -> Self {
        return Self::new(v, v, v);
    }
}