use js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use crate::Result;

pub struct JsWriteStream {
    #[allow(unused)]
    pub(crate) stream: web_sys::WritableStream,
    writer: Option<web_sys::WritableStreamDefaultWriter>,
}

impl JsWriteStream {
    #[inline]
    pub fn new<T: Into<web_sys::WritableStream>> (stream: T) -> Result<Self> {
        return Ok(Self { stream: stream.into(), writer: None })
    }

    #[inline]
    pub async fn write_chunk (&mut self, buf: &[u8]) -> Result<()> {
        let chunk = unsafe { Uint8Array::view(buf) };
        JsFuture::from(self.get_writer()?.write_with_chunk(&chunk)).await?;
        return Ok(())
    }

    #[inline]
    fn get_writer (&mut self) -> Result<&web_sys::WritableStreamDefaultWriter> {
        if let Some(ref writer) = self.writer {
            return Ok(writer)
        }
        
        let writer = self.stream.get_writer()?;
        self.writer = Some(writer);
        return Ok(unsafe { self.writer.as_ref().unwrap_unchecked() })
    }
}

impl Drop for JsWriteStream {
    #[inline]
    fn drop(&mut self) {
        if let Some(ref writer) = self.writer {
            writer.release_lock()
        }
        let _ = self.stream.close();
    }
}

// SAFETY: WritableStream is a [transferable object](https://developer.mozilla.org/en-US/docs/Glossary/Transferable_objects)
unsafe impl Send for JsWriteStream {}
unsafe impl Sync for JsWriteStream {}