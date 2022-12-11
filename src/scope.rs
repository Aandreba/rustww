use elor::Either;
use wasm_bindgen::JsCast;
use web_sys::{WorkerGlobalScope, Window};
use crate::Result;

thread_local! {
    static LOCAL_SCOPE: Either<Window, WorkerGlobalScope> = {
        match js_sys::global().dyn_into::<Window>() {
            Ok(window) => Either::Left(window),
            Err(global) => match global.dyn_into::<WorkerGlobalScope>() {
                Ok(worker) => Either::Right(worker),
                Err(e) => wasm_bindgen::throw_val(e.into())
            }
        }
    }
}

macro_rules! impl_global {
    ($(
        $( #[$meta:meta] )*
        fn $name:ident ( $( $arg_name:ident : $arg_ty:ty ),* $(,)? ) $(-> $($ret:tt)+)?
    );+) => {
        $(
            pub fn $name ($($arg_name: $arg_ty),+) -> Result<$()?> {
                return LOCAL_SCOPE.with(|scope| todo!())
            }    
        )+
    };

    (@ret) => { Result<()> };
    (@ret Result<$ret:ty>) => { Result<$ret> };
    (@ret $ret:ty) => { Result<$ret> }
}

/// Returns the current local scope
pub fn scope () -> Either<Window, WorkerGlobalScope> {
    return LOCAL_SCOPE.with(Clone::clone)
}

/// Getter for the `Window` object
#[inline]
pub fn window () -> Result<Window> {
    return js_sys::global().dyn_into::<Window>().map_err(Into::into);
}

/// Getter for the `WorkerGlobalScope` object
#[inline]
pub fn worker_scope () -> Result<WorkerGlobalScope> {
    return js_sys::global().dyn_into::<WorkerGlobalScope>().map_err(Into::into);
}