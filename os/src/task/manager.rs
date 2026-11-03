//!Implementation of [`TaskManager`]
use super::{stride, TaskControlBlock};
use crate::sync::UPSafeCell;
use alloc::boxed::Box;
use alloc::sync::Arc;
use lazy_static::*;

/// Interface used by the task manager to enqueue and select processes.
pub(super) trait Scheduler: Send {
    fn add(&mut self, task: Arc<TaskControlBlock>);
    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>>;
}

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    scheduler: Box<dyn Scheduler>,
}

impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            scheduler: Box::new(stride::StrideScheduler::new()),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.scheduler.add(task);
    }
    /// Take the next ready process selected by the scheduler
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.scheduler.fetch()
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
