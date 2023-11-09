use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll};
use core::{future::Future, pin::Pin};

pub mod executor;
pub mod keyboard;
pub mod primes;
pub mod simple_executor;

/// A task.
///
/// # Fields
///
/// * `id`: The task ID.
/// * `future`: The future to be executed.
pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Creates a new `Task`.
    ///
    /// # Arguments
    ///
    /// * `future`: The future to be executed.
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    /// Polls the task.
    ///
    /// # Arguments
    ///
    /// * `context`: The context to use for polling.
    ///
    /// # Returns
    ///
    /// * `Poll<()>` - The result of polling the task.
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

/// A task ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TaskId(u64);

impl TaskId {
    /// Creates a new `TaskId`.
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
