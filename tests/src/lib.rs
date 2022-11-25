use std::time::Duration;
use futures::{TryStreamExt, StreamExt};
use rustww::{thread::spawn, notify::{Notification}, geo::Geolocation, orient::{Orientation, Motion}, math::*, battery::Battery};
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
    test_battery();
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

        /*let mut watch = Geolocation::watch().unwrap().take(5);
        while let Some(geo) = watch.try_next().await.unwrap() {
            log(&format!("{geo:?}"));
        }*/

        let mut watch = Geolocation::watch_send().unwrap();
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }
    });
}

fn test_orientation () {
    wasm_bindgen_futures::spawn_local(async move {
        let orientation = Orientation::current().await.unwrap();
        log(&format!("{orientation:?}"));

        /*let mut watch = Orientation::watch().unwrap();
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }*/

        let mut watch = Orientation::watch_send().unwrap().take(100);
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }
    });
}

fn test_motion () {
    wasm_bindgen_futures::spawn_local(async move {
        let motion = Motion::current().await.unwrap();
        log(&format!("{motion:?}"));

        let mut watch = Motion::watch().unwrap();
        while let Some(motion) = watch.next().await {
            log(&format!("{motion:?}"));
        }
    });
}

fn test_battery () {
    wasm_bindgen_futures::spawn_local(async move {
        let battery = Battery::new_snapshot().await;
        log(&format!("{battery:?}"))
    });
}

fn test_math () {
    let vec2 = Vec2d::new(1., 2.);
    log(&format!("{} = 5", vec2 * vec2));

    let vec3 = Vec3d::new(1., 2., 3.);
    log(&format!("{} = 14", vec3 * vec3));

    let vec4 = Vec4d::new(1., 2., 3., 4.);
    log(&format!("{} = 30", vec4 * vec4));
}