//! Types related to task management

use alloc::collections::BTreeMap;

use super::TaskContext;

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// Number of syscalls made by this task, keyed by syscall ID.
    ///
    /// Linux similarly stores task-specific state in `task_struct`.
    /// https://github.com/torvalds/linux/blob/master/include/linux/sched.h#task_struct
    /// We use a simple sparse map here for simplicity
    pub syscall_stats: BTreeMap<usize, usize>,
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
