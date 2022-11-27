use std::{task::Poll, io::ErrorKind};

use futures::{AsyncWrite, FutureExt};
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use crate::Result;

pub struct JsWriteStream {
    #[allow(unused)]
    stream: web_sys::WritableStream,
    writer: web_sys::WritableStreamDefaultWriter,
    err: Option<JsValue>,
    close: Option<JsFuture>
}

impl JsWriteStream {
    #[inline]
    pub fn new (stream: web_sys::WritableStream) -> Result<Self> {
        let writer = stream.get_writer()?;
        return Ok(Self { stream, writer, err: None, close: None })
    }

    #[inline]
    pub async fn write_chunk (&mut self, buf: &[u8]) -> Result<()> {
        let chunk = unsafe { Uint8Array::view(buf) };
        JsFuture::from(self.writer.write_with_chunk(&chunk)).await?;
        return Ok(())
    }
}

impl AsyncWrite for JsWriteStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut offset = 0;

        while buf.len() > 0 {
            let chunk = unsafe { Uint8Array::view(core::slice::from_ref(&buf.get_unchecked(0))) };
            let mut fut = JsFuture::from(self.writer.write_with_chunk(&chunk));
            match fut.poll_unpin(cx) {
                Poll::Ready(Ok(_)) => {
                    buf = &buf[1..];
                    offset += 1
                },

                Poll::Ready(Err(e)) => {
                    self.err = Some(e);
                    return Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, "error writing to js stream")))
                },

                Poll::Pending if offset == 0 => return Poll::Pending,
                Poll::Pending => break
            }
        }

        return Poll::Ready(Ok(offset))
    }

    #[inline]
    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        return Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        if self.close.is_none() {
            self.close = Some(JsFuture::from(self.writer.close()))
        }

        let close = unsafe { self.close.as_mut().unwrap_unchecked() };
        return match close.poll_unpin(cx) {
            Poll::Ready(Ok(_)) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => {
                self.err = Some(e);
                return Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, "error closing js stream")))
            },
            Poll::Pending => Poll::Pending
        }
    }
}

impl Drop for JsWriteStream {
    #[inline]
    fn drop(&mut self) {
        self.writer.release_lock();
    }
}