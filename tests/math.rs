#![feature(concat_idents)]

use rand::random;
use rustww::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

macro_rules! impl_tests {
    ($($name:ty as [$ty:ty; $len:literal] => ($($var:ident),+)),+) => {
        $(
            #[wasm_bindgen_test]
            fn addition () {
                let [$(concat_idents!($var, _1)),+] = random::<[$ty; $len]>();
                let [$(concat_idents!($var, _2)),+] = random::<[$ty; $len]>();
                
                let result = <$name>::new($(concat_idents!($var, _1)),+) + $name::new($( concat_idents!($var, _2) ),+);
                $(
                    assert_eq!(result.$var(), concat_idents!($var, _1) + concat_idents!($var, _2))
                )+
            }

            /*#[wasm_bindgen_test]
            fn subtraction () {
                let [x1, y1] = random::<[f32; 2]>();
                let [x2, y2] = random::<[f32; 2]>();
                
                let result = Vec2f::new(x1, y1) - Vec2f::new(x2, y2);
                assert_eq!(result.x(), x1 - x2);
                assert_eq!(result.y(), y1 - y2);
            }

            #[wasm_bindgen_test]
            fn multiplication () {
                let [x1, y1] = random::<[f32; 2]>();
                let x2 = random::<f32>();
                
                let result = Vec2f::new(x1, y1) * x2;
                assert_eq!(result.x(), x1 * x2);
                assert_eq!(result.y(), y1 * x2);
            }

            #[wasm_bindgen_test]
            fn dot () {
                let [x1, y1] = random::<[f32; 2]>();
                let [x2, y2] = random::<[f32; 2]>();
                
                let result = Vec2f::new(x1, y1) * Vec2f::new(x2, y2);
                assert!(result - ((x1 * x2) + (y1 * y2)) <= f32::EPSILON);
            }

            #[wasm_bindgen_test]
            fn squared_magnitude () {
                let [x, y] = random::<[f32; 2]>();
                let result = Vec2f::new(x, y).sq_magn();
                assert_eq!(result, x * x + y * y);
            }

            #[wasm_bindgen_test]
            fn magnitude () {
                let [x, y] = random::<[f32; 2]>();
                let result = Vec2f::new(x, y).magn();
                assert_eq!(result, f32::sqrt(x * x + y * y));
            }

            #[wasm_bindgen_test]
            fn unit () {
                let [x, y] = random::<[f32; 2]>();
                let result = Vec2f::new(x, y).unit();
                assert_eq!(result.x(), x / f32::sqrt(x * x + y * y));
                assert_eq!(result.y(), y / f32::sqrt(x * x + y * y));
            }*/
        )+
    };
}

impl_tests! {
    Vec2f as [f32; 2] => (x, y)
}
