use std::{time::Duration, intrinsics::unlikely, fmt::Debug, mem::ManuallyDrop};
use futures::{Stream, StreamExt};
use js_sys::{Promise, Function};
use wasm_bindgen::{JsValue, prelude::Closure, JsCast};
use crate::{Result, window, utils::{LocalReceiver, local_channel, ShotReceiver, one_shot}};

const MAX_MILLIS: u128 = i32::MAX as u128;

/// An owned handler of an interval
/// 
/// Interval handlers contain the data related to their closure, and will clear the interval when dropped.
/// They also receive the return value of each call to their closure.
pub struct Interval<T> {
    id: i32,
    recv: LocalReceiver<T>,
    #[allow(unused)]
    closure: Closure<dyn FnMut()>
}

impl<T: 'static> Interval<T> {
    /// Creates a new handled interval with the specified callback and timeout.
    /// 
    /// If you want to create a permanently living interval, use [`spawn_interval`] or [`Interval::leak`]
    pub fn new<F: 'static + FnMut() -> T> (timeout: Duration, mut f: F) -> Result<Self> {
        let millis = timeout.as_millis();
        if unlikely(millis > MAX_MILLIS) {
            return Err(JsValue::from_str("timeout overflow"))
        }
    
        let (send, recv) = local_channel();
        let f = move || {
            let _ = send.try_send(f());
        };

        let closure = Closure::new(f);
        let handler = closure.as_ref();
        debug_assert!(handler.is_instance_of::<Function>());
    
        let id = window()?.set_interval_with_callback_and_timeout_and_arguments_0(handler.unchecked_ref(), millis as i32)?;
        return Ok(Self { id, recv, closure })
    }
}

impl<T> Interval<T> {
    /// Returns the id of the interval
    #[inline]
    pub fn id (&self) -> i32 {
        return self.id
    }

    /// Leaks the interval, releasing memory management of the closure to the JavaScrpt GC.
    #[inline]
    pub fn leak (this: Self) {
        core::mem::forget(this)
    }
}

impl<T> Stream for Interval<T> {
    type Item = T;

    #[inline]
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        self.recv.poll_next_unpin(cx)
    }
}

impl<T> Debug for Interval<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interval")
            .field("id", &self.id)
            .field("closure", &self.closure)
            .finish()
    }
}

impl<T> Drop for Interval<T> {
    #[inline]
    fn drop (&mut self) {
        window().unwrap().clear_interval_with_handle(self.id);
    }
}

/// Spawns an interval directly into JavaScript memory management
#[inline]
pub fn spawn_interval<F: 'static + FnMut()> (timeout: Duration, f: F) -> Result<()> {
    let _ = Interval::leak(Interval::new(timeout, f)?);
    return Ok(())
}

#[inline]
pub fn spawn_timeout<T: 'static, F: 'static + FnOnce() -> T> (timeout: Duration, f: F) -> Result<ShotReceiver<T>> {
    let millis = timeout.as_millis();
    if unlikely(millis > MAX_MILLIS) {
        return Err(JsValue::from_str("timeout overflow"))
    }

    let (send, recv) = one_shot();
    let f = move || {
        let _ = send.try_send(f());
    };

    let closure = Closure::once_into_js(f);
    debug_assert!(closure.is_instance_of::<Function>());

    let _ = window()?.set_timeout_with_callback_and_timeout_and_arguments_0(closure.unchecked_ref(), millis as i32)?;
    return Ok(recv)
}

#[inline]
pub async fn sleep (dur: Duration) -> Result<()> {
    return wasm_bindgen_futures::JsFuture::from(sleep_promise(dur)).await.map(|_| ())
}

pub fn sleep_promise (dur: Duration) -> Promise {
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

        match web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis as i32) {
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

    return Promise::new(&mut f);
}