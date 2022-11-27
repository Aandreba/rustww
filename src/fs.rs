use js_sys::Function;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = requestFileSystem)]
    fn request_file_system (this: &web_sys::Window) -> Option<Function>;
    #[wasm_bindgen(js_namespace = window, js_name = webkitRequestFileSystem)]
    fn webkit_request_file_system (this: &web_sys::Window) -> Option<Function>;
}

thread_local! {
    static FILE_SYSTEM: JsValue = todo!()
}