use futures::{Future};
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::{ReadableStreamDefaultController, WritableStreamDefaultController};
use crate::{Result, utils::{AbortHandle, Abortable}};
use super::*;
use core::marker::PhantomData;
use std::pin::Pin;
use wasm_bindgen::closure::WasmClosureFnOnce;
use crate::utils::AbortSignal;
use wasm_bindgen::JsCast;

#[derive(Debug)]
enum MaybePromise<'a, T> {
    Blocking (Closure<dyn FnMut<T, Output = Result<()>>>, PhantomData<&'a mut &'a dyn FnMut<T, Output = Result<()>>>),
    Promise (Closure<dyn FnMut<T, Output = js_sys::Promise>>, PhantomData<&'a mut (&'a (dyn 'a + Future<Output = Result<()>>), &'a dyn FnMut<T, Output = js_sys::Promise>)>)
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

#[derive(Debug)]
pub struct ReadBuilder<'a, T> {
    start: Option<MaybePromise<'a, (ReadableStreamDefaultController,)>>,
    pull: Option<MaybePromise<'a, (ReadableStreamDefaultController,)>>,
    cancel: Option<MaybePromise<'a, (JsValue,)>>,
    pub(super) handle: AbortHandle,
    _phtm: PhantomData<T>
}

impl<'a, T: JsCast> ReadBuilder<'a, T> {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the stream source, and do anything else required to set up the stream functionality.
    #[inline]
    pub fn start<F: 'a + FnOnce(ReadStreamController<T>) -> Result<()>> (mut self, f: F) -> Self {
        let f = move |inner| f(ReadStreamController { inner, _phtm: PhantomData });
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
    pub fn start_async<F: 'a + FnOnce(ReadStreamController<T>) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |inner| {
            return future_to_promise(Box::pin(f(ReadStreamController { inner, _phtm: PhantomData })), my_handle)
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
    pub fn pull<F: 'a + FnMut(ReadStreamController<T>) -> Result<()>> (mut self, mut f: F) -> Self {
        let f = move |inner| f(ReadStreamController { inner, _phtm: PhantomData });
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
    pub fn pull_async<F: 'a + FnMut(ReadStreamController<T>) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |inner| {
            return future_to_promise(Box::pin(f(ReadStreamController { inner, _phtm: PhantomData })), my_handle.clone())
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
    pub fn cancel_async<F: 'a + FnOnce(JsValue) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |c| {
            return future_to_promise(Box::pin(f(c)), my_handle)
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

