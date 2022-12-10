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

/*#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen_test]
async fn custom_read () -> Result<()> {
    use std::time::Duration;
    use js_sys::Uint8Array;
    use rand::random;
    use rustww::{log_js};

    let interval = std::cell::Cell::new(None);
    let mut reader: JsReadStream<Uint8Array> = JsReadStream::custom()
        .start(|con| {
            let int = Interval::new(Duration::from_millis(500), move || {
                let byte = Uint8Array::from(&[random()] as &[u8]);
                con.enqueue(&byte).unwrap();
            })?;

            interval.set(Some(int));
            Ok(())
        })
        .cancel(|_| {
            let _ = interval.take();
            Ok(())
        })
        .build()?;

    while let Some(next) = reader.read_chunk().await? {
        log_js!(next)
    }

    Ok(())
}*/

#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen_test]
async fn write_slice () -> Result<()> {
    use js_sys::Uint8Array;

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
}

#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen_test]
async fn write_slice_async () -> Result<()> {
    use std::rc::Rc;
    use js_sys::Uint8Array;
    use wasm_bindgen::__rt::WasmRefCell;

    let mut vec = Rc::new(WasmRefCell::new(Vec::<u8>::new()));
    let mut writer: JsWriteStream<'_, Uint8Array> = JsWriteStream::custom()
        .write_async(|chunk: Uint8Array, con| {
            let vec = vec.clone();
            async move {
                let mut vec = vec.borrow_mut();
                let len = chunk.length() as usize;
                vec.reserve(vec.len() + len);
                unsafe {
                    chunk.raw_copy_to_ptr(vec.as_mut_ptr().add(vec.len()));
                    vec.set_len(vec.len() + len);
                };
                Ok(())
            }
        })
        .build()?;

    writer.write_slice(&[4, 5, 6]).await;
    drop(writer);

    println!("{vec:?}");
    Ok(())
}

#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen_test]
fn write_drop () {
    // todo
}