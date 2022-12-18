use std::{time::Duration, intrinsics::unlikely, fmt::Debug, mem::ManuallyDrop, marker::PhantomData, task::Poll};
use futures::{Stream, StreamExt, Future, FutureExt};
use js_sys::{Function};
use wasm_bindgen::{JsValue, prelude::Closure, JsCast, closure::WasmClosureFnOnce};
use crate::{Result, utils::{LocalReceiver, local_channel}, sync::{ShotReceiver, one_shot}};
use crate::scope::*;
const MAX_MILLIS: u128 = i32::MAX as u128;

/// An owned handler of an interval
/// 
/// An interval is a closure thet is executed repeatedly after a specified delay, without blocking.
/// 
/// Interval handlers contain the data related to their closure, and when dropped will clear the interval 
/// and release any memory relating to their closure.
/// 
/// They also receive the return value of each call to the closure, so they can be used as a Rust [`Stream`]
pub struct Interval<'a, T> {
    id: i32,
    #[cfg(target_feature = "atomics")]
    recv: async_channel::Receiver<T>,
    #[cfg(not(target_feature = "atomics"))]
    recv: LocalReceiver<T>,
    #[cfg(target_feature = "atomics")]
    _closure: DropHandle,
    #[cfg(not(target_feature = "atomics"))]
    _closure: Closure<dyn FnMut()>,
    _phtm: PhantomData<&'a mut &'a dyn FnMut()>
}

impl<'a, T: 'a> Interval<'a, T> {
    /// Creates a new handled interval with the specified callback and timeout.
    /// 
    /// If you want to create a permanently living interval, use [`spawn_interval`] or [`Interval::leak`]
    pub fn new<F: 'a + FnMut() -> T> (timeout: Duration, mut f: F) -> Result<Self> {
        let millis = timeout.as_millis();
        if unlikely(millis > MAX_MILLIS) {
            return Err(JsValue::from_str("timeout overflow"))
        }
    
        #[cfg(target_feature = "atomics")]
        let (send, recv) = async_channel::unbounded();
        #[cfg(not(target_feature = "atomics"))]
        let (send, recv) = local_channel();
        let mut send = Some(send);

        let f = move || {
            let v = f();
            if let Some(ref current_send) = send {
                if current_send.try_send(v).is_err() {
                    // This helps free memory, because we're droping the weak reference
                    send = None;
                }
            }
        };

        let f = unsafe {
            core::mem::transmute::<Box<dyn 'a + FnMut()>, Box<dyn 'static + FnMut()>>(Box::new(f))
        };

        let closure = Closure::wrap(f);
        let handler = closure.as_ref();
        debug_assert!(handler.is_instance_of::<Function>());
    
        let id = set_interval(handler.unchecked_ref(), millis as i32)?;
        #[cfg(target_feature = "atomics")]
        let closure = drop_local(closure);

        return Ok(Self {
            id,
            recv,
            _closure: closure,
            _phtm: PhantomData
        })
    }
}

impl<T> Interval<'_, T> {
    /// Returns the id of the interval
    #[inline]
    pub fn id (&self) -> i32 {
        return self.id
    }
}

impl<T> Interval<'static, T> {
    /// Leaks the interval, releasing memory management of the closure to the JavaScrpt GC.
    #[inline]
    pub fn leak (this: Self) -> impl Stream<Item = T> {
        let this = ManuallyDrop::new(this);
        return unsafe { core::ptr::read(&this.recv) }
    }
}

impl<T> Stream for Interval<'_, T> {
    type Item = T;

    #[inline]
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        self.recv.poll_next_unpin(cx)
    }
}

impl<T> Debug for Interval<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interval")
            .field("id", &self.id)
            .field("closure", &self._closure)
            .finish()
    }
}

impl<T> Drop for Interval<'_, T> {
    #[inline]
    fn drop (&mut self) {
        clear_interval(self.id);
    }
}

/// Spawns an interval directly into JavaScript memory management, leaking any Rust memory related to it,
/// and returning a [`Stream`] that returns the result of each interval.
#[inline]
pub fn spawn_interval<T: 'static, F: 'static + FnMut() -> T> (timeout: Duration, f: F) -> Result<impl Stream<Item = T>> {
    let recv = Interval::leak(Interval::new(timeout, f)?);
    return Ok(recv)
}

