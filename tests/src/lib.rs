use std::time::Duration;
use futures::{TryStreamExt, StreamExt};
use rustww::{thread::spawn, notify::{Notification, get_permision}, geo::Geolocation, orient::Orientation};
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
    test_orientation();
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
        .spawn_local();
}

fn test_geo () {
    wasm_bindgen_futures::spawn_local(async move {
        let geo = Geolocation::current().unwrap().await.unwrap();
        log(&format!("{geo:?}"));

        let mut watch = Geolocation::watch().unwrap();
        while let Some(geo) = watch.try_next().await.unwrap() {
            log(&format!("{geo:?}"));
        }
    });
}

fn test_orientation () {
    wasm_bindgen_futures::spawn_local(async move {
        let orientation = Orientation::current().await.unwrap();
        log(&format!("{orientation:?}"));

        let mut watch = Orientation::watch().unwrap();
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }
    });
}