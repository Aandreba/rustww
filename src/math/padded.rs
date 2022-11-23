use core::arch::wasm32::*;
use super::full::*;
use core::ops::*;

macro_rules! impl_padded {
    (
        $(
            $v:vis struct $name:ident => $parent:ident as ($($field:ident),+): [$ty:ty; $len:literal] + ($($zero:literal),+)
        ),+
    ) => {
        $(
            #[derive(Clone, Copy, PartialEq, Default)]
            #[repr(transparent)]
            $v struct $name {
                inner: $parent
            }

            impl $name {
                const DIV_MASK: v128 = unsafe {
                    Self::const_splat(core::mem::transmute(!0)).inner.inner
                };

                #[inline]
                pub const fn new ($($field: $ty),+) -> Self {
                    return Self { inner: <$parent>::new($($field,)+ $($zero),+) };
                }

                $(
                    #[inline]
                    pub fn $field (self) -> $ty {
                        return self.inner.$field()
                    }
                )+

                #[inline]
                pub fn dot (self, rhs: Self) -> $ty {
                    return self * rhs
                }

                #[inline]
                pub fn sq_magn (self) -> $ty {
                    return self.inner.sq_magn()
                }

                #[inline]
                pub fn magn (self) -> $ty {
                    return self.inner.magn()
                }

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
                    let div = (self / rhs.inner).inner;
                    return $name {
                        inner: $parent {
                            inner: v128_and(div, <$name>::DIV_MASK)
                        }
                    }
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
    #[inline]
    pub fn splat (v: f32) -> Self {
        return Self::new(v, v);
    }

    #[inline]
    const fn const_splat (v: f32) -> Self {
        return Self::new(v, v);
    }
}

impl Vec3f {
    #[inline]
    pub fn splat (v: f32) -> Self {
        return Self::new(v, v, v);
    }

    #[inline]
    const fn const_splat (v: f32) -> Self {
        return Self::new(v, v, v);
    }
}