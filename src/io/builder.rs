use futures::{Future, TryFutureExt};
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::{ReadableByteStreamController, ReadableStreamDefaultController};
use js_sys::*;
use crate::Result;
use super::*;
use wasm_bindgen::Clamped;

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

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the stream source, and do anything else required to set up the stream functionality.
    #[inline]
    pub fn start<F: 'static + FnOnce(ReadStreamController) -> Result<()>> (mut self, f: F) -> Self {
        self.start = Some(MaybePromise::Blocking(Closure::once(|inner| f(ReadStreamController { inner }))));
        self
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the stream source, and do anything else required to set up the stream functionality.
    #[inline]
    pub fn start_async<F: 'static + FnOnce(ReadStreamController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |inner| {
            let fut = f(ReadStreamController { inner }).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.start = Some(MaybePromise::Promise(Closure::once(f)));
        self
    }

    /// This method, also defined by the developer, will be called repeatedly when the stream's internal queue of chunks is not full, up until it reaches its high water mark.
    #[inline]
    pub fn pull<F: 'static + FnMut(ReadStreamController) -> Result<()>> (mut self, f: F) -> Self {
        self.pull = Some(MaybePromise::Blocking(Closure::new(|inner| f(ReadStreamController { inner }))));
        self
    }

    /// This method, also defined by the developer, will be called repeatedly when the stream's internal queue of chunks is not full, up until it reaches its high water mark.
    #[inline]
    pub fn pull_async<F: 'static + FnMut(ReadableStreamDefaultController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let f = move |inner| {
            let fut = f(ReadStreamController { inner }).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        self.pull = Some(MaybePromise::Promise(Closure::new(f)));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that the stream is to be cancelled
    #[inline]
    pub fn cancel<F: 'static + FnOnce(JsValue) -> Result<()>> (mut self, f: F) -> Self {
        self.cancel = Some(MaybePromise::Blocking(Closure::once(f)));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that the stream is to be cancelled
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

#[derive(Debug, Clone)]
pub struct ReadStreamController {
    inner: ReadableStreamDefaultController
}

impl ReadStreamController {
    #[inline]
    pub fn close (&self) -> Result<()> {
        self.inner.close()
    }

    #[inline]
    pub fn desired_size (&self) -> Result<f64> {
        self.inner.desired_size()
    }

    #[inline]
    pub fn enqueue<C: AsChunk> (&self, chunk: &C) -> Result<()> {
        let chunk: JsValue = chunk.as_chunk();
        return self.inner.enqueue_with_chunk(&chunk)
    }

    #[inline]
    pub fn error (&self, e: Option<&JsValue>) {
        match e {
            Some(e) => self.inner.error_with_e(e),
            None => self.inner.error()
        }
    }
}

/// A value that can be turned into a JavaScript's stream chunk
pub trait AsChunk {
    fn as_chunk (&self) -> JsValue;
}

impl<T: AsChunk> AsChunk for &T {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        T::as_chunk(*self)
    }
}

macro_rules! impl_chunk {
    ($($name:ty $(=> @clamped [$cty:ty])? $(=> [$ty:ty])?),+) => {
        $(
            impl AsChunk for $name {
                #[inline]
                fn as_chunk (&self) -> JsValue { self.into() }
            }

            $(
                impl AsChunk for [$ty] {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        let chunk = <$name>::from(self);
                        return chunk.into()
                    }
                }

                impl AsChunk for Vec<$ty> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        <[$ty]>::as_chunk(self)
                    }
                }

                impl AsChunk for Box<[$ty]> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        <[$ty]>::as_chunk(self)
                    }
                }

                impl AsChunk for std::rc::Rc<[$ty]> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        <[$ty]>::as_chunk(self)
                    }
                }

                impl AsChunk for std::sync::Arc<[$ty]> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        <[$ty]>::as_chunk(self)
                    }
                }
            )?

            $(
                impl AsChunk for Clamped<&[$cty]> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        let chunk = <$name>::from(self);
                        return chunk.into()
                    }
                }

                impl AsChunk for Clamped<Vec<$cty>> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        let chunk = <$name>::from(self);
                        return chunk.into()
                    }
                }
            )?
        )+
    };

    (@slice $($t:ty as $name:ident),+) => {
        $(
            impl AsChunk for [$t] {
                #[inline]
                fn as_chunk (&self) -> JsValue {
                    self.into()
                }
            }
        )+
    }
}

impl_chunk! {
    Uint8Array => [u8],
    Uint8ClampedArray => @clamped [u8],
    Uint16Array => [u16],
    Uint32Array => [u32],
    BigUint64Array => [u64],
    Int8Array => [i8],
    Int16Array => [i16],
    Int32Array => [i32],
    BigInt64Array => [i64],
    Float32Array => [f32],
    Float64Array => [f64]
}

impl AsChunk for JsString {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        self.into()
    }
}

impl AsChunk for str {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        JsValue::from_str(self)
    }
}

impl AsChunk for String {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        <str as AsChunk>::as_chunk(self)
    } 
}

impl AsChunk for Box<str> {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        <str as AsChunk>::as_chunk(self)
    } 
}

impl AsChunk for std::rc::Rc<str> {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        <str as AsChunk>::as_chunk(self)
    } 
}

impl AsChunk for std::sync::Arc<str> {
    #[inline]
    fn as_chunk (&self) -> JsValue {
        <str as AsChunk>::as_chunk(self)
    } 
}