/// An owned handler of a timeout.
/// 
/// A timeout is a closure that is executed only once, after a specified delay, without blocking.
/// 
/// Timeout handlers contain the data related to their closure, and when dropped will clear the timeout 
/// and release any memory relating to their closure.
/// 
/// They also receive the return value of the closure, so they can be used as a Rust [`Future`]
pub struct Timeout<'a, T> {
    id: i32,
    recv: ShotReceiver<T>,
    #[cfg(target_feature = "atomics")]
    _closure: DropHandle,
    #[cfg(not(target_feature = "atomics"))]
    _closure: Closure<dyn FnMut()>,
    _phtm: PhantomData<&'a mut &'a dyn FnMut()>
}

impl<'a, T: 'a> Timeout<'a, T> {
    /// Creates a new timout, returning it's handle
    pub fn new<F: 'a + FnOnce() -> T> (timeout: Duration, f: F) -> Result<Self> {
        let millis = timeout.as_millis();
        if unlikely(millis > MAX_MILLIS) {
            return Err(JsValue::from_str("timeout overflow"))
        }

        let (send, recv) = one_shot::<T>();
        let f = move || send.send(f());

        let f = unsafe {
            core::mem::transmute::<Box<dyn 'a + FnOnce()>, Box<dyn 'static + FnOnce()>>(Box::new(f))
        };

        let closure = Closure::wrap(f.into_fn_mut());
        let function = closure.as_ref();
        debug_assert!(function.is_instance_of::<Function>());

        let id = set_timeout(function.unchecked_ref(), millis as i32)?;
        #[cfg(target_feature = "atomics")]
        let closure = drop_local(closure);

        return Ok(Self {
            id,
            recv,
            _closure: closure,
            _phtm: PhantomData
        })
    }
}

impl<T> Timeout<'_, T> {
    /// Returns the id of the timeout
    #[inline]
    pub fn id (&self) -> i32 {
        return self.id
    }
}

impl<T> Timeout<'static, T> {
    /// Returns a future that resolves when the timeout executes.
    /// Unlike [`Timeout`], this future will not clear the interval when droped.
    #[inline]
    pub fn leak (this: Self) -> ShotReceiver<T> {
        let this = ManuallyDrop::new(this);
        return unsafe { core::ptr::read(&this.recv) }
    }
}

impl<T> Future for Timeout<'_, T> {
    type Output = T;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        return match self.recv.poll_unpin(cx) {
            Poll::Ready(Some(x)) => Poll::Ready(x),
            Poll::Ready(None) => panic!("the timeout completed without returning a value"),
            Poll::Pending => Poll::Pending
        }
    }
}

impl<T> Drop for Timeout<'_, T> {
    #[inline]
    fn drop(&mut self) {
        clear_timeout(self.id);
    }
}

/// Returns a [`Future`] that resolves after a specified delay.
#[inline]
pub fn sleep (dur: Duration) -> Result<Sleep> {
    return Ok(Sleep {
        timeout: Timeout::new(dur, <()>::default)?
    })
}

pub struct Sleep {
    timeout: Timeout<'static, ()>
}

impl Future for Sleep {
    type Output = ();

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.timeout.poll_unpin(cx)
    }
}

/*
/// Returns a [`Future`] that resolves after a specified delay.
#[cfg(not(target_feature = "atomics"))]
#[inline]
pub async fn sleep (dur: Duration) {
    let millis = dur.as_millis();
    let mut f = |resolve: Function, reject: Function| {
        if unlikely(millis > MAX_MILLIS) {
            match reject.call1(&JsValue::UNDEFINED, &JsValue::from_str("Duration overflow")) {
                Ok(_) => {},
                Err(e) => {
                    drop(resolve);
                    drop(reject);
                    wasm_bindgen::throw_val(e)
                }
            }
        }

        match set_timeout(&resolve, millis as i32) {
            Ok(_) => {},
            Err(e) => match reject.call1(&JsValue::UNDEFINED, &e) {
                Ok(_) => {},
                Err(e) => {
                    drop(resolve);
                    drop(reject);
                    wasm_bindgen::throw_val(e);
                }
            }
        }
    };

    let promise = Promise::new(&mut f);
    let _ = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .unwrap_throw();
}
*/