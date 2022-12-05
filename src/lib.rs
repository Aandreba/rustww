#![feature(new_uninit, ptr_metadata, get_mut_unchecked, is_some_and, let_chains, unboxed_closures, iter_intersperse, io_error_other, type_alias_impl_trait, const_fn_floating_point_arithmetic, concat_idents, const_trait_impl, core_intrinsics)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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
pub(crate) type Result<T> = ::core::result::Result<T, ::wasm_bindgen::JsValue>;

#[doc(hidden)]
pub extern crate wasm_thread;

/// Web Worker threads (from the [`wasm_thread`](https://github.com/chemicstry/wasm_thread) crate).
#[docfg::docfg(target_feature = "atomics")]
pub use wasm_thread as thread;
use web_sys::Window;

/// Task-related functionality
pub mod task;

/// Time-related functionality
pub mod time;

/// Math-related types
pub mod math;

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

mod utils;

#[inline]
pub(crate) fn window () -> Result<Window> {
    return ::web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window not found"));
}