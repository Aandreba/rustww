use futures::{Future};
pub use wasm_bindgen_futures::spawn_local;
use js_sys::*;
use crate::{utils::{AbortSignal, AbortController}, Result};

/// Spawns the specified `Future` into the JavaScript runtime, and throwing an exception if it fails.
/// 
/// See [`spawn_local`](wasm_bindgen_futures::spawn_local)
#[inline]
pub fn spawn_catch_local<Fut: 'static + Future<Output = crate::Result<()>>> (fut: Fut) {
    spawn_local(async move {
        if let Err(e) = fut.await {
            wasm_bindgen::throw_val(e);
        }
    })
}

/// Returns a future that resolves when `p` settles, or when `con` is aborted
pub fn abortable_promise_with_controller<T> (p: js_sys::Promise, con: &AbortController<T>) -> js_sys::Promise {
    let array = js_sys::Array::new_with_length(2);
    array.set(0, p.into());
    array.set(1, con.signal_promise().into());
    return Promise::race(&array)
}

/// Returns a future that resolves when `p` settles, or when `signal` is aborted
pub fn abortable_promise_with_signal<T> (p: js_sys::Promise, signal: &AbortSignal<T>) -> js_sys::Promise {
    let array = js_sys::Array::new_with_length(2);
    array.set(0, p.into());
    array.set(1, signal.promise().into());
    return Promise::race(&array)
}

/// Returns a future that resolves when `p` settles, or when the resulting controller is aborted
#[inline]
pub fn abortable_promise<T> (p: js_sys::Promise) -> Result<(js_sys::Promise, AbortController<T>)> {
    let con = AbortController::<T>::new()?;
    let p = abortable_promise_with_controller(p, &con);
    return Ok((p, con))
}