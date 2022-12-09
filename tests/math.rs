use rand::random;
use rustww::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test};
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

macro_rules! impl_tests {
    ($($name:ty as [$ty:ty; $len:literal] => ($($var:ident),+)),+) => {
        #[wasm_bindgen_test]
        fn addition () {
            $(
                let alpha: $name = random();
                let beta: $name = random();
                let result = alpha + beta;
                $(
                    assert_eq!(result.$var(), alpha.$var() + beta.$var());
                )+
            )+
        }

        #[wasm_bindgen_test]
        fn subtraction () {
            $(
                let alpha: $name = random();
                let beta: $name = random();
                
                let result = alpha - beta;
                $(
                    assert_eq!(result.$var(), alpha.$var() - beta.$var());
                )+
            )+
        }

        #[wasm_bindgen_test]
        fn multiplication () {
            $(
                let alpha: $name = random();
                let beta: $ty = random();
                
                let result = alpha * beta;
                $(
                    assert_eq!(result.$var(), alpha.$var() * beta);
                )+
            )+
        }

        #[wasm_bindgen_test]
        fn dot () {
            $(
                let alpha: $name = random();
                let beta: $name = random();
                
                let result = alpha * beta;
                let expected = $(
                    (alpha.$var() * beta.$var()) +
                )+ 0.0;
    
                assert!(
                    <$ty>::abs(result - expected) <= <$ty>::EPSILON,
                    "{} dot: {result} v. {expected}",
                    stringify!($name),
                );
            )+
        }

        #[wasm_bindgen_test]
        fn squared_magnitude () {
            $(
                let alpha: $name = random();
                
                let result = alpha.sq_magn();
                let expected = $(
                    (alpha.$var() * alpha.$var()) +
                )+ 0.0;
    
                assert!(
                    <$ty>::abs(result - expected) <= <$ty>::EPSILON,
                    "{} squared magintude: {result} v. {expected}",
                    stringify!($name),
                );
            )+
        }

        #[wasm_bindgen_test]
        fn magnitude () {
            $(
                let alpha: $name = random();
                let result = alpha.magn();
                let expected = <$ty>::sqrt($(
                    (alpha.$var() * alpha.$var()) +
                )+ 0.0);
                assert!(
                    <$ty>::abs(result - expected) <= <$ty>::EPSILON,
                    "{} magnitude: {result} v. {expected}",
                    stringify!($name)
                );
            )+
        }

        #[wasm_bindgen_test]
        fn unit () {
            $(
                let alpha: $name = random();
                let result = alpha.unit();
                let expected = <$ty>::sqrt($(
                    (alpha.$var() * alpha.$var()) +
                )+ 0.0);
                $(
                    assert!(
                        <$ty>::abs(result.$var() - (alpha.$var() / expected)) <= <$ty>::EPSILON,
                        "{} unit: {} v. {}",
                        stringify!($name),
                        result.$var(),
                        alpha.$var() / expected
                    );
                )+
            )+
        }
    };
}

impl_tests! {
    //Vec2f as [f32; 2] => (x, y),
    Vec3f as [f32; 3] => (x, y, z),
    Vec4f as [f32; 4] => (x, y, z, w),

    Vec2d as [f64; 2] => (x, y)
    //Vec3d as [f64; 3] => (x, y, z)
    //Vec4d as [f64; 4] => (x, y, z, w)
}
