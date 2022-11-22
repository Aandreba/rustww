#![feature(new_uninit, ptr_metadata, is_some_and, iter_intersperse, core_intrinsics)]

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
pub(crate) type Result<T> = ::core::result::Result<T, ::wasm_bindgen::JsValue>;

#[cfg(all(not(docsrs), target_family = "wasm", not(target_feature = "atomics")))]
compile_error!("The `atomics` target feature must be enabled. Try enabling it with `-C target-feature=+atomics`");

thread_local! {
    pub(crate) static WINDOW: web_sys::Window = web_sys::window().unwrap();
    pub(crate) static NAVIGATOR: web_sys::Navigator = WINDOW.with(|window| window.navigator());
}

#[doc(hidden)]
extern crate wasm_thread;

/// Web Worker threads (from the [`wasm_thread`](https://github.com/chemicstry/wasm_thread) crate).
pub mod thread {
    use std::{time::Duration, intrinsics::unlikely};
    use js_sys::{Promise, Function};
    use wasm_bindgen::JsValue;
    pub use wasm_thread::*;
    use crate::{WINDOW, Result};

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

            match WINDOW.with(|window| window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis as i32)) {
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

pub mod notify;
pub mod geo;
mod utils;