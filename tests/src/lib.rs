use std::time::Duration;

use rustww::{thread::spawn, notify::Notification};
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
    test_notification();
}

fn test_thread () {
    spawn(move || {
        log("Hello world!");
    });
}

fn test_notification () {
    Notification::new("Hello world!")
        .fire_after(Duration::from_secs(1))
        .close_after(Duration::from_secs(10))
        .spawn();

}