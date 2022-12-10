#![allow(unused)]

use std::{time::Duration, cell::Cell};
use futures::{TryStreamExt, StreamExt, join, AsyncReadExt};
use js_sys::Uint8Array;
use rustww::{notify::{Notification}, geo::Geolocation, orient::{Orientation, Motion}, math::*, battery::Battery, io::{JsReadStream, Request}, fs::File, task::spawn_catch_local, time::Interval, Result, log, prelude::JsWriteStream, println};
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsValue, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Response, Blob};

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
pub fn runner () -> Result<()> {
    test_interval()?;
    Ok(())
}

#[cfg(target_feature = "atomics")]
fn test_thread () {
    rustww::thread::spawn(move || {
        log("Hello world!");
    });
}

fn test_interval () -> Result<()> {
    spawn_catch_local(async move {
        let mut x = 0;
        let int = Interval::new(
            Duration::from_secs(1),
            move || {
                x += 1;
                log!(x);
            }
        )?;
    
        let _ = Interval::leak(int);
        Ok(())
    });

    Ok(())
} 

fn test_notification () {
    Notification::new("Hello world!")
        .fire_after(Duration::from_secs(1))
        .close_after(Duration::from_secs(10))
        .spawn();
}

fn test_geo () {
    wasm_bindgen_futures::spawn_local(async move {
        let geo = Geolocation::current().unwrap().await.unwrap();
        log(&format!("{geo:?}"));

        /*let mut watch = Geolocation::watch().unwrap().take(5);
        while let Some(geo) = watch.try_next().await.unwrap() {
            log(&format!("{geo:?}"));
        }*/

        /*let mut watch = Geolocation::watch_send().unwrap();
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }*/
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

        /*let mut watch = Orientation::watch_send().unwrap().take(100);
        while let Some(geo) = watch.next().await {
            log(&format!("{geo:?}"));
        }*/
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
        let battery = Battery::new().await.unwrap();
        let mut level = battery.watch_charging().unwrap();

        let alpha = async {
            while let Some(level) = level.next().await {
                log(&format!("{level:?}"));
            }
        };

        let beta = async move {
            loop {
                // sleep(Duration::from_secs(1)).await;
                //let snapshot = battery.snapshot();
                //log(&format!("{snapshot:?}"));
            }
        };

        let _ = join! { alpha, beta };
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

fn test_fetch_and_read () {
    wasm_bindgen_futures::spawn_local(async move {
        let fetch = Request::get("index.html").await.unwrap();
        let text = fetch.bytes();

        //let text = fetch.body().unwrap().unwrap().read_remaining().await.unwrap();
        //let text = String::from_utf8(text).unwrap();
    });
}

fn test_fs () -> Result<()> {
    let elem = window()
        .unwrap()
        .document()
        .unwrap()
        .create_element_with_str("button", "hello")
        .unwrap();

    let f = Closure::<dyn FnMut()>::new(move || {
        spawn_catch_local(async move {
            let mut file = File::from_picker()
                .await?
                .next()
                .unwrap();

            let meta = file.reader()
                .await?
                .read_remaining()
                .await?;

            //unsafe { ::web_sys::console::log_1(&Uint8Array::view(&meta)) };
            Ok(())
        })
    });

    elem.add_event_listener_with_callback("click", f.into_js_value().unchecked_ref())?;

    window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_with_node_1(&elem)?;

    return Ok(())
}

fn test_writer () {
    spawn_catch_local(async move {
        let mut vec = Vec::<u8>::new();
        let mut writer: JsWriteStream<'_, Uint8Array> = JsWriteStream::custom()
            .write(|chunk: Uint8Array, con| {
                let len = chunk.length() as usize;
                vec.reserve(vec.len() + len);
                unsafe {
                    chunk.raw_copy_to_ptr(vec.as_mut_ptr().add(vec.len()));
                    vec.set_len(vec.len() + len);
                };
                Ok(())
            })
            .build()?;


        writer.write_slice(&[1, 2, 3]).await;
        drop(writer);

        println!("{vec:?}");
        Ok(())
    });
}