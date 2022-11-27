use futures::{Future, TryFutureExt};
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::{ReadableByteStreamController, ReadableStreamDefaultController};
use crate::Result;
use super::*;

#[derive(Debug)]
enum MaybePromise<T> {
    Blocking (Closure<dyn FnMut(T) -> Result<()>>),
    Promise (Closure<dyn FnMut(T) -> js_sys::Promise>)
}

impl<T> AsRef<JsValue> for MaybePromise<T> {
    #[inline]
    fn as_ref(&self) -> &JsValue {
        match self {
            Self::Blocking(x) => x.as_ref(),
            Self::Promise(x) => x.as_ref()
        }
    }
}

#[derive(Debug, Default)]
pub struct ReadBuilder {
    start: Option<MaybePromise<ReadableStreamDefaultController>>,
    pull: Option<MaybePromise<ReadableStreamDefaultController>>,
    cancel: Option<MaybePromise<JsValue>>
}

impl ReadBuilder {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    #[inline]
    pub fn start<F: 'static + FnOnce(ReadableStreamDefaultController) -> Result<()>> (mut self, f: F) -> Self {
        self.start = Some(MaybePromise::Blocking(Closure::once(f)));
        self
    }

    #[inline]
    pub fn start_async<F: 'static + FnOnce(ReadableStreamDefaultController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.start = Some(MaybePromise::Promise(Closure::once(f)));
        self
    }

    #[inline]
    pub fn pull<F: 'static + FnMut(ReadableStreamDefaultController) -> Result<()>> (mut self, f: F) -> Self {
        self.pull = Some(MaybePromise::Blocking(Closure::new(f)));
        self
    }

    #[inline]
    pub fn pull_async<F: 'static + FnMut(ReadableStreamDefaultController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.pull = Some(MaybePromise::Promise(Closure::new(f)));
        self
    }

    #[inline]
    pub fn cancel<F: 'static + FnOnce(JsValue) -> Result<()>> (mut self, f: F) -> Self {
        self.cancel = Some(MaybePromise::Blocking(Closure::once(f)));
        self
    }

    #[inline]
    pub fn cancel_async<F: 'static + FnOnce(JsValue) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.cancel = Some(MaybePromise::Promise(Closure::once(f)));
        self
    }

    pub fn build (self) -> Result<JsReadStream> {
        macro_rules! set {
            ($($name:ident [$key:literal] = $value:expr;)+) => {
                $(
                    js_sys::Reflect::set(&$name, &JsValue::from_str($key), $value)?;
                )+
            };
        }

        let underlying_source = js_sys::Object::new();

        if let Some(ref start) = self.start {
            set! { underlying_source["start"] = start.as_ref(); }
        }

        if let Some(ref pull) = self.pull {
            set! { underlying_source["pull"] = pull.as_ref(); }
        }

        if let Some(ref cancel) = self.cancel {
            set! { underlying_source["cancel"] = cancel.as_ref(); }
        }

        let stream = web_sys::ReadableStream::new_with_underlying_source(&underlying_source)?;
        let mut result = JsReadStream::new(stream)?;
        result._builder = Some(self);
        return Ok(result)
    }
}

#[derive(Debug, Default)]
pub struct ReadByteBuilder {
    start: Option<MaybePromise<ReadableByteStreamController>>,
    pull: Option<MaybePromise<ReadableByteStreamController>>,
    cancel: Option<MaybePromise<JsValue>>,
    auto_allocate_chunk_size: Option<u32>,
}

impl ReadByteBuilder {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    #[inline]
    pub fn start<F: 'static + FnOnce(ReadableByteStreamController) -> Result<()>> (mut self, f: F) -> Self {
        self.start = Some(MaybePromise::Blocking(Closure::once(f)));
        self
    }

    #[inline]
    pub fn start_async<F: 'static + FnOnce(ReadableByteStreamController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.start = Some(MaybePromise::Promise(Closure::once(f)));
        self
    }

    #[inline]
    pub fn pull<F: 'static + FnMut(ReadableByteStreamController) -> Result<()>> (mut self, f: F) -> Self {
        self.pull = Some(MaybePromise::Blocking(Closure::new(f)));
        self
    }

    #[inline]
    pub fn pull_async<F: 'static + FnMut(ReadableByteStreamController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.pull = Some(MaybePromise::Promise(Closure::new(f)));
        self
    }

    #[inline]
    pub fn cancel<F: 'static + FnOnce(JsValue) -> Result<()>> (mut self, f: F) -> Self {
        self.cancel = Some(MaybePromise::Blocking(Closure::once(f)));
        self
    }

    #[inline]
    pub fn cancel_async<F: 'static + FnOnce(JsValue) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.cancel = Some(MaybePromise::Promise(Closure::once(f)));
        self
    }

    #[inline]
    pub fn auto_allocate_chunk_size (mut self, v: u32) -> Self {
        self.auto_allocate_chunk_size = Some(v);
        self
    }

    pub fn build (self) -> Result<JsReadByteStream> {
        macro_rules! set {
            ($($name:ident [$key:literal] = $value:expr;)+) => {
                $(
                    js_sys::Reflect::set(&$name, &JsValue::from_str($key), $value)?;
                )+
            };
        }

        let underlying_source = js_sys::Object::new();
        set! { underlying_source["type"] = &JsValue::from_str("bytes"); }

        if let Some(ref start) = self.start {
            set! { underlying_source["start"] = start.as_ref(); }
        }

        if let Some(ref pull) = self.pull {
            set! { underlying_source["pull"] = pull.as_ref(); }
        }

        if let Some(ref cancel) = self.cancel {
            set! { underlying_source["cancel"] = cancel.as_ref(); }
        }

        if let Some(aacs) = self.auto_allocate_chunk_size {
            set! { underlying_source["autoAllocateChunkSize"] = &JsValue::from(aacs); }
        }

        let stream = web_sys::ReadableStream::new_with_underlying_source(&underlying_source)?;
        let mut result = JsReadByteStream::new(stream)?;
        result._builder = Some(self);
        return Ok(result)
    }
}