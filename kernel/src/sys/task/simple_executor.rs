use alloc::collections::VecDeque;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use super::Task;

/// The task executor.
///
/// This is a simple FIFO executor that runs tasks on a single thread.
///
/// # Fields
///
/// * `task_queue`: The queue of tasks.
pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    /// Creates a new `SimpleExecutor`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
        }
    }

    /// Spawns a task.
    ///
    /// # Arguments
    ///
    /// * `task`: The task to spawn.
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task);
    }

    /// Runs all tasks to completion.
    ///
    /// This function runs all tasks in the executor until they are complete.
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {} // Task done.
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}

/// A dummy waker.
///
/// This waker does nothing when woken.
///
/// # Returns
///
/// * `RawWaker` - The dummy waker.
fn dummy_raw_waker() -> RawWaker {
    /// A no-op function.
    const fn no_op(_: *const ()) {}

    /// A clone function.
    ///
    /// # Returns
    ///
    /// * `RawWaker` - The dummy waker.
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);

    RawWaker::new(core::ptr::null::<()>(), vtable)
}

/// A dummy waker.
///
/// This waker does nothing when woken.
///
/// # Returns
///
/// * `Waker` - The dummy waker.
fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
