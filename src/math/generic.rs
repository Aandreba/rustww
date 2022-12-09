use super::*;
use serde::*;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use rand::{distributions::*, prelude::*};

macro_rules! impl_generic {
    ($($name:ident as [$ty:ty; $len:literal] => ($($var:ident),+)),+) => {
        $(
            impl From<[$ty; $len]> for $name {
                #[inline]
                fn from ([$($var),+]: [$ty; $len]) -> Self {
                    return Self::new($($var),+)
                }
            }

            impl Distribution<$name> for Standard {
                #[inline]
                fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $name {
                    <$name>::new(
                        $(
                            <Self as Distribution<$ty>>::sample(self, rng)
                        ),+
                    )
                }
            }

            impl Serialize for $name {
                #[inline]
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
                    let mut serializer = serializer.serialize_seq(Some($len))?;
                    $(
                        serializer.serialize_element::<$ty>(&self.$var())?;
                    )+
                    return serializer.end()
                }
            }

            impl<'de> Deserialize<'de> for $name {
                #[inline]
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
                    struct Vis;
                    impl<'de> Visitor<'de> for Vis {
                        type Value = $name;
            
                        #[inline]
                        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                            f.write_str(concat!("an array of ", stringify!($len), " ", stringify!($ty)))
                        }
            
                        #[inline]
                        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: de::SeqAccess<'de>, {
                            return Ok(<$name>::new(
                                $(
                                    match seq.next_element::<$ty>()? {
                                        Some(x) => x,
                                        None => return Err(<A::Error as serde::de::Error>::custom(concat!("expected the ", stringify!($var)," value of an array of ", stringify!($len), " ", stringify!($ty)))),
                                    }
                                ),+
                            ))
                        }
                    }
            
                    return deserializer.deserialize_seq(Vis)
                }
            }
        )+
    };
}

impl_generic! {
    Vec2f as [f32; 2] => (x, y),
    Vec3f as [f32; 3] => (x, y, z),
    Vec4f as [f32; 4] => (x, y, z, w),

    Vec2d as [f64; 2] => (x, y),
    Vec3d as [f64; 3] => (x, y, z),
    Vec4d as [f64; 4] => (x, y, z, w)
}