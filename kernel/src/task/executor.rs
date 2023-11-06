use alloc::task::Wake;
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::{Context, Poll, Waker};

use crossbeam_queue::ArrayQueue;

use super::{Task, TaskId};

/// The task executor.
///
/// This is a simple FIFO executor that runs tasks on a single thread.
///
/// # Fields
///
/// * `tasks`: The tasks to be executed.
/// * `task_queue`: The queue of task IDs.
/// * `waker_cache`: The cache of task wakers.
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    /// Creates a new `Executor`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    /// Spawns a task.
    ///
    /// # Arguments
    ///
    /// * `task`: The task to spawn.
    ///
    /// # Panics
    ///
    /// * If a task with the same ID already exists.
    /// * If the queue is full.
    #[allow(clippy::expect_used)]
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        assert!(
            self.tasks.insert(task.id, task).is_none(),
            "Task with same ID already in tasks!"
        );

        self.task_queue.push(task_id).expect("Task queue full!");
    }

    /// Runs all ready tasks.
    ///
    /// This function runs all tasks that are ready to be run.
    fn run_ready_tasks(&mut self) {
        // Destructure `self` to avoid borrow checker errors.
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Ok(task_id) = task_queue.pop() {
            let Some(task) = tasks.get_mut(&task_id) else {
                continue;
            };

            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));

            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // Task done -> remove it and its cached waker.
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    /// Runs the executor.
    ///
    /// This function runs the executor.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    /// Sleeps if the executor is idle.
    ///
    /// This function sleeps if the executor is idle.
    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// The task waker.
///
/// This is a simple task waker that wakes tasks on a single thread.
///
/// # Fields
///
/// * `task_id`: The ID of the task to wake.
/// * `task_queue`: The queue of task IDs.
struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    /// Creates a new `TaskWaker`.
    ///
    /// # Arguments
    ///
    /// * `task_id`: The ID of the task to wake.
    /// * `task_queue`: The queue of task IDs.
    #[allow(clippy::new_ret_no_self)]
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(Self {
            task_id,
            task_queue,
        }))
    }

    /// Wakes the task.
    ///
    /// This function wakes the task.
    ///
    /// # Panics
    ///
    /// * If the queue is full.
    #[allow(clippy::expect_used)]
    fn wake_task(&self) {
        self.task_queue
            .push(self.task_id)
            .expect("task_queue full!");
    }
}

impl Wake for TaskWaker {
    /// Wakes the task.
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    /// Wakes the task by reference.
    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
