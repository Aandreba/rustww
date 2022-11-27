use crate::Result;

pub struct JsWriteStream {
    inner: web_sys::WritableStreamDefaultWriter,
}

impl JsWriteStream {
    #[inline]
    pub fn new (writer: web_sys::WritableStream) -> Result<Self> {
        let inner = writer.get_writer()?;
        return Ok(Self { inner })
    }
}

/*#[async_trait_impl]
impl AsyncWrite for JsWriteStream {
    #[inline]
    async fn write<'a> (&'a mut self, buf: &'a [u8]) -> Result<usize> {
        let chunk = unsafe { Uint8Array::view(buf) };
        let _ = JsFuture::from(self.inner.write_with_chunk(&chunk)).await?;
        return Ok(buf.len());
    }
}*/

impl Drop for JsWriteStream {
    #[inline]
    fn drop(&mut self) {
        self.inner.release_lock()
    }
}