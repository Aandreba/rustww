use std::{task::{Poll, RawWaker, RawWakerVTable, Context, Waker}, sync::{Arc, atomic::AtomicUsize, Mutex, TryLockError, MutexGuard}, ops::DerefMut, pin::Pin};
use futures::{Future, FutureExt};
use wasm_bindgen::{prelude::*, JsCast};
use crate::{sync::{one_shot, drop_local}, runtime::DynFuture};
use super::{Handle, Task};

#[wasm_bindgen]
extern "C" {
    type Scheduler;

    #[wasm_bindgen(method, js_name = postTask)]
    fn post_task (this: &Scheduler, cb: &js_sys::Function) -> js_sys::Promise;
}

impl Scheduler {
    #[inline]
    pub fn spawn_boxed (fut: DynFuture<()>) {        
        todo!()
    }
}

fn create_raw_waker<T: 'static + Send> (handle: Handle, fut: DynFuture<()>) -> RawWaker {
    type Inner = (Handle, DynFuture<()>);
    
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    unsafe fn clone (ptr: *const ()) -> RawWaker {
        Arc::increment_strong_count(ptr as *const Inner);
        return RawWaker::new(ptr, &VTABLE)
    }

    unsafe fn wake (ptr: *const ()) {
        let raw = RawWaker::new(ptr, &VTABLE);
        let waker = Waker::from_raw(raw);
        let mut cx = Context::from_waker(&waker);
        
        let inner = Arc::from_raw(ptr as *const Inner);
        inner.0.inner.queue.push(Box::new(|| {
            let raw = RawWaker::new(ptr, &VTABLE);
            let waker = Waker::from_raw(raw);
            let mut cx = Context::from_waker(&waker);
            inner.1
        }));<
    }

    unsafe fn wake_by_ref (ptr: *const ()) {
        
    }

    unsafe fn drop (ptr: *const ()) {
        Arc::decrement_strong_count(ptr as *const Inner);
    }

    let data = Arc::new((handle, fut));

    todo!()
}