use std::{task::{Poll, Waker}, cell::Cell};
use docfg::docfg;
use futures::{Future, TryFutureExt, FutureExt, Stream, TryStreamExt};
use js_sys::{Uint8Array};
use wasm_bindgen::{JsCast, JsValue, prelude::{wasm_bindgen}};
use wasm_bindgen_futures::JsFuture;
use web_sys::ReadableStream;
use crate::Result;
use super::IntoFetchBody;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Uint8Array, extends = js_sys::Uint8Array)]
    #[derive(Debug, Clone)]
    type ByteArray;

    #[wasm_bindgen(method)]
    fn subarray(this: &ByteArray, begin: u32) -> ByteArray;
}

/// A rustfull wrapper arround a JavaScript [`ReadableStream`](web_sys::ReadableStream)
pub struct JsReadStream {
    #[allow(unused)]
    stream: web_sys::ReadableStream,
    reader: Option<web_sys::ReadableStreamDefaultReader>,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::ReadBuilder>,
    current: Option<NextChunk>,
    done: bool
}

impl JsReadStream {
    /// Returns a builder for a custom [`JsReadStream`]
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::ReadBuilder {
        return super::builder::ReadBuilder::new()
    }

    /// Creates a new [`JsReadStream`]
    #[inline]
    pub fn new<T: Into<web_sys::ReadableStream>> (stream: T) -> Result<Self> {
        let stream = <T as Into<web_sys::ReadableStream>>::into(stream);
        return Ok(Self { stream, reader: None, #[cfg(web_sys_unstable_apis)] _builder: None, current: None, done: false })
    }
    
    /// Creates a new [`JsReadStream`] from a teed [`ReadableStream`], assigning one of
    /// the teed streams to `stream`, and the other into the reader.
    pub fn from_mut (stream: &mut web_sys::ReadableStream) -> Result<Self> {
        let tee = stream.tee();

        let other = tee.get(0);
        debug_assert!(other.is_instance_of::<web_sys::ReadableStream>());
        *stream = other.unchecked_into();

        let this = tee.get(1);
        debug_assert!(this.is_instance_of::<web_sys::ReadableStream>());
        let this = this.unchecked_into::<web_sys::ReadableStream>();

        return Ok(Self { stream: this, reader: None, #[cfg(web_sys_unstable_apis)] _builder: None, current: None, done: false })
    }
    
    /// Reads the remaining bytes in the stream into a `Vec<u8>`
    pub async fn read_remaining (&mut self) -> Result<Vec<u8>> {
        let mut result = Vec::<u8>::new();

        while let Some(chunk) = self.try_next().await? {
            let len = chunk.length() as usize;
            result.reserve(len);

            unsafe {
                chunk.raw_copy_to_ptr(result.as_mut_ptr().add(result.len()));
                result.set_len(result.len() + len);
            }
        }

        return Ok(result)
    }

    /// Attempts to clone the [`JsReadStream`].
    pub async fn try_clone (&mut self) -> Result<Self> {
        // Wait for current chunk to finish
        while let Some(ref mut current) = self.current {
            let _ = JsFuture::from(current.promise.clone()).await?;
        }

        // Release read lock
        if let Some(ref reader) = self.reader {
            reader.release_lock();
            self.reader = None;
        }

        // Tee stream
        let array = self.stream.tee();
        self.stream = array.get(0).unchecked_into();
        let clone = array.get(1).unchecked_into::<ReadableStream>();
        return Self::new(clone)
    }

    /// Returns a [`Future`] that resolves when the next chunk of the stream is available
    #[inline]
    fn next_chunk (&mut self) -> NextChunk {
        let promise = self.get_reader().read();
        let future = JsFuture::from(promise.clone());
        return NextChunk { promise, future, waker: Cell::new(None) }
    }

    fn get_reader (&mut self) -> &web_sys::ReadableStreamDefaultReader {
        if let Some(ref reader) = self.reader {
            return reader
        }

        let reader = self.stream.get_reader();
        debug_assert!(reader.is_instance_of::<web_sys::ReadableStreamDefaultReader>());
        self.reader = Some(reader.unchecked_into::<web_sys::ReadableStreamDefaultReader>());
        return unsafe { self.reader.as_ref().unwrap_unchecked() }
    }
}

impl Stream for JsReadStream {
    type Item = Result<Uint8Array>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done { return Poll::Ready(None) }
        if self.current.is_none() { self.current = Some(self.next_chunk()) }

        let chunk = unsafe { self.current.as_mut().unwrap_unchecked() };
        if let Poll::Ready(ChunkResult { done, value }) = chunk.poll_unpin(cx)? {
            self.done = done || value.is_none();
            self.current = None;
            return unsafe { Poll::Ready(Some(Ok(value.unwrap_unchecked()))) };
        }

        return Poll::Pending
    }
}

impl IntoFetchBody for JsReadStream {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(self.stream.clone().into())
    }
}

impl Drop for JsReadStream {
    #[inline]
    fn drop(&mut self) {
        if let Some(ref reader) = self.reader {
            reader.release_lock()
        }
        let _ = self.stream.cancel();
    }
}

/*
/// A rustfull wrapper arround a JavaScript byte [`ReadableStream`](web_sys::ReadableStream)
pub struct JsReadByteStream {
    #[allow(unused)]
    stream: web_sys::ReadableStream,
    reader: web_sys::ReadableStreamByobReader,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::ReadByteBuilder>,
    err: Option<JsValue>
}

impl JsReadByteStream {
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::ReadByteBuilder {
        return super::builder::ReadByteBuilder::new()
    }

    #[inline]
    pub fn new<T: Into<web_sys::ReadableStream>> (stream: T) -> Result<Self> {
        let stream = <T as Into<web_sys::ReadableStream>>::into(stream);

        let mut ops = ReadableStreamGetReaderOptions::new();
        ops.mode(ReadableStreamReaderMode::Byob);

        let inner = stream.get_reader_with_options(&ops);
        debug_assert!(inner.is_instance_of::<web_sys::ReadableStreamByobReader>());
        let inner = inner.unchecked_into::<web_sys::ReadableStreamByobReader>();

        return Ok(Self { stream, reader: inner, #[cfg(web_sys_unstable_apis)] _builder: None, err: None })
    }
    
    pub fn from_mut (stream: &mut web_sys::ReadableStream) -> Result<Self> {
        let tee = stream.tee();

        let other = tee.get(0);
        debug_assert!(other.is_instance_of::<web_sys::ReadableStream>());
        *stream = other.unchecked_into();

        let this = tee.get(1);
        debug_assert!(this.is_instance_of::<web_sys::ReadableStream>());
        let this = this.unchecked_into::<web_sys::ReadableStream>();

        let mut ops = ReadableStreamGetReaderOptions::new();
        ops.mode(ReadableStreamReaderMode::Byob);

        let inner = this.get_reader_with_options(&ops);
        debug_assert!(inner.is_instance_of::<web_sys::ReadableStreamByobReader>());
        let inner = inner.unchecked_into::<web_sys::ReadableStreamByobReader>();
        
        return Ok(Self { stream: this, reader: inner, #[cfg(web_sys_unstable_apis)] _builder: None, err: None })
    }

    async fn next_chunk (&mut self, v: &mut [u8]) {

    }
}

impl Drop for JsReadByteStream {
    #[inline]
    fn drop(&mut self) {
        self.reader.release_lock();
    }
}
*/

/// Future for [`next_chunk`](JsReadStream::next_chunk)
struct NextChunk {
    promise: js_sys::Promise,
    future: JsFuture,
    waker: Cell<Option<Waker>>
}

impl Future for NextChunk {
    type Output = Result<ChunkResult>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(result) = self.future.try_poll_unpin(cx)? {
            if let Some(waker) = self.waker.take() { waker.wake() }
            return Poll::Ready(ChunkResult::try_from(&result))
        }

        return Poll::Pending
    }
}

impl TryFrom<&JsValue> for ChunkResult {
    type Error = JsValue;

    fn try_from(result: &JsValue) -> Result<Self> {
        let done = unsafe {
            js_sys::Reflect::get(result, &JsValue::from_str("done"))?
                .as_bool()
                .unwrap_unchecked()
        };

        let value = js_sys::Reflect::get(result, &JsValue::from_str("value"))?;
        if value.is_null() || value.is_undefined() { return Ok(Self { done, value: None }) }
        debug_assert!(value.is_instance_of::<Uint8Array>());
        return Ok(Self { done, value: Some(value.unchecked_into()) })
    }
}

impl TryFrom<JsValue> for ChunkResult {
    type Error = JsValue;

    #[inline]
    fn try_from(result: JsValue) -> Result<Self> {
        return Self::try_from(&result)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
struct ChunkResult {
    pub done: bool,
    pub value: Option<Uint8Array>
}