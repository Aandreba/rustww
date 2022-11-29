use futures::Future;
pub use wasm_bindgen_futures::spawn_local;
use crate::Result;

#[inline]
pub fn spawn_catch_local<Fut: 'static + Future<Output = Result<()>>> (fut: Fut) {
    spawn_local(async move {
        if let Err(e) = fut.await {
            wasm_bindgen::throw_val(e);
        }
    })
}