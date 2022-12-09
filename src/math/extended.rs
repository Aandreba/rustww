use core::ops::*;
use super::full::*;
use rand::{distributions::*, prelude::*};

macro_rules! impl_extended {
    (
        $(
            $v:vis struct $name:ident => [$ty:ty; $len:literal]: $parent1:ident as ($($field1:ident),+) + $parent2:ident as ($($prevfield2:ident as $field2:ident),+)
        ),+
    ) => {
        $(
            #[doc = concat!("Euclidian vector of ", stringify!($len), " `", stringify!($ty), "` values")]
            #[derive(Clone, Copy, PartialEq, Default)]
            $v struct $name {
                field1: $parent1,
                field2: $parent2
            }

            impl $name {
                #[doc = concat!("Creates a new [`", stringify!($name), "`]")]
                #[inline]
                pub const fn new ($($field1: $ty,)+ $($field2: $ty),+) -> Self {
                    Self {
                        field1: <$parent1>::new($($field1),+),
                        field2: <$parent2>::new($($field2),+)
                    }
                }

                #[doc = concat!("Creates a new [`", stringify!($name), "`] by expanding `v` into every lane")]
                #[inline]
                pub fn splat (v: $ty) -> Self {
                    Self {
                        field1: <$parent1>::splat(v),
                        field2: <$parent2>::splat(v)
                    }
                }

                $(
                    #[doc = concat!("Returns the `", stringify!($field1), "` component of the vector")]
                    #[inline]
                    pub fn $field1 (self) -> $ty {
                        return self.field1.$field1()
                    }
                )+

                $(
                    #[doc = concat!("Returns the `", stringify!($field2), "` component of the vector")]
                    #[inline]
                    pub fn $field2 (self) -> $ty {
                        return self.field2.$prevfield2()
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
                        field1: self.field1 + rhs.field1,
                        field2: self.field2 + rhs.field2
                    }
                }
            }

            impl Sub for $name {
                type Output = Self;

                #[inline]
                fn sub (self, rhs: Self) -> Self::Output {
                    return Self {
                        field1: self.field1 - rhs.field1,
                        field2: self.field2 - rhs.field2
                    }
                }
            }

            impl Mul for $name {
                type Output = $ty;

                #[inline]
                fn mul (self, rhs: Self) -> Self::Output {
                    return 
                        self.field1.dot(rhs.field1) + 
                        self.field2.dot(rhs.field2)
                }
            }

            impl Mul<$ty> for $name {
                type Output = Self;

                #[inline]
                fn mul (self, rhs: $ty) -> Self::Output {
                    return Self {
                        field1: self.field1 * rhs,
                        field2: self.field2 * rhs
                    }
                }
            }

            impl Div<$ty> for $name {
                type Output = Self;

                #[inline]
                fn div (self, rhs: $ty) -> Self::Output {
                    return Self {
                        field1: self.field1 / rhs,
                        field2: self.field2 / rhs
                    }
                }
            }

            impl Mul<$name> for $ty {
                type Output = $name;

                #[inline]
                fn mul (self, rhs: $name) -> Self::Output {
                    return $name {
                        field1: self * rhs.field1,
                        field2: self * rhs.field2
                    }
                }
            }

            impl Div<$name> for $ty {
                type Output = $name;

                #[inline]
                fn div (self, rhs: $name) -> Self::Output {
                    return $name {
                        field1: self / rhs.field1,
                        field2: self / rhs.field2
                    }
                }
            }

            impl Distribution<$name> for Standard {
                #[inline]
                fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $name {
                    return $name {
                        field1: self.sample(rng),
                        field2: self.sample(rng)
                    }
                }
            }

            impl core::fmt::Debug for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!($name))
                        $(
                            .field(stringify!($field1), &self.$field1())
                        )+

                        $(
                            .field(stringify!($field2), &self.$field2())
                        )+

                        .finish()
                }
            }
        )+ 
    };
}

impl_extended! {
    pub struct Vec3d => [f64;3]: Vec2d as (x, y) + f64 as (x as z),
    pub struct Vec4d => [f64; 4]: Vec2d as (x, y) + Vec2d as (x as z, y as w)
}

#[const_trait]
trait Simdlike {
    fn new (v: Self) -> Self;
    fn splat (v: Self) -> Self;
    fn x (self) -> Self;
    fn dot (self, rhs: Self) -> Self;
}

macro_rules! impl_simdlike {
    ($($t:ty),+) => {
        $(
            impl const Simdlike for $t {
                #[inline(always)]
                fn new (v: Self) -> Self { v }
                #[inline(always)]
                fn splat (v: Self) -> Self { v }
                #[inline(always)]
                fn x (self) -> Self { self }
                #[inline(always)]
                fn dot (self, rhs: Self) -> Self { self * rhs }
            }  
        )+
    };
}

impl_simdlike! { f32, f64 }
