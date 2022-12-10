#![allow(unused)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{cell::{UnsafeCell, Cell}, mem::{MaybeUninit}, rc::{Rc, Weak}, task::{Waker, Poll, Context}, future::Future, ops::{Deref, DerefMut}, collections::VecDeque, fmt::{Debug, Display}, pin::Pin, io::ErrorKind, marker::{PhantomPinned, PhantomData}, any::{Any, TypeId}};
use futures::{Stream, AsyncRead};
use js_sys::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utils_atomics::{flag::{AsyncFlag, AsyncSubscribe}, TakeCell};
use wasm_bindgen::{__rt::WasmRefCell, prelude::*, closure::WasmClosureFnOnce, JsStatic, JsCast};

use crate::Result;

const UNINIT: u8 = 0;
const WORKING: u8 = 1;
const INIT: u8 = 2;

mod sealed { pub trait Sealed {} }

struct ChannelInner<T> {
    buffer: VecDeque<T>,
    waker: Option<Waker>
}

/// A local channel's receiver, designed to receive values from or inside JavaScript callbacks.
pub struct LocalReceiver<T> {
    inner: Rc<UnsafeCell<ChannelInner<T>>>
}

impl<T> Stream for LocalReceiver<T> {
    type Item = T;

    #[inline]
    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner = unsafe { &mut *self.inner.get() };

        if let Some(value) = inner.buffer.pop_front() {
            return Poll::Ready(Some(value))
        } else if Rc::weak_count(&self.inner) == 0 {
            return Poll::Ready(None)
        }

        inner.waker = Some(cx.waker().clone());
        return Poll::Pending
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let inner = unsafe { &*self.inner.get() };
        let size = inner.buffer.len();

        if Rc::weak_count(&self.inner) == 0 {
            return (size, Some(size))
        } else {
            return (size, None)
        }
    }
}

/// A local channel's sender, designed to send values from or into JavaScript callbacks.
pub struct LocalSender<T> {
    inner: Weak<UnsafeCell<ChannelInner<T>>>
}

impl<T> LocalSender<T> {
    /// Attempts to send the value through the channel, panicking if it fails.
    #[inline]
    pub fn send (&self, v: T) where T: Debug {
        self.try_send(v).unwrap()
    }
    
    /// Attempts to send the value through the channel, returning it if it fails.
    /// 
    /// A send through the channel will fail if there are no receivers left.
    #[inline]
    pub fn try_send (&self, v: T) -> ::core::result::Result<(), T> {
        if let Some(inner) = self.inner.upgrade() {
            let mut inner = unsafe { &mut *inner.get() };
            inner.buffer.push_back(v);
            if let Some(waker) = inner.waker.take() { waker.wake() }
            return Ok(())
        }

        // There are no more recievers
        return Err(v)
    }
}

impl<T> Clone for LocalSender<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// Creates a new local channel, which can be used to send values from (and into) JavaScript callbacks,
/// which are known to be executed on a single thread, but aren't attached to a specific scope. 
#[inline]
pub fn local_channel<T> () -> (LocalSender<T>, LocalReceiver<T>) {
    let inner = Rc::new(UnsafeCell::new(ChannelInner {
        buffer: VecDeque::new(),
        waker: None
    }));

    return (LocalSender { inner: Rc::downgrade(&inner) }, LocalReceiver { inner });
}

/// Receiver to a local one-shot channel
pub struct ShotReceiver<T> {
    pub(crate) inner: Rc<FutureInner<T>>
}

impl<T> Future for ShotReceiver<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Some(geo) = self.inner.value.take() {
            return Poll::Ready(geo.expect("Value has already been taken"));
        }

        self.inner.waker.set(Some(cx.waker().clone()));
        return Poll::Pending
    }
}

/// Sender of a local one-shot channel
pub struct ShotSender<T> {
    inner: Weak<FutureInner<T>>
}

impl<T> ShotSender<T> {
    /// Attempts to send the value through the channel, returning it if it fails.
    /// 
    /// A send through the channel fails of a value has already been sent through it.
    #[inline]
    pub fn try_send (&self, v: T) -> ::core::result::Result<(), T> {
        if let Some(inner) = self.inner.upgrade() {    
            inner.value.set(Some(Some(v)));
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
            return Ok(())
        }
        return Err(v)
    }
}

impl<T> Clone for ShotSender<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// Creates a new local one-shot channel, which is optimized to be able to send a single value.
#[inline]
pub fn one_shot<T> () -> (ShotSender<T>, ShotReceiver<T>) {
    let inner = Rc::new(FutureInner::default());
    return (ShotSender { inner: Rc::downgrade(&inner) }, ShotReceiver { inner })
}

pub(crate) struct FutureInner<T> {
    pub(crate) value: Cell<Option<Option<T>>>,
    pub(crate) waker: Cell<Option<Waker>>
}

