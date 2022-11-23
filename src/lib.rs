#![feature(new_uninit, ptr_metadata, is_some_and, iter_intersperse, const_fn_floating_point_arithmetic, concat_idents, const_trait_impl, core_intrinsics)]
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
extern crate wasm_thread;

/// Web Worker threads (from the [`wasm_thread`](https://github.com/chemicstry/wasm_thread) crate).
pub mod thread {
    use std::{time::Duration, intrinsics::unlikely};
    use js_sys::{Promise, Function};
    use wasm_bindgen::JsValue;
    use crate::{Result};
    
    #[docfg::docfg(target_feature = "atomics")]
    pub use wasm_thread::*;

    #[inline]
    pub async fn sleep (dur: Duration) -> Result<()> {
        return wasm_bindgen_futures::JsFuture::from(sleep_promise(dur)).await.map(|_| ())
    }

    pub fn sleep_promise (dur: Duration) -> Promise {
        const MAX: u128 = i32::MAX as u128;
        let millis = dur.as_millis();

        let mut f = |resolve: Function, reject: Function| {
            if unlikely(millis > MAX) {
                match reject.call1(&JsValue::UNDEFINED, &JsValue::from_str("Duration overflow")) {
                    Ok(_) => {},
                    Err(e) => {
                        drop(resolve);
                        drop(reject);
                        wasm_bindgen::throw_val(e)
                    }
                }
            }

            match web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis as i32) {
                Ok(_) => {},
                Err(e) => match reject.call1(&JsValue::UNDEFINED, &e) {
                    Ok(_) => {},
                    Err(e) => {
                        drop(resolve);
                        drop(reject);
                        wasm_bindgen::throw_val(e);
                    }
                }
            }
        };

        return Promise::new(&mut f);
    }
}

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

/// Sendable
pub mod send;

mod utils;