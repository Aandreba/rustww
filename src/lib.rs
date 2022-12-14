#![feature(new_uninit, waker_getters, min_specialization, ptr_metadata, get_mut_unchecked, is_some_and, let_chains, unboxed_closures, iter_intersperse, io_error_other, type_alias_impl_trait, const_fn_floating_point_arithmetic, concat_idents, const_trait_impl, core_intrinsics)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "simd", not(all(target_family = "wasm", target_feature = "simd128"))))]
compile_error!("`simd` feature enabled without target feature `simd128`");

#[wasm_bindgen]
extern "C" {
    #[allow(unused_doc_comments)]
    #[doc(hidden)]
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn _log (s: &str);
}

#[doc(hidden)]
pub trait ArgsExt: Sized {
    fn into_str (self) -> Cow<'static, str>;
}

impl ArgsExt for std::fmt::Arguments<'_> {
    #[inline]
    fn into_str (self) -> Cow<'static, str> {
        return match self.as_str() {
            Some(x) => Cow::Borrowed(x),
            None => Cow::Owned(std::fmt::format(self))
        }
    }
}

/// Prints the formated arguments into the JavaScript console
#[macro_export]
macro_rules! println {
    ($($t:tt)*) => {{
        $crate::_log(&$crate::ArgsExt::into_str(::std::format_args!($($t)*)));
    }};
}

/// Logs the inserted values by serializing them into [`JsValue`](wasm_bindgen::JsValue)
#[macro_export]
macro_rules! log {
    ($($v:expr),+) => {{
        let values = $crate::js_sys::Array::new();
        $(
            let _ = values.push(&$crate::serde_wasm_bindgen::to_value(&$v).unwrap());
        )+
        $crate::web_sys::console::log(&values)
    }};
}

/// Logs the inserted JavaScript values
#[macro_export]
macro_rules! log_js {
    ($($v:expr),+) => {{
        let values = $crate::js_sys::Array::new();
        $(
            let _ = values.push(&$v);
        )+
        $crate::web_sys::console::log(&values)
    }};
}

#[allow(unused)]
macro_rules! throw {
    ($($tt:tt)*) => {
        Err(
            {
                let args = ::std::format_args!($($tt)*);
                match args.as_str() {
                    Some(x) => ::wasm_bindgen::JsValue::from_str(x),
                    None => ::wasm_bindgen::JsValue::from_str(&::std::string::ToString::to_string(&args))
                }
            }
        )
    };
}

#[allow(unused)]
macro_rules! flat_mod {
    ($($i:ident),+) => {
        $(
            mod $i;
            pub use $i::*;
        )+
    };
}

#[allow(unused)]
pub type Result<T> = ::core::result::Result<T, ::wasm_bindgen::JsValue>;

#[doc(hidden)]
pub extern crate wasm_thread;
#[doc(hidden)]
pub extern crate web_sys;
#[doc(hidden)]
pub extern crate js_sys;
#[doc(hidden)]
pub extern crate serde_wasm_bindgen;

use std::borrow::Cow;

use wasm_bindgen::prelude::wasm_bindgen;
/// Web Worker threads (from the [`wasm_thread`](https://github.com/chemicstry/wasm_thread) crate).
#[docfg::docfg(target_feature = "atomics")]
pub use wasm_thread as thread;

/// Context scope functionality
pub mod scope;

/// Task-related functionality
pub mod task;

/// Time-related functionality
pub mod time;

/// Math-related types
pub mod math;

/// Syncronization-related types
pub mod sync;

/// Notification API
pub mod notify;

/// Geolocation API
pub mod geo;

/// Device Orientation API
pub mod orient;

/// Battery API
pub mod battery;

/// File API
pub mod fs;

/// Input-Output
pub mod io;

/// Local Storage
pub mod storage;

/// Various utils
pub mod utils;

/// Async runtime for WASM + JS
#[cfg(any(test, target_feature = "atomics"))]
#[cfg_attr(docsrs, doc(cfg(target_feature = "atomics")))]
pub mod runtime;

/// Prelude
pub mod prelude {
    pub use crate::{Result, log, println};
    pub use crate::io::{JsReadStream, JsWriteStream};
    pub use crate::math::*;
    pub use crate::battery::{Battery, Snapshot, BatteryTime};
    pub use crate::fs::{File};
    pub use crate::geo::{Geolocation};
    pub use crate::notify::Notification;
    pub use crate::orient::{Orientation, Motion, EulerAngles};
    pub use crate::storage::{Storage};
    pub use crate::task::spawn_local;
    pub use crate::time::{Interval, Timeout, sleep};
}