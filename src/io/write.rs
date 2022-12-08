use js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use crate::Result;

/// A rustfull wrapper arround a JavaScript [`WriteableStream`](web_sys::WritableStream)
pub struct JsWriteStream {
    #[allow(unused)]
    pub(crate) stream: web_sys::WritableStream,
    writer: Option<web_sys::WritableStreamDefaultWriter>,
}

impl JsWriteStream {
    /// Creates a new [`JsWriteStream`]
    #[inline]
    pub fn new<T: Into<web_sys::WritableStream>> (stream: T) -> Result<Self> {
        return Ok(Self { stream: stream.into(), writer: None })
    }

    /// Writes a chunk of bytes into the stream.
    #[inline]
    pub async fn write_chunk (&mut self, buf: &[u8]) -> Result<()> {
        let chunk = unsafe { Uint8Array::view(buf) };
        JsFuture::from(self.get_writer()?.write_with_chunk(&chunk)).await?;
        return Ok(())
    }

    /// Writes `byte` into the stream
    #[inline]
    pub async fn write_byte (&mut self, byte: u8) -> Result<()> {
        return self.write_chunk(core::slice::from_ref(&byte)).await
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