    pub fn build (self) -> Result<JsReadStream<'a, T>> {
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


impl<T> Default for ReadBuilder<'_, T> {
    #[inline]
    fn default() -> Self {
        Self {
            start: Default::default(),
            pull: Default::default(),
            cancel: Default::default(),
            handle: Default::default(),
            _phtm: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct WriteBuilder<'a, T> {
    start: Option<MaybePromise<'a, (WritableStreamDefaultController,)>>,
    write: Option<MaybePromise<'a, (JsValue, WritableStreamDefaultController)>>,
    close: Option<MaybePromise<'a, (WritableStreamDefaultController,)>>,
    abort: Option<MaybePromise<'a, (JsValue,)>>,
    pub(super) handle: AbortHandle,
    _phtm: PhantomData<T>
}

impl<'a, T: JsCast> WriteBuilder<'a, T> {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the underlying sink.
    #[inline]
    pub fn start<F: 'a + FnOnce(WriteStreamController) -> Result<()>> (mut self, f: F) -> Self {
        let f = move |inner| f(WriteStreamController { inner });
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(WritableStreamDefaultController) -> Result<()>>,
                Box<dyn 'static + FnOnce(WritableStreamDefaultController) -> Result<()>>,
            >(Box::new(f))
        };

        self.start = Some(MaybePromise::Blocking(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This is a method, called immediately when the object is constructed. The contents of this method are defined by the developer, and should aim to get access to the underlying sink.
    #[inline]
    pub fn start_async<F: 'a + FnOnce(WriteStreamController) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |inner| {
            return future_to_promise(Box::pin(f(WriteStreamController { inner })), my_handle)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(WritableStreamDefaultController) -> js_sys::Promise>,
                Box<dyn 'static + FnOnce(WritableStreamDefaultController) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.start = Some(MaybePromise::Promise(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called when a new chunk of data (specified in the chunk parameter) is ready to be written to the underlying sink.
    #[inline]
    pub fn write<F: 'a + FnMut(T, WriteStreamController) -> Result<()>> (mut self, mut f: F) -> Self {
        let f = move |chunk, inner| {
            let chunk = JsCast::dyn_into::<T>(chunk)?;
            f(chunk, WriteStreamController { inner })
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnMut(JsValue, WritableStreamDefaultController) -> Result<()>>,
                Box<dyn 'static + FnMut(JsValue, WritableStreamDefaultController) -> Result<()>>,
            >(Box::new(f))
        };

        self.write = Some(MaybePromise::Blocking(Closure::wrap(f), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called when a new chunk of data (specified in the chunk parameter) is ready to be written to the underlying sink.
    #[inline]
    pub fn write_async<F: 'a + FnMut(T, WriteStreamController) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, mut f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |chunk: JsValue, inner| {
            let chunk = match JsCast::dyn_into::<T>(chunk) {
                Ok(x) => x,
                Err(e) => return js_sys::Promise::reject(&e),
            };

            return future_to_promise(Box::pin(f(chunk, WriteStreamController { inner })), my_handle.clone())
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnMut(JsValue, WritableStreamDefaultController) -> js_sys::Promise>,
                Box<dyn 'static + FnMut(JsValue, WritableStreamDefaultController) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.write = Some(MaybePromise::Promise(Closure::wrap(f), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that it has finished writing chunks to the stream. The contents should do whatever is necessary to finalize writes to the underlying sink, and release access to it.
    #[inline]
    pub fn close<F: 'a + FnOnce(WriteStreamController) -> Result<()>> (mut self, f: F) -> Self {
        let f = move |inner| f(WriteStreamController { inner });
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(WritableStreamDefaultController) -> Result<()>>,
                Box<dyn 'static + FnOnce(WritableStreamDefaultController) -> Result<()>>,
            >(Box::new(f))
        };

        self.close = Some(MaybePromise::Blocking(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that it has finished writing chunks to the stream. The contents should do whatever is necessary to finalize writes to the underlying sink, and release access to it.
    #[inline]
    pub fn close_async<F: 'a + FnOnce(WriteStreamController) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |inner| {
            return future_to_promise(Box::pin(f(WriteStreamController { inner })), my_handle)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(WritableStreamDefaultController) -> js_sys::Promise>,
                Box<dyn 'static + FnOnce(WritableStreamDefaultController) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.close = Some(MaybePromise::Promise(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that it wishes to abruptly close the stream and put it in an errored state. It can clean up any held resources, much like close(), but abort() will be called even if writes are queued up — those chunks will be thrown away.
    #[inline]
    pub fn abort<F: 'a + FnOnce(JsValue) -> Result<()>> (mut self, f: F) -> Self {
        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(JsValue) -> Result<()>>,
                Box<dyn 'static + FnOnce(JsValue) -> Result<()>>,
            >(Box::new(f))
        };

        self.abort = Some(MaybePromise::Blocking(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    /// This method, also defined by the developer, will be called if the app signals that it wishes to abruptly close the stream and put it in an errored state. It can clean up any held resources, much like close(), but abort() will be called even if writes are queued up — those chunks will be thrown away.
    #[inline]
    pub fn abort_async<F: 'a + FnOnce(JsValue) -> Fut, Fut: 'a + Future<Output = Result<()>>> (mut self, f: F) -> Self {
        let my_handle = self.handle.clone();
        let f = move |c| {
            return future_to_promise(Box::pin(f(c)), my_handle)
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn 'a + FnOnce(JsValue) -> js_sys::Promise>,
                Box<dyn 'static + FnOnce(JsValue) -> js_sys::Promise>,
            >(Box::new(f))
        };

        self.abort = Some(MaybePromise::Promise(Closure::wrap(f.into_fn_mut()), PhantomData));
        self
    }

    pub fn build (self) -> Result<JsWriteStream<'a, T>> {
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

        if let Some(ref write) = self.write {
            set! { underlying_source["write"] = write.as_ref(); }
        }

        if let Some(ref close) = self.close {
            set! { underlying_source["close"] = close.as_ref(); }
        }

        if let Some(ref abort) = self.abort {
            set! { underlying_source["abort"] = abort.as_ref(); }
        }

        let stream = web_sys::WritableStream::new_with_underlying_sink(&underlying_source)?;
        let mut result = JsWriteStream::new(stream)?;
        result._builder = Some(self);
        return Ok(result)
    }
}

impl<T> Default for WriteBuilder<'_, T> {
    #[inline]
    fn default() -> Self {
        Self {
            start: Default::default(),
            write: Default::default(),
            close: Default::default(),
            abort: Default::default(),
            handle: Default::default(),
            _phtm: Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReadStreamController<T: ?Sized> {
    inner: ReadableStreamDefaultController,
    _phtm: PhantomData<T>
}

impl<T: ?Sized> ReadStreamController<T> {
    #[inline]
    pub fn close (&self) -> Result<()> {
        self.inner.close()
    }

    #[inline]
    pub fn desired_size (&self) -> Option<f64> {
        self.inner.desired_size()
    }

    #[inline]
    pub fn enqueue (&self, chunk: &T) -> Result<()> where T: AsRef<JsValue> {
        return self.inner.enqueue_with_chunk(chunk.as_ref())
    }

    #[inline]
    pub fn error (&self, e: Option<&JsValue>) {
        match e {
            Some(e) => self.inner.error_with_e(e),
            None => self.inner.error()
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteStreamController {
    inner: WritableStreamDefaultController
}

impl WriteStreamController {
    #[inline]
    pub fn raw_signal (&self) -> web_sys::AbortSignal {
        return self.inner.signal()
    }

    #[inline]
    pub fn signal<T> (&self) -> Result<AbortSignal<T>> {
        return AbortSignal::new(self.inner.signal())
    }

    #[inline]
    pub fn error (&self, e: Option<&JsValue>) {
        match e {
            Some(e) => self.inner.error_with_e(e),
            None => self.inner.error()
        }
    }
}

fn future_to_promise<'a> (fut: Pin<Box<dyn 'a + Future<Output = Result<()>>>>, handle: AbortHandle) -> js_sys::Promise {
    let fut: Pin<Box<dyn 'static + Future<Output = Result<()>>>> = unsafe { core::mem::transmute(fut) };
    let fut = Abortable::new(fut, handle);

    // Awaiting https://github.com/rustwasm/wasm-bindgen/pull/3193
    return wasm_bindgen_futures::future_to_promise(async move {
        match fut.await {
            Ok(Ok(_)) | Err(_) => return Ok(JsValue::UNDEFINED),
            Ok(Err(e)) => return Err(e) 
        }
    });
}