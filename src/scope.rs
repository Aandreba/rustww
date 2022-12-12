use js_sys::Function;
use wasm_bindgen::JsCast;
use web_sys::{WorkerGlobalScope, Window, EventTarget};
use crate::Result;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Returns the current global scope
    pub static GLOBAL_SCOPE: Scope = {
        let global = js_sys::global();
        assert!(global.is_instance_of::<Window>() || global.is_instance_of::<WorkerGlobalScope>());
        global.unchecked_into()
    };
}

#[inline]
pub fn set_interval(fun: &Function, millis: i32) -> Result<i32> {
    return GLOBAL_SCOPE.with(|scope| scope.set_interval(fun, millis))
}

#[inline]
pub fn clear_interval(handle: i32) {
    return GLOBAL_SCOPE.with(|scope| scope.clear_interval(handle))
}

#[inline]
pub fn set_timeout(fun: &Function, millis: i32) -> Result<i32> {
    return GLOBAL_SCOPE.with(|scope| scope.set_timeout(fun, millis))
}

#[inline]
pub fn clear_timeout(handle: i32) {
    return GLOBAL_SCOPE.with(|scope| scope.clear_timeout(handle))
}

#[inline]
pub fn fetch (req: &web_sys::Request) -> js_sys::Promise {
    return GLOBAL_SCOPE.with(|scope| scope.fetch(req))
}

#[inline]
pub fn add_global_listener (ty: &str, f: &Function) -> Result<()> {
    return GLOBAL_SCOPE.with(|scope| scope.add_event_listener_with_callback(ty, f))
}

#[inline]
pub fn remove_global_listener (ty: &str, f: &Function) -> Result<()> {
    return GLOBAL_SCOPE.with(|scope| scope.remove_event_listener_with_callback(ty, f))
}

#[wasm_bindgen]
extern "C" {
    /// Represents a JavaScript global scope
    #[derive(Debug, Clone)]
    #[wasm_bindgen(extends = EventTarget)]
    pub type Scope;

    #[wasm_bindgen(js_name = setInterval, structural, method, catch)]
    pub fn set_interval(this: &Scope, fun: &Function, millis: i32) -> Result<i32>;
    #[wasm_bindgen(js_name = clearInterval, structural, method)]
    pub fn clear_interval(this: &Scope, handle: i32);

    #[wasm_bindgen(js_name = setTimeout, structural, method, catch)]
    pub fn set_timeout(this: &Scope, fun: &Function, millis: i32) -> Result<i32>;
    #[wasm_bindgen(js_name = clearTimeout, structural, method)]
    pub fn clear_timeout(this: &Scope, handle: i32);

    #[wasm_bindgen(structural, method)]
    pub fn fetch (this: &Scope, req: &web_sys::Request) -> js_sys::Promise;
}

impl Default for Scope {
    #[inline]
    fn default() -> Self {
        return GLOBAL_SCOPE.with(Clone::clone)
    }
}

#[inline]
pub fn window () -> Result<Window> {
    return js_sys::global().dyn_into().map_err(|_| JsValue::from_str("current global scope isn't a window. you may be in a web worker."));
}