impl<T> Default for FutureInner<T> {
    #[inline]
    fn default() -> Self {
        Self {
            value: Default::default(),
            waker: Default::default()
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    #[wasm_bindgen(js_name = AbortController, extends = web_sys::AbortController)]
    type AbortControllerExt;

    #[derive(Debug, Clone)]
    #[wasm_bindgen(js_name = AbortSignal, extends = web_sys::AbortSignal)]
    type AbortSignalExt;

    #[wasm_bindgen(method)]
    fn abort (this: &AbortControllerExt, reason: &JsValue);
    #[wasm_bindgen(method)]
    fn reason (this: &AbortSignalExt) -> JsValue;
}

/// Creates a new abortion flag
#[inline]
pub fn abort<T> () -> Result<(AbortController<T>, AbortSignal<T>)> {
    let con = AbortController::new()?;
    let sig = con.signal()?;
    return Ok((con, sig))
}

/// Represents a controller object that allows you to abort one or more Web requests as and when desired.
#[derive(Debug, Clone)]
pub struct AbortController<T> {
    inner: AbortControllerExt,
    _phtm: PhantomData<T>
}

impl<T> AbortController<T> {
    /// Creates a new [`AbortController`]
    #[inline]
    pub fn new () -> Result<Self> {
        return Ok(Self {
            inner: web_sys::AbortController::new()?.unchecked_into(),
            _phtm: PhantomData
        })
    }

    /// Sends the abortion signal with the specified reason
    #[inline]
    pub fn abort (&self, reason: &T) -> Result<()> where T: Serialize {
        let reason = serde_wasm_bindgen::to_value(reason)?;
        self.inner.abort(reason.as_ref());
        Ok(())
    }

    /// Sends the abortion signal, casting the specified reason as a [`JsValue`]
    #[inline]
    pub fn abort_cast (&self, reason: &T) where T: AsRef<JsValue> {
        self.inner.abort(reason.as_ref())
    }

    /// Returns a raw [`web_sys::AbortSignal`] for the controller.
    #[inline]
    pub fn raw_signal (&self) -> web_sys::AbortSignal {
        return self.inner.signal()
    }

    /// Returns an [`AbortSignal`] for the controller.
    #[inline]
    pub fn signal (&self) -> Result<AbortSignal<T>> {
        return AbortSignal::new(self.inner.signal())
    }
}

impl<T> From<web_sys::AbortController> for AbortController<T> {
    #[inline]
    fn from(value: web_sys::AbortController) -> Self {
        return Self {
            inner: value.unchecked_into(),
            _phtm: PhantomData
        }
    }
}

/// Represents a signal object that allows you to communicate with a DOM request (such as a fetch request) and abort it if required via an [`AbortController`].
pub struct AbortSignal<T> {
    inner: AbortSignalExt,
    _list: Closure<dyn FnMut()>, 
    waker: Pin<Box<Cell<Option<Waker>>>>,
    _phtm: PhantomData<T>
}

impl<T> AbortSignal<T> {
    pub fn new (inner: web_sys::AbortSignal) -> Result<Self> {
        let waker = Box::pin(Cell::new(None::<Waker>));
        let f = || if let Some(waker) = waker.take() {
            waker.wake();
        };

        let f = unsafe {
            core::mem::transmute::<
                Box<dyn FnOnce()>,
                Box<dyn 'static + FnOnce()>
            >(Box::new(f))
        };

        let _list = Closure::wrap(f.into_fn_mut());
        
        let fun = _list.as_ref();
        debug_assert!(fun.is_instance_of::<js_sys::Function>());
        inner.add_event_listener_with_callback("abort", fun.unchecked_ref())?;

        return Ok(Self {
            inner: inner.unchecked_into(),
            _list,
            waker,
            _phtm: PhantomData
        })
    }

    /// Returns `true` if the signal is aborted, `false` otherwise
    #[inline]
    pub fn is_aborted (&self) -> bool {
        return self.inner.aborted()
    }

    #[inline]
    pub fn reason (&self) -> Result<Option<T>> where T: DeserializeOwned {
        let reason = self.inner.reason();
        if reason.is_undefined() {
            return Ok(None)
        }

        let v = serde_wasm_bindgen::from_value(reason)?;
        return Ok(v)
    }

    #[inline]
    pub fn reason_cast (&self) -> Result<Option<T>> where T: JsCast {
        let reason = self.inner.reason();
        if reason.is_undefined() {
            return Ok(None)
        }
        return JsCast::dyn_into::<T>(reason).map(Some)
    }

    #[inline]
    pub fn try_clone (&self) -> Result<Self> {
        return Self::new(self.inner.clone().unchecked_into())
    }
}

impl<T> AsRef<web_sys::AbortSignal> for AbortSignal<T> {
    #[inline]
    fn as_ref(&self) -> &web_sys::AbortSignal {
        &self.inner
    }
}

impl<T: DeserializeOwned> Future for AbortSignal<T> {
    type Output = Result<T>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_aborted() {
            let v = serde_wasm_bindgen::from_value::<T>(self.inner.reason())?;
            return Poll::Ready(Ok(v))
        }

        Cell::set(&self.waker, Some(cx.waker().clone()));
        return Poll::Pending
    }
}

impl<T> Debug for AbortSignal<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbortSignal")
            .field("aborted", &self.is_aborted())
            .finish()
    }
}

