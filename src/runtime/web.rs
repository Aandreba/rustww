use std::{task::{Poll, RawWaker, RawWakerVTable, Waker, Context}, sync::Arc, marker::PhantomPinned, pin::Pin, cell::Cell};
use futures::{Future, FutureExt};
use pin_project_lite::pin_project;
use utils_atomics::TakeCell;
use wasm_bindgen::{prelude::*, JsCast};
use crate::sync::{one_shot, ShotReceiver, ShotSender, DropHandle, drop_local};

#[wasm_bindgen]
extern "C" {
    type Scheduler;

    #[wasm_bindgen(method, js_name = postTask)]
    fn post_task (this: &Scheduler, cb: &js_sys::Function) -> js_sys::Promise;
}

impl Scheduler {
    #[inline]
    pub fn schedule_closure<T: 'static, F: 'static + FnOnce() -> T> (&self, f: F) -> ClosureHandle<T> {
        let (send, recv) = one_shot();
        let closure = Closure::once(move || send.send(f()));

        let fun = closure.as_ref();
        debug_assert!(fun.is_instance_of::<js_sys::Function>());
        let _ = self.post_task(fun.unchecked_ref());
        
        return ClosureHandle {
            cb: drop_local(closure),
            recv
        }
    }
}

/// Handle to wait for a scheduled closure
struct ClosureHandle<T> {
    cb: DropHandle,
    recv: ShotReceiver<T>
}

impl Future for ClosureHandle<T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return match self.recv.poll_unpin(cx) {
            Poll::Ready(Some(x)) => Poll::Ready(x),
            Poll::Ready(None) => unreachable!(),
            Poll::Pending => Poll::Pending
        }
    }
}