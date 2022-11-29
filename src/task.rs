use futures::Future;
pub use wasm_bindgen_futures::spawn_local;
use crate::Result;

/// Spawns the specified `Future` into the JavaScript runtime, and throwing an exception if it fails.
/// 
/// See [`spawn_local`](wasm_bindgen_futures::spawn_local)
#[inline]
pub fn spawn_catch_local<Fut: 'static + Future<Output = Result<()>>> (fut: Fut) {
    spawn_local(async move {
        if let Err(e) = fut.await {
            wasm_bindgen::throw_val(e);
        }
    })
}