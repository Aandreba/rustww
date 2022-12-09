use futures::{Future, TryFutureExt};
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::{ReadableByteStreamController, ReadableStreamDefaultController};
use js_sys::*;
use crate::Result;
use super::*;
use wasm_bindgen::Clamped;
use core::marker::PhantomData;
use wasm_bindgen::closure::WasmClosureFnOnce;

#[derive(Debug)]
enum MaybePromise<'a, T> {
    Blocking (Closure<dyn FnMut(T) -> Result<()>>, PhantomData<&'a mut &'a dyn FnMut(T) -> Result<()>>),
    Promise (Closure<dyn FnMut(T) -> js_sys::Promise>, PhantomData<&'a mut &'a dyn FnMut(T) -> js_sys::Promise>)
}

impl<T> AsRef<JsValue> for MaybePromise<'_, T> {
    #[inline]
    fn as_ref(&self) -> &JsValue {
        match self {
            Self::Blocking(x, _) => x.as_ref(),
            Self::Promise(x, _) => x.as_ref()
        }
    }
}

#[derive(Debug, Default)]
pub struct ReadBuilder<'a> {
    start: Option<MaybePromise<'a, ReadableStreamDefaultController>>,
    pull: Option<MaybePromise<'a, ReadableStreamDefaultController>>,
    cancel: Option<MaybePromise<'a, JsValue>>
}

impl<'a> ReadBuilder<'a> {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the stream source, and do anything else required to set up the stream functionality.
    #[inline]
    pub fn start<F: 'a + FnOnce(ReadStreamController) -> Result<()>> (mut self, f: F) -> Self {
        let f = move |inner| f(ReadStreamController { inner });
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(ReadableStreamDefaultController) -> Result<()>>,
                Box<dyn 'static + FnOnce(ReadableStreamDefaultController) -> Result<()>>,
            >(Box::new(f))
        };

        self.start = Some(MaybePromise::Blocking(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the stream source, and do anything else required to set up the stream functionality.
    #[inline]
    pub fn start_async<F: 'a + FnOnce(ReadStreamController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |inner| {
            let fut = f(ReadStreamController { inner }).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(ReadableStreamDefaultController) -> js_sys::Promise>,
                Box<dyn 'static + FnOnce(ReadableStreamDefaultController) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.start = Some(MaybePromise::Promise(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called repeatedly when the stream's internal queue of chunks is not full, up until it reaches its high water mark.
    #[inline]
    pub fn pull<F: 'a + FnMut(ReadStreamController) -> Result<()>> (mut self, mut f: F) -> Self {
        let f = move |inner| f(ReadStreamController { inner });
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnMut(ReadableStreamDefaultController) -> Result<()>>,
                Box<dyn 'static + FnMut(ReadableStreamDefaultController) -> Result<()>>,
            >(Box::new(f))
        };

        self.pull = Some(MaybePromise::Blocking(Closure::wrap(f), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called repeatedly when the stream's internal queue of chunks is not full, up until it reaches its high water mark.
    #[inline]
    pub fn pull_async<F: 'a + FnMut(ReadStreamController) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let f = move |inner| {
            let fut = f(ReadStreamController { inner }).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnMut(ReadableStreamDefaultController) -> js_sys::Promise>,
                Box<dyn 'static + FnMut(ReadableStreamDefaultController) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.pull = Some(MaybePromise::Promise(Closure::wrap(f), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that the stream is to be cancelled
    #[inline]
    pub fn cancel<F: 'a + FnOnce(JsValue) -> Result<()>> (mut self, f: F) -> Self {
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(JsValue) -> Result<()>>,
                Box<dyn 'static + FnOnce(JsValue) -> Result<()>>,
            >(Box::new(f))
        };

        self.cancel = Some(MaybePromise::Blocking(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that the stream is to be cancelled
    #[inline]
    pub fn cancel_async<F: 'a + FnOnce(JsValue) -> Fut, Fut: 'static + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let f = move |c| {
            let fut = f(c).map_ok(|_| JsValue::UNDEFINED);
            return wasm_bindgen_futures::future_to_promise(fut)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(JsValue) -> js_sys::Promise>,
                Box<dyn 'static + FnOnce(JsValue) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.cancel = Some(MaybePromise::Promise(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    pub fn build (self) -> Result<JsReadStream<'a>> {
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
    pub fn desired_size (&self) -> Option<f64> {
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

                impl<const N: usize> AsChunk for [$ty; N] {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        <[$ty]>::as_chunk(self)
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
                        let chunk = <$name>::from(self.0);
                        return chunk.into()
                    }
                }

                impl AsChunk for Clamped<Vec<$cty>> {
                    #[inline]
                    fn as_chunk (&self) -> JsValue {
                        let chunk = <$name>::from(&self.0 as &[$cty]);
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