use std::{task::Poll, io::ErrorKind, future::poll_fn};
use futures::{AsyncRead, Future, TryFutureExt, FutureExt};
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;
use crate::Result;

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
    inner: web_sys::ReadableStreamDefaultReader,
    buff: Option<ByteArray>,
    err: Option<JsValue>,
    current: NextChunk
}

impl JsReadStream {
    #[inline]
    pub fn new<T: Into<web_sys::ReadableStream>> (stream: T) -> Result<Self> {
        let inner = <T as Into<web_sys::ReadableStream>>::into(stream).get_reader();
        debug_assert!(inner.is_instance_of::<web_sys::ReadableStreamDefaultReader>());
        let inner = inner.unchecked_into::<web_sys::ReadableStreamDefaultReader>();

        let current = NextChunk { future: JsFuture::from(inner.read()) };
        return Ok(Self { inner: inner, err: None, buff: None, current })
    }
    
    pub fn from_mut (stream: &mut web_sys::ReadableStream) -> Result<Self> {
        let tee = stream.tee();

        let other = tee.get(0);
        debug_assert!(other.is_instance_of::<web_sys::ReadableStream>());
        *stream = other.unchecked_into();

        let this = tee.get(1);
        debug_assert!(this.is_instance_of::<web_sys::ReadableStream>());
        let this = this.unchecked_into::<web_sys::ReadableStream>();

        let inner = this.get_reader();
        debug_assert!(inner.is_instance_of::<web_sys::ReadableStreamDefaultReader>());
        let inner = inner.unchecked_into::<web_sys::ReadableStreamDefaultReader>();
        
        let current = NextChunk { future: JsFuture::from(inner.read()) };
        return Ok(Self { inner, err: None, buff: None, current })
    }

    #[inline]
    pub fn next_chunk (&mut self) -> NextChunk {
        let future = JsFuture::from(self.inner.read());
        return NextChunk { future }
    }

    pub async fn read_remaining (&mut self) -> Result<Vec<u8>> {
        let mut result = match self.buff {
            Some(ref x) => x.to_vec(),
            None => Vec::new()
        };

        loop {
            let ChunkResult { done, value } = poll_fn(|cx| self.current.poll_unpin(cx)).await?;
            if done || value.is_undefined() {
                break
            }
            
            self.current = self.next_chunk();
            let len = value.byte_length() as usize;
            result.reserve(len);

            unsafe {
                value.raw_copy_to_ptr(result.as_mut_ptr().add(result.len()));
                result.set_len(result.len() + len)
            }
        }

        return Ok(result)
    }

    #[inline]
    pub fn latest_error (&self) -> Option<&JsValue> {
        return self.err.as_ref()
    }
}

impl AsyncRead for JsReadStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut offset = 0;

        loop {
            /* BUFFER */
            if let Some(ref mut self_buf) = self.buff {
                let len = u32::min(self_buf.length(), u32::try_from(buf.len()).unwrap_or(u32::MAX));
                let len_usize = len as usize;
    
                Uint8Array::subarray(self_buf, 0, len).copy_to(&mut buf[..len_usize]);
                *self_buf = self_buf.subarray(len);
    
                offset += len_usize;
                if self_buf.length() > 0 {
                    break;
                }

                buf = &mut buf[len_usize..];
                self.buff = None
            }
    
            /* PROMISE */
            match self.current.poll_unpin(cx) {
                Poll::Ready(Ok(ChunkResult { done, value })) => {
                    if done || value.is_undefined() {
                        break;
                    }
    
                    self.buff = Some(value.unchecked_into::<ByteArray>());
                    self.current = self.next_chunk();
                },
    
                Poll::Ready(Err(e)) => {
                    self.err = Some(e);
                    return Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, "error reading js stream")))
                },

                Poll::Pending if offset == 0 => return Poll::Pending,
                Poll::Pending => break
            }
        }

        return Poll::Ready(Ok(offset))
    }
}

impl Drop for JsReadStream {
    #[inline]
    fn drop(&mut self) {
        self.inner.release_lock()
    }
}

/// Future for [`next_chunk`](JsReadStream::next_chunk)
pub struct NextChunk {
    future: JsFuture
}

impl Future for NextChunk {
    type Output = Result<ChunkResult>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(result) = self.future.try_poll_unpin(cx)? {
            let done = unsafe {
                js_sys::Reflect::get(&result, &JsValue::from_str("done"))?
                    .as_bool()
                    .unwrap_unchecked()
            };
    
            let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))?;
            debug_assert!(value.is_instance_of::<Uint8Array>());
            return Poll::Ready(Ok(ChunkResult { done, value: value.unchecked_into::<Uint8Array>() }));
        }

        return Poll::Pending
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChunkResult {
    pub done: bool,
    pub value: Uint8Array
}