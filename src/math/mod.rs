use core::ops::*;

macro_rules! impl_scalar_vec {
    (
        $(
            $v:vis struct $name:ident: ($($vname:ident),+) => [$ty:ty; $len:literal]
        ),+
    ) => {
        $(
            #[doc = concat!("Euclidian vector of ", stringify!($len), " `", stringify!($ty), "` values")]
            #[derive(Debug, Clone, Copy, PartialEq, Default)]
            $v struct $name {
                $(
                    $vname: $ty
                ),+
            }

            impl $name {
                #[doc = concat!("Creates a new [`", stringify!($name), "`]")]
                #[inline]
                pub const fn new ($($vname: $ty),+) -> Self {
                    Self {
                        $($vname),+
                    }
                }

                #[doc = concat!("Creates a new [`", stringify!($name), "`] by expanding `v` into every lane")]
                #[inline]
                pub fn splat (v: $ty) -> Self {
                    Self {
                        $(
                            $vname: v
                        ),+
                    }
                }

                $(
                    #[doc = concat!("Returns the `", stringify!($vname), "` component of the vector")]
                    #[inline]
                    pub fn $vname (self) -> $ty {
                        return self.$vname
                    }
                )*

                /// Calculates the dot product between the vectors
                #[inline]
                pub fn dot (self, rhs: Self) -> $ty {
                    return self * rhs
                }

                /// Calculates the squared magnitude of the vector
                #[inline]
                pub fn sq_magn (self) -> $ty {
                    return self * self
                }

                /// Calculates the magnitude of the vector
                #[inline]
                pub fn magn (self) -> $ty {
                    return <$ty>::sqrt(self.sq_magn());
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
                    return Self {
                        $(
                            $vname: self.$vname + rhs.$vname
                        ),+
                    }
                }
            }

            impl Sub for $name {
                type Output = Self;

                #[inline]
                fn sub (self, rhs: Self) -> Self::Output {
                    return Self {
                        $(
                            $vname: self.$vname - rhs.$vname
                        ),+
                    }
                }
            }

            impl Mul for $name {
                type Output = $ty;

                #[inline]
                fn mul (self, rhs: Self) -> Self::Output {
                    let mut result = 0.0;

                    $(
                        result = <$ty>::mul_add(self.$vname, rhs.$vname, result);
                    )+

                    return result;
                }
            }

            impl Mul<$ty> for $name {
                type Output = Self;

                #[inline]
                fn mul (self, rhs: $ty) -> Self::Output {
                    return Self {
                        $(
                            $vname: self.$vname * rhs
                        ),+
                    }
                }
            }

            impl Div<$ty> for $name {
                type Output = Self;

                #[inline]
                fn div (self, rhs: $ty) -> Self::Output {
                    return Self {
                        $(
                            $vname: self.$vname / rhs
                        ),+
                    }
                }
            }

            impl Mul<$name> for $ty {
                type Output = $name;

                #[inline]
                fn mul (self, rhs: $name) -> Self::Output {
                    return $name {
                        $(
                            $vname: self * rhs.$vname
                        ),+
                    }
                }
            }

            impl Div<$name> for $ty {
                type Output = $name;

                #[inline]
                fn div (self, rhs: $name) -> Self::Output {
                    return $name {
                        $(
                            $vname: self / rhs.$vname
                        ),+
                    }
                }
            }
        )+
    };
}

#[cfg(target_feature = "simd128")]
flat_mod! { full, padded, extended }

#[cfg(not(target_feature = "simd128"))]
impl_scalar_vec! {
    pub struct Vec2f: (x, y) => [f32; 2],
    pub struct Vec3f: (x, y, z) => [f32; 3],
    pub struct Vec4f: (x, y, z, w) => [f32; 4],

    pub struct Vec2d: (x, y) => [f64; 2],
    pub struct Vec3d: (x, y, z) => [f64; 3],
    pub struct Vec4d: (x, y, z, w) => [f64; 4]
}