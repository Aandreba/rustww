#![allow(unused)]

use std::{cell::{UnsafeCell, Cell}, mem::{MaybeUninit}, rc::{Rc}, task::{Waker, Poll}, future::Future};
use utils_atomics::{flag::{AsyncFlag, AsyncSubscribe}, TakeCell};

const UNINIT: u8 = 0;
const WORKING: u8 = 1;
const INIT: u8 = 2;

pub struct OnceCell<T> {
    sub: AsyncSubscribe,
    flag: TakeCell<AsyncFlag>,
    inner: UnsafeCell<MaybeUninit<T>>
}

impl<T> OnceCell<T> {
    pub fn new () -> Self {
        let flag = AsyncFlag::new();
        let sub = flag.subscribe();

        Self {
            sub,
            flag: TakeCell::new(flag),
            inner: UnsafeCell::new(MaybeUninit::uninit())
        }
    }

    #[inline]
    pub fn try_set (&self, v: T) -> Result<(), T> {
        if let Some(flag) = self.flag.try_take() {
            unsafe {
                (&mut *self.inner.get()).write(v);
                flag.mark();
                return Ok(())
            }
        }

        return Err(v)
    }

    #[inline]
    pub async fn get (&self) -> &T {
        self.sub.clone().await;
        return unsafe { (&*self.inner.get()).assume_init_ref() }
    }
}

unsafe impl<T: Send> Send for OnceCell<T> {}
unsafe impl<T: Sync> Sync for OnceCell<T> {}

pub struct OneShot<T> {
    pub(crate) inner: Option<Rc<FutureInner<T>>>
}

impl<T> OneShot<T> {
    #[inline]
    pub(crate) fn new () -> (Self, Sender<T>) {
        let inner = Rc::new(FutureInner::default());
        return (Self {
            inner: Some(inner.clone())
        }, Sender { inner })
    }
}

impl<T> Future for OneShot<T> {
    type Output = T;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Some(ref mut inner) = self.inner {
            if let Some(geo) = inner.value.take() {
                self.inner = None;
                return Poll::Ready(geo);
            }
    
            inner.waker.set(Some(cx.waker().clone()));
            return Poll::Pending
        }

        panic!("Value already extracted")
    }
}

pub struct Sender<T> {
    inner: Rc<FutureInner<T>>
}

impl<T> Sender<T> {
    #[inline]
    pub fn try_send (&self, v: T) -> Result<(), T> {
        if self.inner.sent.get() { return Err(v) }

        self.inner.sent.set(true);
        self.inner.value.set(Some(v));

        if let Some(waker) = self.inner.waker.take() {
            waker.wake();
        }

        return Ok(())
    }
}

impl<T> Clone for Sender<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

pub(crate) struct FutureInner<T> {
    pub(crate) sent: Cell<bool>,
    pub(crate) value: Cell<Option<T>>,
    pub(crate) waker: Cell<Option<Waker>>
}

impl<T> Default for FutureInner<T> {
    #[inline]
    fn default() -> Self {
        Self {
            sent: Cell::new(false),
            value: Default::default(),
            waker: Default::default()
        }
    }
}