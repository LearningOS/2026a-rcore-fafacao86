//! Stride scheduling policy for the ready queue.

use super::manager::Scheduler;
use super::TaskControlBlock;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::{Ordering, Reverse};

const DEFAULT_PRIORITY: usize = 16;
const BIG_STRIDE: u64 = 1 << 16;

/// Per-process state used by the stride scheduler.
pub(super) struct StrideState {
    priority: usize,
    stride: u64,
}

impl StrideState {
    pub(super) fn new() -> Self {
        Self {
            priority: DEFAULT_PRIORITY,
            stride: 0,
        }
    }

    pub(super) fn set_priority(&mut self, priority: usize) {
        self.priority = priority;
    }

    fn stride(&self) -> u64 {
        self.stride
    }

    fn advance(&mut self) {
        self.stride += BIG_STRIDE / self.priority as u64;
    }
}

/// A ready task and its stride at the time it enters the queue.
struct ReadyTask {
    stride: u64,
    task: Arc<TaskControlBlock>,
}

impl PartialEq for ReadyTask {
    fn eq(&self, other: &Self) -> bool {
        self.stride == other.stride
    }
}

impl Eq for ReadyTask {}

impl PartialOrd for ReadyTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReadyTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.stride.cmp(&other.stride)
    }
}

/// A min-heap of ready tasks ordered by stride.
pub(super) struct StrideScheduler {
    ready_heap: BinaryHeap<Reverse<ReadyTask>>,
}

impl StrideScheduler {
    pub(super) fn new() -> Self {
        Self {
            ready_heap: BinaryHeap::new(),
        }
    }
}

impl Scheduler for StrideScheduler {
    fn add(&mut self, task: Arc<TaskControlBlock>) {
        let stride = task.inner_exclusive_access().scheduling.stride();
        self.ready_heap.push(Reverse(ReadyTask { stride, task }));
    }

    /// Select the task with the least stride and charge one time slice.
    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_heap.pop()?.0.task;
        task.inner_exclusive_access().scheduling.advance();
        Some(task)
    }
}
