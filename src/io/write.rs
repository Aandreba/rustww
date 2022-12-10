use std::{marker::PhantomData, task::Poll};
use docfg::docfg;
use elor::Either;
use futures::{Future, FutureExt, Sink};
use js_sys::{Uint8Array, JsString};
use wasm_bindgen::{JsValue};
use wasm_bindgen_futures::JsFuture;
use crate::{Result, utils::{TypedArrayExt}};

/// A rustfull wrapper arround a JavaScript [`WriteableStream`](web_sys::WritableStream)
pub struct JsWriteStream<'a, T> {
    pub(crate) _stream: web_sys::WritableStream,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::WriteBuilder<'a, T>>,
    pub(super) writer: Option<web_sys::WritableStreamDefaultWriter>,
    _phtm: PhantomData<&'a T>
}

impl<'a, T: AsRef<JsValue>> JsWriteStream<'a, T> {
    /// Returns a builder for a custom [`JsWriteStream`]
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> Result<super::builder::WriteBuilder<'a, T>> where T: wasm_bindgen::JsCast {
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

    pub fn into_sink (self) -> WriteSink<'a, T> where T: Unpin {
        return WriteSink {
            inner: self,
            flush: Either::Left(js_sys::Array::new()),
            close: None
        }
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

        return Self::custom()?
            .write_async(move |chunk: Uint8Array, _con| {
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

        #[cfg(web_sys_unstable_apis)]
        if let Some(ref builder) = self._builder {
            builder.handle.abort();
        }

        let _ = self._stream.abort();
    }
}

/// The [`Sink`](futures::Sink) version of [`JsWriteStream`]
pub struct WriteSink<'a, T> {
    inner: JsWriteStream<'a, T>,
    flush: Either<js_sys::Array, JsFuture>,
    close: Option<JsFuture>
}

impl<'a, T: Unpin> WriteSink<'a, T> {
    fn poll_ready_inner (mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        if let Either::Right(ref mut rhs) = self.flush {
            if rhs.poll_unpin(cx)?.is_pending() {
                return Poll::Pending
            }
            self.flush = Either::Left(js_sys::Array::new())
        }
        return Poll::Ready(Ok(()))
    }

    fn poll_flush_inner (mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        if let Either::Left(ref array) = self.flush {
            let promise = js_sys::Promise::all(array);
            self.flush = Either::Right(JsFuture::from(promise))
        }

        let flush = unsafe { self.flush.as_mut().unwrap_right_unchecked() };
        if flush.poll_unpin(cx)?.is_ready() {
            self.flush = Either::Left(js_sys::Array::new());
            return Poll::Ready(Ok(()))
        }
        return Poll::Pending
    }

    fn poll_close_inner (mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        if self.close.is_none() {
            if let Some(ref writer) = self.inner.writer.take() {
                writer.release_lock()
            }
            self.close = Some(JsFuture::from(self.inner._stream.close()));
        }

        let close = unsafe { self.close.as_mut().unwrap_unchecked() };
        if close.poll_unpin(cx)?.is_ready() {
            self.close = None;
            return Poll::Ready(Ok(()))
        }
        return Poll::Pending
    }
}

impl<'a, T: Unpin + AsRef<JsValue>> Sink<T> for WriteSink<'a, T> {
    type Error = JsValue;

    #[inline]
    fn poll_ready(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_ready_inner(cx)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: T) -> Result<()> {
        debug_assert!(self.flush.is_left());
        let promise = self.inner.get_writer()?.write_with_chunk(item.as_ref());
        unsafe { self.flush.as_mut().unwrap_left_unchecked().push(&promise) };
        return Ok(())
    }

    #[inline]
    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_flush_inner(cx)
    }

    #[inline]
    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_close_inner(cx)
    }
}

impl<'a, T: Unpin + TypedArrayExt> Sink<&'a [T::Element]> for WriteSink<'a, T> {
    type Error = JsValue;

    #[inline]
    fn poll_ready(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_ready_inner(cx)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: &'a [T::Element]) -> Result<()> {
        debug_assert!(self.flush.is_left());
        let view = unsafe { T::view(item) };
        let promise = self.inner.get_writer()?.write_with_chunk(view.as_ref());
        unsafe { self.flush.as_mut().unwrap_left_unchecked().push(&promise) };
        return Ok(())
    }

    #[inline]
    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_flush_inner(cx)
    }

    #[inline]
    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        self.poll_close_inner(cx)
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