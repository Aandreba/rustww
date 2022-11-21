use std::{cell::UnsafeCell, mem::MaybeUninit};
use utils_atomics::{flag::{AsyncFlag, AsyncSubscribe}, TakeCell};

const UNINIT: u8 = 0;
const WORKING: u8 = 1;
const INIT: u8 = 2;

pub struct OnceCell<T> {
    sub: AsyncSubscribe,
    flag: TakeCell<AsyncFlag>,
    inner: UnsafeCell<MaybeUninit<T>>
}

impl<T> OnceCell<T> {
    pub fn new () -> Self {
        let flag = AsyncFlag::new();
        let sub = flag.subscribe();

        Self {
            sub,
            flag: TakeCell::new(flag),
            inner: UnsafeCell::new(MaybeUninit::uninit())
        }
    }

    #[inline]
    pub fn try_set (&self, v: T) -> Result<(), T> {
        if let Some(flag) = self.flag.try_take() {
            unsafe {
                (&mut *self.inner.get()).write(v);
                flag.mark();
                return Ok(())
            }
        }

        return Err(v)
    }

    #[inline]
    pub async fn get (&self) -> &T {
        self.sub.clone().await;
        return unsafe { (&*self.inner.get()).assume_init_ref() }
    }
}

unsafe impl<T: Send> Send for OnceCell<T> {}
unsafe impl<T: Sync> Sync for OnceCell<T> {}