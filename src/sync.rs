use std::{cell::{Cell, UnsafeCell}, task::{Poll, Waker}, collections::VecDeque, ops::{Deref, DerefMut}, sync::{Arc, atomic::{AtomicU8, Ordering}}, cmp::Ordering, hint::unreachable_unchecked};
use futures::{Future, task::AtomicWaker};
use utils_atomics::flag::spsc::{AsyncFlag, async_flag};
use wasm_bindgen_futures::spawn_local;

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

/*cfg_if::cfg_if! {
    if #[cfg(target_feature = "atomics")] {

    } else {
        /// Receiver to a local one-shot channel
        pub struct ShotReceiver<T> {
            pub(crate) inner: Rc<FutureInner<T>>
        }

        impl<T> Future for ShotReceiver<T> {
            type Output = T;

            #[inline]
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                if let Some(geo) = self.inner.value.take() {
                    return Poll::Ready(geo.expect("Value has already been taken"));
                }

                self.inner.waker.set(Some(cx.waker().clone()));
                return Poll::Pending
            }
        }

        /// Sender of a local one-shot channel
        pub struct ShotSender<T> {
            inner: Weak<FutureInner<T>>
        }

        impl<T> ShotSender<T> {
            /// Attempts to send the value through the channel, returning it if it fails.
            /// 
            /// A send through the channel fails of a value has already been sent through it.
            #[inline]
            pub fn try_send (&self, v: T) -> ::core::result::Result<(), T> {
                if let Some(inner) = self.inner.upgrade() {    
                    inner.value.set(Some(Some(v)));
                    if let Some(waker) = inner.waker.take() {
                        waker.wake();
                    }
                    return Ok(())
                }
                return Err(v)
            }
        }

        impl<T> Clone for ShotSender<T> {
            #[inline]
            fn clone(&self) -> Self {
                Self { inner: self.inner.clone() }
            }
        }

        /// Creates a new local one-shot channel, which is optimized to be able to send a single value.
        #[inline]
        pub fn one_shot<T> () -> (ShotSender<T>, ShotReceiver<T>) {
            let inner = Rc::new(FutureInner::default());
            return (ShotSender { inner: Rc::downgrade(&inner) }, ShotReceiver { inner })
        }

        pub(crate) struct FutureInner<T> {
            pub(crate) value: Cell<Option<Option<T>>>,
            pub(crate) waker: Cell<Option<Waker>>
        }

        impl<T> Default for FutureInner<T> {
            #[inline]
            fn default() -> Self {
                Self {
                    value: Default::default(),
                    waker: Default::default()
                }
            }
        }
    }
}*/

const UNINIT: u8 = 0;
const WORKING: u8 = 1;
const INIT: u8 = 2;
const TAKEN: u8 = 3;

/// Receiver to a local one-shot channel
pub struct ShotReceiver<T> {
    pub(crate) inner: Arc<FutureInner<T>>
}

impl<T> Future for ShotReceiver<T> {
    type Output = Option<T>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.inner.waker.register(cx.waker());

        loop {
            match self.inner.state.compare_exchange(INIT, TAKEN, Ordering::AcqRel, Ordering::Acquire) {
                Err(UNINIT) if Arc::weak_count(&self.inner) == 0 => return Poll::Ready(None),
                Err(UNINIT) => return Poll::Pending,
                Err(WORKING) => core::hint::spin_loop(),
                Ok(_) => todo!(),
                Err(TAKEN) => panic!("value already taken"),
                #[cfg(debug_assertions)]
                _ => unreachable!(),
                #[cfg(not(debug_assertions))]
                _ => unsafe { unreachable_unchecked() }
            }
        }

        todo!()
    }
}

/// Sender of a local one-shot channel
pub struct ShotSender<T> {
    inner: std::sync::Weak<FutureInner<T>>
}

impl<T> ShotSender<T> {
    /// Attempts to send the value through the channel, returning it if it fails.
    /// 
    /// A send through the channel fails of a value has already been sent through it.
    #[inline]
    pub fn try_send (&self, v: T) -> ::core::result::Result<(), T> {
        if let Some(inner) = self.inner.upgrade() {    
            inner.value.set(Some(Some(v)));
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
            return Ok(())
        }
        return Err(v)
    }
}

impl<T> Clone for ShotSender<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// Creates a new local one-shot channel, which is optimized to be able to send a single value.
#[inline]
pub fn one_shot<T> () -> (ShotSender<T>, ShotReceiver<T>) {
    let inner = Arc::new(FutureInner::default());
    return (ShotSender { inner: Arc::downgrade(&inner) }, ShotReceiver { inner })
}

struct FutureInner<T> {
    state: AtomicU8,
    value: UnsafeCell<Option<T>>,
    waker: AtomicWaker
}

impl<T> Default for FutureInner<T> {
    #[inline]
    fn default() -> Self {
        Self {
            state: AtomicU8::new(UNINIT),
            value: Default::default(),
            waker: Default::default()
        }
    }
}

/// Handle to drop a value owned by another thread.
/// 
/// When dropped, the associated value will be dropped from it's original thread 
/// 
/// Usefull to make structs with JavaScript closures thread-safe.
#[derive(Debug)]
pub struct DropHandle {
    _inner: AsyncFlag
}

/// Makes sure the value `t` is dropped in the current thread, returning a [`DropHandle`]
/// that when dropped, it will signal to the current thread to drop `t`.
/// 
/// This is mainly usefull to make structs with JavaScript closures thread-safe.
#[inline]
pub fn drop_local<T: 'static> (t: T) -> DropHandle {
    let (_inner, sub) = async_flag();

    spawn_local(async move {
        sub.await;
        drop(t);
    });
    
    return DropHandle { _inner }
}

/// Handle to drop a value owned by another thread.
/// 
/// When all the handles of a value are dropped, the associated value will be dropped from it's original thread 
/// 
/// Usefull to make structs with JavaScript closures thread-safe.
#[derive(Debug, Clone)]
pub struct SharedDropHandle {
    _inner: utils_atomics::flag::mpmc::AsyncFlag
}

/// Makes sure the value `t` is dropped in the current thread, returning a [`SharedDropHandle`].
/// 
/// This is mainly usefull to make structs with JavaScript closures thread-safe.
#[inline]
pub fn drop_shared_local<T: 'static> (t: T) -> SharedDropHandle {
    let (_inner, sub) = utils_atomics::flag::mpmc::async_flag();

    spawn_local(async move {
        sub.await;
        drop(t);
    });
    
    return SharedDropHandle { _inner }
}

/// Makes sure the value `t` is dropped in the current thread, after `fut` has completed.
/// 
/// This is mainly usefull to make structs with JavaScript closures thread-safe.
#[inline]
pub fn drop_local_with<T: 'static> (t: T, fut: impl 'static + Future<Output = ()>) {
    spawn_local(async move {
        fut.await;
        drop(t);
    });
}