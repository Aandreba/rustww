use rustww::thread::spawn;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log (s: &str);
}

#[wasm_bindgen(start)]
pub fn main () {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn runner () {
    spawn(move || {
        log("Hello world!");
    });
}
