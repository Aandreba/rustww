use std::{sync::Arc, pin::Pin, task::{Context, Poll}};
use crossbeam::queue::SegQueue;
use futures::Future;
mod web;

pub(super) type DynFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;

pub(super) struct Inner {
    queue: SegQueue<Box<dyn FnOnce()>>
}

/// Handle to work with a runtime instance
#[derive(Clone)]
pub struct Handle {
    pub(super) inner: Arc<Inner>
}

impl Handle {
    #[inline]
    pub fn schedule<T> (&self, task: Task) {
        self.inner.queue.push(task)
    }
}