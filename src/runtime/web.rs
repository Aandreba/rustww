use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    type Scheduler;

    #[wasm_bindgen(method, js_name = postTask)]
    fn post_task (this: &Scheduler, cb: &js_sys::Function) -> js_sys::Promise;
}

impl Scheduler {
    #[inline]
    pub fn schedule<F: 'static + FnOnce() -> T> (&self, f: F) {

    }
}

pub struct SchedulerJoinHandle<T> {
    promise: js_sys::Promise
}