impl<T> Clone for AbortSignal<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

impl<T> Drop for AbortSignal<T> {
    #[inline]
    fn drop(&mut self) {
        let fun = self._list.as_ref();
        debug_assert!(fun.is_instance_of::<js_sys::Function>());
        self.inner.remove_event_listener_with_callback("abort", fun.unchecked_ref()).unwrap();
    }
}

/// Represents a JavaScript typed array
pub trait TypedArray: sealed::Sealed + AsRef<JsValue> {
    fn buffer (&self) -> ArrayBuffer;
    fn byte_length (&self) -> u32;
    fn byte_offset (&self) -> u32;
    fn length (&self) -> u32;

    #[inline]
    fn as_bytes (&self) -> Uint8Array {
        return Uint8Array::new_with_byte_offset_and_length(
            &self.buffer(),
            self.byte_offset(),
            self.byte_length()
        )
    }
}

/// [`TypedArray`] trait extansion
pub trait TypedArrayExt: TypedArray + JsCast {
    type Element;

    fn bytes_per_element () -> u32;
    fn copy_to (&self, dst: &mut [Self::Element]);
    fn copy_from (&self, src: &[Self::Element]);
    fn to_vec (&self) -> Vec<Self::Element>;

    unsafe fn view (v: &[Self::Element]) -> Self;
    unsafe fn view_mut_raw (ptr: *mut Self::Element, length: usize) -> Self;
    unsafe fn raw_copy_to_ptr (&self, ptr: *mut Self::Element);
}

macro_rules! impl_typed_array {
    ($($name:ident as [$t:ty]),+) => {
        $(
            impl TypedArray for $name {
                #[inline]
                fn buffer (&self) -> ArrayBuffer {
                    <$name>::buffer(self)
                }

                #[inline]
                fn byte_length (&self) -> u32 {
                    <$name>::byte_length(self)
                }

                #[inline]
                fn byte_offset (&self) -> u32 {
                    <$name>::byte_offset(self)
                }

                #[inline]
                fn length (&self) -> u32 {
                    <$name>::length(self)
                }
            }

            impl TypedArrayExt for $name {
                type Element = $t;

                /// There is some kind of bug between thread locals and documentation 
                #[cfg(docsrs)]
                #[inline]
                fn bytes_per_element () -> u32 {
                    <$t>::BITS / 8
                }
                
                #[cfg(not(docsrs))]
                #[inline]
                fn bytes_per_element () -> u32 {
                    #[wasm_bindgen]
                    extern {
                        #[wasm_bindgen(js_namespace = $name)]
                        static BYTES_PER_ELEMENT: u32;
                    }
                    *BYTES_PER_ELEMENT
                }

                #[inline]
                fn copy_to (&self, dst: &mut [Self::Element]) {
                    <$name>::copy_to(self, dst)
                }
                
                #[inline]
                fn copy_from (&self, src: &[Self::Element]) {
                    <$name>::copy_from(self, src)
                }

                #[inline]
                fn to_vec (&self) -> Vec<Self::Element> {
                    <$name>::to_vec(self)
                }

                #[inline]
                unsafe fn view (v: &[Self::Element]) -> Self {
                    <$name>::view(v)
                }

                #[inline]
                unsafe fn view_mut_raw (ptr: *mut Self::Element, length: usize) -> Self {
                    <$name>::view_mut_raw(ptr, length)
                }

                #[inline]
                unsafe fn raw_copy_to_ptr (&self, ptr: *mut Self::Element) {
                    <$name>::raw_copy_to_ptr(self, ptr)
                }
            }

            impl sealed::Sealed for $name {}
        )+

        #[inline]
        pub(crate) fn as_typed_array (this: &JsValue) -> Option<&dyn TypedArray> {
            use wasm_bindgen::JsCast;
            $(
                if this.is_instance_of::<$name>() {
                    return Some(this.unchecked_ref::<$name>());
                }
            )+
            return None;
        }
    };
}

impl_typed_array! {
    Uint8Array as [u8],
    Uint8ClampedArray as [u8],
    Uint16Array as [u16],
    Uint32Array as [u32],
    BigUint64Array as [u64],

    Int8Array as [i8],
    Int16Array as [i16],
    Int32Array as [i32],
    BigInt64Array as [i64],

    Float32Array as [f32],
    Float64Array as [f64]
}