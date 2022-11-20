#![feature(new_uninit, ptr_metadata)]

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
macro_rules! get_worker_script {
    ($path:literal) => {
        unsafe {
            static mut SCRIPT_URL: Option<String> = None;
    
            if let Some(url) = SCRIPT_URL.as_ref() {
                url.clone()
            } else {
                // If wasm bindgen shim url is not provided, try to obtain one automatically
                let wasm_bindgen_shim_url = $crate::get_wasm_bindgen_shim_script_path();
    
                // Generate script from template
                let template = include_str!($path);
                let script = template.replace("WASM_BINDGEN_SHIM_URL", &wasm_bindgen_shim_url);
    
                // Create url encoded blob
                let arr = js_sys::Array::new();
                arr.set(0, wasm_bindgen::JsValue::from_str(&script));
                let blob = web_sys::Blob::new_with_str_sequence(&arr).unwrap();
                let url = web_sys::Url::create_object_url_with_blob(
                    &blob
                        .slice_with_f64_and_f64_and_content_type(0.0, blob.size(), "text/javascript")
                        .unwrap(),
                )
                .unwrap();
                SCRIPT_URL = Some(url.clone());
    
                url
            }
        }
    }
}

#[allow(unused)]
pub(crate) type Result<T> = ::core::result::Result<T, ::wasm_bindgen::JsValue>;

#[cfg(all(target_family = "wasm", not(target_feature = "atomics")))]
compile_error!("The `atomics` target feature must be enabled. Try enabling it with `-C target-feature=+atomics`");

#[doc(hidden)]
pub extern crate wasm_thread;

/// Web Worker threads (from the [`wasm_thread`](https://github.com/chemicstry/wasm_thread) crate).
pub use wasm_thread as thread;

pub mod notify;