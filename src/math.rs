use core::ops::*;

macro_rules! impl_scalar_vec {
    (
        $(
            $v:vis struct $name:ident: ($($vname:ident),+) => [$ty:ty; $len:literal]
        ),+
    ) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Default)]
            $v struct $name {
                $(
                    $vname: $ty
                ),+
            }

            impl $name {
                #[inline]
                pub const fn new ($($vname: $ty),+) -> Self {
                    Self {
                        $($vname),+
                    }
                }

                #[inline]
                pub const fn splat (v: $ty) -> Self {
                    Self {
                        $(
                            $vname: v
                        ),+
                    }
                }

                $(
                    #[inline]
                    pub fn $vname (self) -> $ty {
                        return self.$vname
                    }
                )*

                #[inline]
                pub fn sq_magn (self) -> $ty {
                    return self * self
                }

                #[inline]
                pub fn magn (self) -> $ty {
                    return <$ty>::sqrt(self.sq_magn());
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
                            $vname: self.$vname + rhs.$vname
                        ),+
                    }
                }
            }

            impl Mul for $name {
                type Output = $ty;

                #[inline]
                fn mul (self, rhs: Self) -> Self::Output {
                    let mut result = 0 as $ty;

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

            impl Rem<$ty> for $name {
                type Output = Self;

                #[inline]
                fn rem (self, rhs: $ty) -> Self::Output {
                    return Self {
                        $(
                            $vname: self.$vname % rhs
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

            impl Rem<$name> for $ty {
                type Output = $name;

                #[inline]
                fn rem (self, rhs: $name) -> Self::Output {
                    return $name {
                        $(
                            $vname: self % rhs.$vname
                        ),+
                    }
                }
            }
        )+
    };
}

cfg_if::cfg_if! {
    if #[cfg(target_feature = "simd128")] {
        #[cfg(target_arch = "wasm32")]
        use core::arch::wasm32::*;
        #[cfg(target_arch = "wasm64")]
        use core::arch::wasm64::*;
        #[cfg(target_arch = "wasm")]
        use core::arch::wasm::*;
    }
}

//#[cfg(target_feature = "simd128")]
pub struct Vec4f {
    inner: v128
}

//#[cfg(target_feature = "simd128")]
impl Vec4f {
    #[inline]
    pub const fn new (x: f32, y: f32, z: f32, w: f32) -> Self {
        return Self { inner: f32x4(x, y, z, w) }
    }

    #[inline]
    pub const fn splat (v: f32) -> Self {
        return Self { inner: f32x4_splat(v) }
    }

    #[inline]
    pub fn x (self) -> f32 {
        return f32x4_extract_lane::<0>(self.inner);
    }

    #[inline]
    pub fn y (self) -> f32 {
        return f32x4_extract_lane::<1>(self.inner);
    }

    #[inline]
    pub fn z (self) -> f32 {
        return f32x4_extract_lane::<2>(self.inner);
    }

    #[inline]
    pub fn w (self) -> f32 {
        return f32x4_extract_lane::<3>(self.inner);
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
        f32x4_
        return Self { inner: f32x4_add(self.inner, rhs.inner) }
    }
}

/*#[cfg(not(target_feature = "simd128"))]
impl_scalar_vec! {
    pub struct Vec2f: (x, y) => [f32; 2],
    pub struct Vec3f: (x, y, z) => [f32; 3],
    pub struct Vec4f: (x, y, z, w) => [f32; 4],

    pub struct Vec2d: (x, y) => [f64; 2],
    pub struct Vec3d: (x, y, z) => [f64; 3],
    pub struct Vec4d: (x, y, z, w) => [f64; 4]
}*/