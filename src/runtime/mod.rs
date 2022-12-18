use std::{sync::Arc, task::Waker};
use crossbeam::queue::SegQueue;
use futures::Future;
use utils_atomics::FillQueue;
use crate::sync::one_shot;
use self::task::Task;

mod web;

struct Inner {
    queue: SegQueue<Task>
}

/// Handle to work with a runtime instance
#[derive(Clone)]
pub struct Handle {
    inner: Arc<Inner>
}

impl Handle {
    #[inline]
    pub fn schedule<T> (&self, task: Task) {
        self.inner.queue.push(task)
    }
}

pub struct Scheduler {
    handle: Handle,
}

impl Scheduler {
    #[inline]
    pub fn run (self) {
        loop {
            
        }
    }
}