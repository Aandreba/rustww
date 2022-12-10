use std::{marker::PhantomData, task::Poll};
use docfg::docfg;
use futures::{Future, FutureExt};
use js_sys::{Uint8Array, JsString};
use wasm_bindgen::{JsValue};
use wasm_bindgen_futures::JsFuture;
use crate::{Result, utils::{TypedArrayExt}};

/// A rustfull wrapper arround a JavaScript [`WriteableStream`](web_sys::WritableStream)
pub struct JsWriteStream<'a, T> {
    pub(crate) _stream: web_sys::WritableStream,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::WriteBuilder<'a, T>>,
    writer: Option<web_sys::WritableStreamDefaultWriter>,
    _phtm: PhantomData<&'a T>
}

impl<'a, T: AsRef<JsValue>> JsWriteStream<'a, T> {
    /// Returns a builder for a custom [`JsWriteStream`]
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::WriteBuilder<'a, T> where T: wasm_bindgen::JsCast {
        return super::builder::WriteBuilder::new()
    }

    /// Creates a new [`JsWriteStream`]
    #[inline]
    pub fn new<S: Into<web_sys::WritableStream>> (stream: S) -> Result<Self> {
        return Ok(Self {
            _stream: stream.into(),
            writer: None,
            #[cfg(web_sys_unstable_apis)]
            _builder: None,
            _phtm: PhantomData
        })
    }

    #[inline]
    pub async fn write (&mut self, chunk: &T) -> Result<()> {
        let _ = JsFuture::from(self.get_writer()?.write_with_chunk(chunk.as_ref())).await?;
        return Ok(())
    }

    #[inline]
    pub async fn close (&mut self) -> Result<()> {
        if let Some(ref writer) = self.writer.take() {
            writer.release_lock()
        }
        let _ = JsFuture::from(self._stream.close()).await?;
        return Ok(())
    }

    #[inline]
    pub async fn abort (&mut self) -> Result<()> {
        if let Some(ref writer) = self.writer.take() {
            writer.release_lock()
        }
        let _ = JsFuture::from(self._stream.abort()).await?;
        return Ok(())
    }

    #[inline]
    fn get_writer (&mut self) -> Result<&web_sys::WritableStreamDefaultWriter> {
        if let Some(ref writer) = self.writer {
            return Ok(writer)
        }
        
        let writer = self._stream.get_writer()?;
        self.writer = Some(writer);
        return Ok(unsafe { self.writer.as_ref().unwrap_unchecked() })
    }
}

impl<'a> JsWriteStream<'a, Uint8Array> {
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn from_rust_write<W: 'static + Unpin + futures::AsyncWrite> (w: W) -> Result<Self> {
        use futures::AsyncWriteExt;
        let w = std::rc::Rc::new(wasm_bindgen::__rt::WasmRefCell::new(w)); 

        return Self::custom()
            .write_async(move |chunk: Uint8Array, con| {
                let w = w.clone();
                return async move {
                    let mut w = w.borrow_mut();
                    for i in 0..chunk.length() {
                        let byte = chunk.get_index(i);
                        if let Err(e) = w.write_all(&[byte]).await {
                            return Err(JsValue::from_str(&e.to_string()));
                        }
                    }
                    return Ok(())
                }
            })
            .build();
    }
}

impl<'a, T: TypedArrayExt> JsWriteStream<'a, T> {
    /// Writes a view of the slice into the stream.
    #[inline]
    pub async fn write_slice (&mut self, buf: &[T::Element]) -> Result<()> {
        let chunk = unsafe { T::view(buf) };
        return self.write(&chunk).await
    }
}

impl<'a> JsWriteStream<'a, JsString> {
    /// Writes a string slice into the stream.
    #[inline]
    pub async fn write_str (&mut self, buf: &str) -> Result<()> {
        let chunk = JsString::from(buf);
        return self.write(&chunk).await
    }
}

impl<T> Drop for JsWriteStream<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if let Some(ref writer) = self.writer {
            writer.release_lock()
        }
        let _ = self._stream.abort();
    }
}

struct WriteChunk<'a> {
    inner: JsFuture,
    _phtm: PhantomData<&'a JsValue>
}

impl Future for WriteChunk<'_> {
    type Output = Result<()>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.inner.poll_unpin(cx)?.is_ready() {
            return Poll::Ready(Ok(()))
        }
        return Poll::Pending
    }
}