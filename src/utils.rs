#![allow(unused)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{cell::{UnsafeCell, Cell}, mem::{MaybeUninit}, rc::{Rc, Weak}, task::{Waker, Poll, Context}, future::Future, ops::{Deref, DerefMut}, collections::VecDeque, fmt::{Debug, Display}, pin::Pin, io::ErrorKind};
use futures::{Stream, AsyncRead};
use serde::Deserialize;
use utils_atomics::{flag::{AsyncFlag, AsyncSubscribe}, TakeCell};
use wasm_bindgen::__rt::WasmRefCell;

const UNINIT: u8 = 0;
const WORKING: u8 = 1;
const INIT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct AssertSend<T> (T);

impl<T> AssertSend<T> {
    #[inline]
    pub fn new (t: T) -> Self where T: Send {
        return Self(t)
    }

    #[inline]
    pub unsafe fn new_unchecked (t: T) -> Self {
        return Self(t)
    }

    #[inline]
    pub fn into_inner (self) -> T {
        return self.0
    }
}

impl<T> Deref for AssertSend<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AssertSend<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T> Send for AssertSend<T> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct AssertSync<T> (T);

impl<T> AssertSync<T> {
    #[inline]
    pub fn new (t: T) -> Self where T: Sync {
        return Self(t)
    }

    #[inline]
    pub unsafe fn new_unchecked (t: T) -> Self {
        return Self(t)
    }

    #[inline]
    pub fn into_inner (self) -> T {
        return self.0
    }
}

impl<T> Deref for AssertSync<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AssertSync<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T> Sync for AssertSync<T> {}

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

struct ChannelInner<T> {
    buffer: VecDeque<T>,
    waker: Option<Waker>
}

pub struct LocalReceiver<T> {
    inner: Rc<WasmRefCell<ChannelInner<T>>>
}

impl<T> Stream for LocalReceiver<T> {
    type Item = T;

    #[inline]
    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner = self.inner.borrow_mut();

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
        let inner = self.inner.borrow();
        let size = inner.buffer.len();

        if Rc::weak_count(&self.inner) == 0 {
            return (size, Some(size))
        } else {
            return (size, None)
        }
    }
}

pub struct LocalSender<T> {
    inner: Weak<WasmRefCell<ChannelInner<T>>>
}

impl<T> LocalSender<T> {
    #[inline]
    pub fn send (&self, v: T) where T: Debug {
        self.try_send(v).unwrap()
    }
    
    #[inline]
    pub fn try_send (&self, v: T) -> ::core::result::Result<(), T> {
        if let Some(inner) = self.inner.upgrade() {
            let mut inner = inner.borrow_mut();
            inner.buffer.push_back(v);
            if let Some(waker) = inner.waker.take() { waker.wake() }
            return Ok(())
        }
        return Err(v)
    }
}

impl<T> Clone for LocalSender<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

#[inline]
pub fn local_channel<T> () -> (LocalSender<T>, LocalReceiver<T>) {
    let inner = Rc::new(WasmRefCell::new(ChannelInner {
        buffer: VecDeque::new(),
        waker: None
    }));

    return (LocalSender { inner: Rc::downgrade(&inner) }, LocalReceiver { inner });
}

pub struct ShotReceiver<T> {
    pub(crate) inner: Rc<FutureInner<T>>
}

impl<T> Future for ShotReceiver<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Some(geo) = self.inner.value.take() {
            return Poll::Ready(geo);
        }

        self.inner.waker.set(Some(cx.waker().clone()));
        return Poll::Pending
    }
}

pub struct ShotSender<T> {
    inner: Weak<FutureInner<T>>
}

impl<T> ShotSender<T> {
    #[inline]
    pub fn try_send (&self, v: T) -> Result<(), T> {
        if let Some(inner) = self.inner.upgrade() {    
            inner.value.set(Some(v));
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

#[inline]
pub(crate) fn one_shot<T> () -> (ShotSender<T>, ShotReceiver<T>) {
    let inner = Rc::new(FutureInner::default());
    return (ShotSender { inner: Rc::downgrade(&inner) }, ShotReceiver { inner })
}

pub(crate) struct FutureInner<T> {
    pub(crate) value: Cell<Option<T>>,
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

#[derive(Debug)]
struct BlockError;

impl Display for BlockError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&ErrorKind::WouldBlock, f)
    }
}

impl std::error::Error for BlockError {}