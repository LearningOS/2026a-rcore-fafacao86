//! Stride scheduling policy for the ready queue.

use super::TaskControlBlock;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::{Ordering, Reverse};

const DEFAULT_PRIORITY: usize = 16;
const BIG_STRIDE: u64 = 1 << 16;

pub struct StrideState {
    priority: usize,
    stride: u64,
}

impl StrideState {
    pub(super) fn new() -> Self {
        Self { priority: DEFAULT_PRIORITY, stride: 0 }
    }

    pub fn set_priority(&mut self, priority: usize) {
        self.priority = priority;
    }

    fn stride(&self) -> u64 {
        self.stride
    }

    fn advance(&mut self) {
        self.stride += BIG_STRIDE / self.priority as u64;
    }
}

struct ReadyTask {
    stride: u64,
    task: Arc<TaskControlBlock>,
}

impl PartialEq for ReadyTask {
    fn eq(&self, other: &Self) -> bool { self.stride == other.stride }
}
impl Eq for ReadyTask {}
impl PartialOrd for ReadyTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for ReadyTask {
    fn cmp(&self, other: &Self) -> Ordering { self.stride.cmp(&other.stride) }
}

pub(super) struct StrideScheduler {
    ready_heap: BinaryHeap<Reverse<ReadyTask>>,
}

impl StrideScheduler {
    pub(super) fn new() -> Self {
        Self { ready_heap: BinaryHeap::new() }
    }

    pub(super) fn add(&mut self, task: Arc<TaskControlBlock>) {
        let stride = task.inner_exclusive_access().scheduling.stride();
        self.ready_heap.push(Reverse(ReadyTask { stride, task }));
    }

    pub(super) fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_heap.pop()?.0.task;
        task.inner_exclusive_access().scheduling.advance();
        Some(task)
    }

    pub(super) fn remove(&mut self, task: &Arc<TaskControlBlock>) {
        let mut retained = BinaryHeap::new();
        while let Some(entry) = self.ready_heap.pop() {
            if !Arc::ptr_eq(&entry.0.task, task) {
                retained.push(entry);
            }
        }
        self.ready_heap = retained;
    }
}
