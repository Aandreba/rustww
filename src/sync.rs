use std::{cell::{Cell, UnsafeCell}, task::{Poll, Waker}, collections::VecDeque, ops::{Deref, DerefMut}};
use futures::{Future};

cfg_if::cfg_if! {
    if #[cfg(target_feature = "atomics")] {
        use std::sync::atomic::{AtomicBool, Ordering};
        use crossbeam::queue::SegQueue;

        pub struct Mutex<T> {
            locked: AtomicBool,
            inner: UnsafeCell<T>,
            wakers: SegQueue<Waker>
        }

        impl<T> Mutex<T> {
            #[inline]
            pub fn new (t: T) -> Self {
                return Self {
                    locked: AtomicBool::new(false),
                    inner: UnsafeCell::new(t),
                    wakers: SegQueue::new()
                }
            }
        
            #[inline]
            pub fn is_locked (&self) -> bool {
                self.locked.load(Ordering::Acquire)
            }
        
            #[inline]
            pub fn try_lock (&self) -> Option<MutexGuard<'_, T>> {
                if self.locked.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
                    return Some(MutexGuard { parent: self })
                }
                return None
            }
        
            #[inline]
            pub fn lock (&self) -> MutexLock<'_, T> {
                return MutexLock { parent: self }
            }
        }

        pub struct MutexLock<'a, T> {
            parent: &'a Mutex<T>
        }
        
        impl<'a, T> Future for MutexLock<'a, T> {
            type Output = MutexGuard<'a, T>;
        
            #[inline]
            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
                if let Some(guard) = self.parent.try_lock() {
                    return Poll::Ready(guard)
                }
                self.parent.wakers.push(cx.waker().clone());
                return Poll::Pending
            }
        }

        impl<T> Drop for MutexGuard<'_, T> {
            #[inline]
            fn drop(&mut self) {
                self.parent.locked.store(false, Ordering::Release);
                if let Some(waker) = self.wakers().pop() {
                    waker.wake()
                }
            }
        }

        #[cfg_attr(docsrs, doc(cfg(target_feature = "atomics")))]
        unsafe impl<T: Send> Send for Mutex<T> {}
        #[cfg_attr(docsrs, doc(cfg(target_feature = "atomics")))]
        unsafe impl<T: Sync> Sync for Mutex<T> {}
    } else {
        pub struct Mutex<T> {
            locked: Cell<bool>,
            inner: UnsafeCell<T>,
            wakers: UnsafeCell<VecDeque<Waker>>
        }
        
        impl<T> Mutex<T> {
            #[inline]
            pub fn new (t: T) -> Self {
                return Self {
                    locked: Cell::new(false),
                    inner: UnsafeCell::new(t),
                    wakers: UnsafeCell::new(VecDeque::new())
                }
            }
        
            #[inline]
            pub fn is_locked (&self) -> bool {
                self.locked.get()
            }
        
            #[inline]
            pub fn try_lock (&self) -> Option<MutexGuard<'_, T>> {
                if self.is_locked() { return None }
                self.locked.set(true);
                return Some(MutexGuard { parent: self })
            }
        
            #[inline]
            pub fn lock (&self) -> MutexLock<'_, T> {
                return MutexLock { parent: self }
            }
        }
        
        pub struct MutexLock<'a, T> {
            parent: &'a Mutex<T>
        }
        
        impl<'a, T> Future for MutexLock<'a, T> {
            type Output = MutexGuard<'a, T>;
        
            #[inline]
            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
                if let Some(guard) = self.parent.try_lock() {
                    return Poll::Ready(guard)
                }
                unsafe { &mut *self.parent.wakers.get() }.push_back(cx.waker().clone());
                return Poll::Pending
            }
        }
        
        impl<T> Drop for MutexGuard<'_, T> {
            #[inline]
            fn drop(&mut self) {
                self.parent.locked.set(false);
                if let Some(waker) = unsafe { &mut *self.parent.wakers.get() }.pop_front() {
                    waker.wake()
                }
            }
        }
    }
}

pub struct MutexGuard<'a, T> {
    parent: &'a Mutex<T>
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.parent.inner.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.parent.inner.get() }
    }
}