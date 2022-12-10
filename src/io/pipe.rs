use std::marker::PhantomData;

use wasm_bindgen::{JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::StreamPipeOptions;
use crate::{Result, utils::AbortController};
use super::{JsReadStream, JsWriteStream};

pub struct PipeController<'a, 'b, T> {
    src: JsReadStream<'a, T>,
    abort: AbortController<()>,
    _dst: PhantomData<&'b mut JsWriteStream<'a, T>>,
}

impl<'a, T> PipeController<'a, '_, T> {
    /// Aborts the pipeing, returning it's underlying [`JsReadStream`]
    #[inline]
    pub fn abort (self) -> JsReadStream<'a, T> {
        self.abort.abort();
        return self.src
    }
}

impl<'a, T: JsCast> JsReadStream<'a, T> {
    /// Pipes the contents from the [`JsReadStream`] into the [`JsWriteStream`]
    pub async fn pipe_to<'d> (self, dst: &'d mut JsWriteStream<'a, T>) -> Result<PipeController<'a, 'd, T>> {
        // Release locks
        if let Some(ref reader) = self.reader {
            reader.release_lock();
        }
        if let Some(ref writer) = dst.writer {
            writer.release_lock();
        }
    
        // Pipeing options
        let mut ops = StreamPipeOptions::new();
        let abort = AbortController::new()?;

        ops.prevent_close(true);
        ops.prevent_abort(true);
        ops.prevent_cancel(true);
        ops.signal(&abort.raw_signal());
        // todo signal
    
        // Perform pipeing
        let _ = JsFuture::from(self._stream.pipe_to_with_options(&dst._stream, &ops)).await?;
        return Ok(PipeController {
            src: self,
            _dst: PhantomData,
            abort
        })
    }
}

impl<'a, T: JsCast> JsWriteStream<'a, T> {
    /// Pipes the contents from the [`JsReadStream`] into the [`JsWriteStream`]
    #[inline]
    pub async fn pipe_from<'d> (&'d mut self, src: JsReadStream<'a, T>) -> Result<PipeController<'a, 'd, T>> {
        return src.pipe_to(self).await
    }
}