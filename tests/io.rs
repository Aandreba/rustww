use rustww::{prelude::{*, println}, io::Request};
use serde::Deserialize;
use wasm_bindgen_test::{wasm_bindgen_test};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn ip_json () -> Result<()> {
    #[derive(Deserialize)]
    struct Ip {
        ip: String
    }
    
    let Ip { ip } = Request::get("https://api.ipify.org?format=json")
        .await?
        .json::<Ip>()
        .await?;

    println!("{ip}");
    Ok(())
}

#[wasm_bindgen_test]
async fn ip_text () -> Result<()> {    
    let ip = Request::get("https://api.ipify.org?format=text")
        .await?
        .text()
        .await?;

    println!("{ip}");
    Ok(())
}

#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen_test]
async fn custom_read () -> Result<()> {
    use std::time::Duration;
    use futures::TryStreamExt;

    use rand::random;
    use rustww::{time::spawn_interval, log_js};
    use wasm_bindgen::JsValue;

    let mut reader = JsReadStream::custom()
        .start(|con| {
            let _ = spawn_interval(Duration::from_millis(500), move || {
                let chunk = random::<f64>();
                con.enqueue_with_chunk(&JsValue::from_f64(chunk)).unwrap();
            })?;
            Ok(())
        })
        .build()?;

    while let Some(next) = reader.try_next().await? {
        log_js!(next)
    }

    Ok(())
}