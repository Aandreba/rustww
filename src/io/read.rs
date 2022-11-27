use std::{task::Poll, io::ErrorKind, future::poll_fn, rc::Rc};
use docfg::docfg;
use futures::{AsyncRead, Future, TryFutureExt, FutureExt};
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue, prelude::{wasm_bindgen}, __rt::WasmRefCell};
use wasm_bindgen_futures::JsFuture;
use web_sys::{ReadableStreamGetReaderOptions, ReadableStreamReaderMode};
use crate::Result;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Uint8Array, extends = js_sys::Uint8Array)]
    #[derive(Debug, Clone)]
    type ByteArray;

    #[cfg(web_sys_unstable_apis)]
    #[wasm_bindgen(js_name = ReadableStreamBYOBRequest , typescript_type = "ReadableStreamBYOBRequest", extends = web_sys::ReadableStreamByobRequest)]
    type ExtendedReadableStreamByobRequest;

    #[wasm_bindgen(method)]
    fn subarray(this: &ByteArray, begin: u32) -> ByteArray;

    #[cfg(web_sys_unstable_apis)]
    #[wasm_bindgen(method, js_name = view)]
    fn view_ptr(this: &ByteArray) -> *mut u8;
}

/// Trait that represents a type that wraps a JavaScript [`ReadableStream`](web_sys::ReadableStream)
pub trait AsJsReadStream: AsyncRead {
    fn as_stream (&self) -> &web_sys::ReadableStream;
}

/// A rustfull wrapper arround a JavaScript [`ReadableStream`](web_sys::ReadableStream)
pub struct JsReadStream {
    #[allow(unused)]
    stream: web_sys::ReadableStream,
    reader: web_sys::ReadableStreamDefaultReader,
    #[cfg(web_sys_unstable_apis)]
    pub(super) _builder: Option<super::builder::ReadBuilder>,
    buff: Option<ByteArray>,
    err: Option<JsValue>,
    current: NextChunk
}

impl JsReadStream {   
    #[docfg(web_sys_unstable_apis)]
    #[inline]
    pub fn custom () -> super::builder::ReadBuilder {
        return super::builder::ReadBuilder::new()
    }

    #[inline]
    pub fn new<T: Into<web_sys::ReadableStream>> (stream: T) -> Result<Self> {
        let stream = <T as Into<web_sys::ReadableStream>>::into(stream);
        let inner = stream.get_reader();
        debug_assert!(inner.is_instance_of::<web_sys::ReadableStreamDefaultReader>());
        let inner = inner.unchecked_into::<web_sys::ReadableStreamDefaultReader>();

        let current = NextChunk { future: JsFuture::from(inner.read()) };
        return Ok(Self { stream, reader: inner, #[cfg(web_sys_unstable_apis)] _builder: None, err: None, buff: None, current })
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
        return Ok(Self { stream: this, reader: inner, #[cfg(web_sys_unstable_apis)] _builder: None, err: None, buff: None, current })
    }

    #[inline]
    fn next_chunk (&mut self) -> NextChunk {
        let future = JsFuture::from(self.reader.read());
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

impl AsJsReadStream for JsReadStream {
    #[inline]
    fn as_stream (&self) -> &web_sys::ReadableStream {
        return &self.stream
    }
}

impl Drop for JsReadStream {
    #[inline]
    fn drop(&mut self) {
        self.reader.release_lock()
    }
}

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

    #[docfg(web_sys_unstable_apis)]
    //#[cfg(not(target_feature = "atomics"))]
    pub fn from_reader<R: AsyncRead> (read: R) -> Result<Self> {
        let read = Rc::new(WasmRefCell::new(read));

        let my_read = read.clone();
        let start = move |con: web_sys::ReadableByteStreamController| {
            Ok(())
        };

        let my_read = read.clone();
        let pull = move |con: web_sys::ReadableByteStreamController| async move {
            ::web_sys::console::log_1(&con);

            if let Some(req) = con.byob_request() {
                let req = req.unchecked_into::<ExtendedReadableStreamByobRequest>();
                ::web_sys::console::log_1(&req);

                if let Some(view) = req.view() {
                    ::web_sys::console::log_1(&view);
                }
                
                todo!()
            }

            Ok(())
        };

        return Self::custom()
            .start(start)
            .pull_async(pull)
            .build();
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

    #[inline]
    pub async fn read_chunk (&mut self, buf: &mut [u8]) -> Result<()> {
        if buf.len() == 0 { return Ok(()) }
        JsFuture::from(self.reader.read_with_u8_array(buf)).await?;
        return Ok(())
    }
}

impl AsyncRead for JsReadByteStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut offset = 0;

        while buf.len() > 0 {
            let fut = unsafe {
                self.reader.read_with_u8_array(core::slice::from_mut(buf.get_unchecked_mut(0)))
            };

            match JsFuture::from(fut).poll_unpin(cx) {
                Poll::Ready(Ok(_)) => {
                    offset += 1;
                    buf = &mut buf[1..];
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

impl AsJsReadStream for JsReadByteStream {
    #[inline]
    fn as_stream (&self) -> &web_sys::ReadableStream {
        return &self.stream
    }
}

impl Drop for JsReadByteStream {
    #[inline]
    fn drop(&mut self) {
        self.reader.release_lock();
    }
}

/// Future for [`next_chunk`](JsReadStream::next_chunk)
struct NextChunk {
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