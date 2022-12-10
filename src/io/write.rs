use std::{marker::PhantomData, task::Poll};
use docfg::docfg;
use futures::{Sink, Future, FutureExt};
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use crate::{Result, utils::{TypedArrayExt}};

/// A rustfull wrapper arround a JavaScript [`WriteableStream`](web_sys::WritableStream)
pub struct JsWriteStream<'a, T> {
    pub(crate) _stream: web_sys::WritableStream,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::WriteBuilder<'a, T>>,
    writer: Option<web_sys::WritableStreamDefaultWriter>,
    current: Option<WriteChunk<'a>>,
    closing: Option<JsFuture>,
    _phtm: PhantomData<&'a T>
}

impl<'a, T: AsRef<JsValue>> JsWriteStream<'a, T> {
    /// Returns a builder for a custom [`JsWriteStream`]
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::WriteBuilder<'a, T> {
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
            current: None,
            closing: None,
            _phtm: PhantomData
        })
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
    pub fn from_rust_write<W: 'a + futures::AsyncWrite> (w: W) -> Result<Self> {
        return Self::custom()
            .write_async(move |chunk, con| async move {
                
            })
            .build();
    }
}

impl<'a, T: TypedArrayExt> JsWriteStream<'a, T> {
    /// Writes a view of the slice into the stream. This avoids the extra allocation that would
    /// be done with `write_serialized`.
    #[inline]
    pub async fn write_slice (&mut self, buf: &[T::Element]) -> Result<()> {
        let chunk = unsafe { T::view(buf) };
        todo!()
        //return self.write(&chunk).await
    }
}

impl<T: AsRef<JsValue>> Sink<T> for JsWriteStream<'_, T> {
    type Error = JsValue;

    #[inline]
    fn poll_ready(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<()>> {
        if let Some(ref mut current) = self.current {
            if current.poll_unpin(cx)?.is_pending() {
                return Poll::Pending
            }
            self.current = None;
        }
        return Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: T) -> Result<()> {
        debug_assert!(self.current.is_none());
        let write = self.get_writer()?;

        self.current = Some(WriteChunk {
            inner: JsFuture::from(write.write_with_chunk(item.as_ref())),
            _phtm: PhantomData
        });

        return Ok(())
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<()>> {
        if let Some(ref mut current) = self.current {
            if current.poll_unpin(cx)?.is_pending() {
                return Poll::Pending
            }
            self.current = None
        }
        return Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<()>> {
        if self.closing.is_none() {
            if let Some(ref writer) = self.writer.take() {
                writer.release_lock()
            }
            self.closing = Some(JsFuture::from(self._stream.close()));
        }

        let closing = unsafe { self.closing.as_mut().unwrap_unchecked() };
        if closing.poll_unpin(cx)?.is_ready() {
            return Poll::Ready(Ok(()))
        } else {
            return Poll::Pending
        }
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
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.inner.poll_unpin(cx)?.is_ready() {
            return Poll::Ready(Ok(()))
        }
        return Poll::Pending
    }
}