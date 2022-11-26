use std::{sync::{atomic::AtomicUsize}, ptr::NonNull, mem::{ManuallyDrop, MaybeUninit}, ops::Deref};
use js_sys::Function;
use utils_atomics::{flag::AsyncFlag};
use wasm_bindgen::{JsCast, closure::{WasmClosure, IntoWasmClosure}, prelude::Closure};

struct SendableInner<T> {
    refs: AtomicUsize,
    flag: MaybeUninit<AsyncFlag>,
    v: ManuallyDrop<T>
}

impl<T> SendableInner<T> {
    #[inline]
    pub fn decrement (&self) {
        if self.refs.fetch_sub(1, std::sync::atomic::Ordering::AcqRel) == 1 {
            unsafe { self.flag.assume_init_read().mark() }
        }
    }
}

pub struct Syncable<T: 'static> {
    inner: NonNull<SendableInner<T>>
}

pub struct SyncableRef<T> {
    inner: NonNull<SendableInner<T>>
}

impl<T: JsCast> Syncable<T> {
    #[inline]
    pub fn new (t: T) -> Self {
        let inner = SendableInner {
            refs: AtomicUsize::new(1),
            flag: MaybeUninit::new(AsyncFlag::new()),
            v: ManuallyDrop::new(t),
        };

        let inner = Box::into_raw(Box::new(inner));
        return Self { inner: unsafe { NonNull::new_unchecked(inner) } }
    }

    #[inline]
    pub fn sync (&self) -> SyncableRef<T> {
        unsafe { self.inner.as_ref().refs.fetch_add(1, std::sync::atomic::Ordering::AcqRel); }
        return SyncableRef { inner: self.inner }
    }
}

impl<T> Deref for Syncable<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
       return unsafe { &self.inner.as_ref().v }
    }
}

impl<T> Clone for SyncableRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            self.inner.as_ref().refs.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
        };
        Self { inner: self.inner.clone() }
    }
}

impl<T: 'static> Drop for Syncable<T> {
    #[inline]
    fn drop(&mut self) {
        let mut inner = self.inner;
        let sub = unsafe { inner.as_ref().flag.assume_init_ref().subscribe() };

        wasm_bindgen_futures::spawn_local(async move {
            sub.await;
            unsafe { 
                ManuallyDrop::drop(&mut inner.as_mut().v);
                let _ = Box::from_raw(inner.as_ptr());
            }
        });

        unsafe { self.inner.as_ref().decrement() }
    }
}

impl<T> Drop for SyncableRef<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.inner.as_ref().decrement() }
    }
}

unsafe impl<T> Send for SyncableRef<T> {}
unsafe impl<T> Sync for SyncableRef<T> {}

pub struct SyncableClosure<T: ?Sized> {
    value: ManuallyDrop<SyncableRef<Function>>,
    closure: ManuallyDrop<Box<T>>
}

impl<T: ?Sized> SyncableClosure<T> {
    #[inline]
    pub fn new (value: SyncableRef<Function>, closure: Box<T>) -> Self {
        Self { 
            value: ManuallyDrop::new(value),
            closure: ManuallyDrop::new(closure)
        }
    }

    #[inline]
    pub unsafe fn function (&self) -> &Function {
        return unsafe { &self.value.inner.as_ref().v };
    }

    #[inline]
    pub fn closure (&self) -> &T {
        return &self.closure
    }

    #[inline]
    pub fn into_parts (self) -> (SyncableRef<Function>, Box<T>) {
        let this = ManuallyDrop::new(self);
        return unsafe {
            (core::ptr::read(this.value.deref()), core::ptr::read(this.closure.deref()))
        };
    }
}

impl<T: ?Sized> Drop for SyncableClosure<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.value);
            ManuallyDrop::drop(&mut self.closure);
        }
    }
}

#[inline]
pub unsafe fn syncable_closure<T: 'static + ?Sized + WasmClosure, F: IntoWasmClosure<T> + 'static> (f: F) -> (Syncable<Function>, Box<F>) {
    let f = Box::new(f);
    return (syncable_wrapped_closure(&f), f);
}

pub unsafe fn syncable_wrapped_closure<T: 'static + ?Sized + WasmClosure, F: IntoWasmClosure<T> + 'static> (f: &Box<F>) -> Syncable<Function> {
    // SAFETY: This box will be forgoten by `into_js_value`, so no double-free will occurr
    let dummy_f = unsafe { Box::from_raw(f.deref() as *const F as *mut F).unsize() };
    let closure = Closure::wrap(dummy_f);

    let value = closure.into_js_value();
    debug_assert!(value.is_instance_of::<Function>());
    return Syncable::new(value.unchecked_into())
}