use std::{task::{Poll}, marker::PhantomData};
use docfg::docfg;
use futures::{Future, TryFutureExt, FutureExt, Stream, TryStreamExt};
use js_sys::{Uint8Array};
use wasm_bindgen::{JsCast, JsValue, prelude::{wasm_bindgen}};
use wasm_bindgen_futures::JsFuture;
use crate::{Result, utils::{TypedArrayExt, TypedArray}};
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
pub struct JsReadStream<'a, T> {
    #[allow(unused)]
    stream: web_sys::ReadableStream,
    reader: Option<web_sys::ReadableStreamDefaultReader>,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::ReadBuilder<'a, T>>,
    current: Option<NextChunk>,
    done: bool,
    _phtm: PhantomData<&'a T>
}

impl<T: TypedArrayExt> JsReadStream<'_, T> {
    /// Reads the remaining entries in the stream flattened into a `Vec`
    pub async fn read_remaining_values (&mut self) -> Result<Vec<T::Element>> {
        let mut result = Vec::<T::Element>::new();

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
}

impl<T: TypedArray + JsCast> JsReadStream<'_, T> {
    /// Reads the remaining bytes in the stream into a `Vec<u8>`
    pub async fn read_remaining_bytes (&mut self) -> Result<Vec<u8>> {
        let mut result = Vec::<u8>::new();

        while let Some(chunk) = self.try_next().await? {
            let chunk = chunk.as_bytes();
            let len = chunk.byte_length() as usize;
            result.reserve(len);

            unsafe {
                chunk.raw_copy_to_ptr(result.as_mut_ptr().add(result.len()));
                result.set_len(result.len() + len);
            }
        }

        return Ok(result)
    }
}

impl<'a, T: JsCast> JsReadStream<'a, T> {
    /// Returns a builder for a custom [`JsReadStream`]
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::ReadBuilder<'a, T> {
        return super::builder::ReadBuilder::new()
    }

    /// Creates a new [`JsReadStream`]
    #[inline]
    pub fn new<S: Into<web_sys::ReadableStream>> (stream: S) -> Result<Self> {
        let stream = <S as Into<web_sys::ReadableStream>>::into(stream);
        return Ok(Self { stream, reader: None, #[cfg(web_sys_unstable_apis)] _builder: None, current: None, done: false, _phtm: PhantomData })
    }
    
    /// Creates a new [`JsReadStream`] from a teed [`ReadableStream`](web_sys::ReadableStream), assigning one of
    /// the teed streams to `stream`, and the other into the reader.
    pub fn from_mut (stream: &mut web_sys::ReadableStream) -> Result<Self> {
        let tee = stream.tee();

        let other = tee.get(0);
        debug_assert!(other.is_instance_of::<web_sys::ReadableStream>());
        *stream = other.unchecked_into();

        let this = tee.get(1);
        debug_assert!(this.is_instance_of::<web_sys::ReadableStream>());
        let this = this.unchecked_into::<web_sys::ReadableStream>();

        return Ok(Self { stream: this, reader: None, #[cfg(web_sys_unstable_apis)] _builder: None, current: None, done: false, _phtm: PhantomData })
    }

    /// Reads the remaining values in the stream into a `Vec`
    pub async fn read_remaining (&mut self) -> Result<Vec<T>> {
        let mut result = Vec::<T>::new();
        while let Some(chunk) = self.try_next().await? {
            result.push(chunk)
        }

        return Ok(result)
    }

    /// Returns a [`Future`] that resolves when the next chunk of the stream is available
    #[inline]
    fn next_chunk (&mut self) -> NextChunk {
        let promise = self.get_reader().read();
        let future = JsFuture::from(promise.clone());
        return NextChunk { future }
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

impl<T: JsCast> Stream for JsReadStream<'_, T> {
    type Item = Result<T>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done { return Poll::Ready(None) }
        if self.current.is_none() { self.current = Some(self.next_chunk()) }

        let chunk = unsafe { self.current.as_mut().unwrap_unchecked() };
        if let Poll::Ready(ChunkResult { done, value }) = chunk.poll_unpin(cx)? {
            self.done = done || value.is_none();
            self.current = None;

            if let Some(value) = value {
                return Poll::Ready(Some(JsCast::dyn_into(value)));
            } else if done {
                return Poll::Ready(None)
            }
        }

        return Poll::Pending
    }
}

impl IntoFetchBody for JsReadStream<'static, Uint8Array> {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(self.stream.clone().into())
    }
}

impl<T> Drop for JsReadStream<'_, T> {
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
    future: JsFuture
}

impl Future for NextChunk {
    type Output = Result<ChunkResult>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(result) = self.future.try_poll_unpin(cx)? {
            return Poll::Ready(ChunkResult::try_from(&result))
        }

        return Poll::Pending
    }
}

impl TryFrom<&JsValue> for ChunkResult {
    type Error = JsValue;

    fn try_from(result: &JsValue) -> Result<Self> {
        let done = match js_sys::Reflect::get(result, &JsValue::from_str("done"))?.as_bool() {
            Some(x) => x,
            None => return Err(JsValue::from_str("`done` field not found"))
        };

        let value = js_sys::Reflect::get(result, &JsValue::from_str("value"))?;
        if value.is_null() || value.is_undefined() { return Ok(Self { done, value: None }) }
        return Ok(Self { done, value: Some(value) })
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
    pub value: Option<JsValue>
}