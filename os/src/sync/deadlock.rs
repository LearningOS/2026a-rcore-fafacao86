//! Per-process deadlock detection for mutexes and semaphores.

use alloc::vec;
use alloc::vec::Vec;

/// Returned when a resource request would make the process unsafe.
pub(crate) const DEADLOCK_ERROR: isize = -0xdead;

#[derive(Default)]
struct BankerState {
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
    granted: Vec<Vec<bool>>,
}

impl BankerState {
    fn register_resource(&mut self, count: usize) {
        self.available.push(count);
        for allocation in self.allocation.iter_mut() {
            allocation.push(0);
        }
        for need in self.need.iter_mut() {
            need.push(0);
        }
        for granted in self.granted.iter_mut() {
            granted.push(false);
        }
    }

    fn ensure_resource(&mut self, resource: usize) {
        while self.available.len() <= resource {
            self.register_resource(0);
        }
    }

    fn ensure_task(&mut self, task: usize) {
        while self.allocation.len() <= task {
            self.allocation.push(vec![0; self.available.len()]);
            self.need.push(vec![0; self.available.len()]);
            self.granted.push(vec![false; self.available.len()]);
        }
    }

    fn reset_task(&mut self, task: usize) {
        self.ensure_task(task);
        self.allocation[task].fill(0);
        self.need[task].fill(0);
        self.granted[task].fill(false);
    }

    fn request(&mut self, task: usize, resource: usize, check_safe: bool) -> bool {
        self.ensure_resource(resource);
        self.ensure_task(task);
        self.need[task][resource] = 1;

        if check_safe && !self.is_safe() {
            self.need[task][resource] = 0;
            return false;
        }
        true
    }

    fn complete_request(&mut self, task: usize, resource: usize) {
        self.ensure_resource(resource);
        self.ensure_task(task);

        if self.granted[task][resource] {
            self.granted[task][resource] = false;
        } else {
            self.available[resource] = self.available[resource].saturating_sub(1);
            self.allocation[task][resource] += 1;
        }
        self.need[task][resource] = 0;
    }

    fn release_allocation(&mut self, task: usize, resource: usize) -> bool {
        self.ensure_resource(resource);
        self.ensure_task(task);
        if self.allocation[task][resource] == 0 {
            false
        } else {
            self.allocation[task][resource] -= 1;
            true
        }
    }

    fn handoff_or_release(&mut self, resource: usize, next_task: Option<usize>) {
        if let Some(next_task) = next_task {
            self.ensure_task(next_task);
            self.allocation[next_task][resource] += 1;
            self.need[next_task][resource] = 0;
            self.granted[next_task][resource] = true;
        } else {
            self.available[resource] += 1;
        }
    }

    fn is_safe(&self) -> bool {
        let task_count = self.allocation.len();
        let resource_count = self.available.len();
        let mut work = self.available.clone();
        let mut finish = vec![false; task_count];

        loop {
            let mut progressed = false;
            for task in 0..task_count {
                if finish[task]
                    || (0..resource_count)
                        .any(|resource| self.need[task][resource] > work[resource])
                {
                    continue;
                }

                for resource in 0..resource_count {
                    work[resource] += self.allocation[task][resource];
                }
                finish[task] = true;
                progressed = true;
            }
            if !progressed {
                break;
            }
        }

        finish.iter().all(|finished| *finished)
    }
}

/// Deadlock detection state owned by one process.
pub(crate) struct DeadlockDetector {
    enabled: bool,
    mutex: BankerState,
    semaphore: BankerState,
}

impl DeadlockDetector {
    pub(crate) fn new() -> Self {
        Self {
            enabled: false,
            mutex: BankerState::default(),
            semaphore: BankerState::default(),
        }
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(crate) fn register_mutex(&mut self, resource: usize) {
        self.mutex.ensure_resource(resource);
        self.mutex.available[resource] = 1;
    }

    pub(crate) fn register_semaphore(&mut self, resource: usize, count: usize) {
        self.semaphore.ensure_resource(resource);
        self.semaphore.available[resource] = count;
    }

    pub(crate) fn reset_task(&mut self, task: usize) {
        self.mutex.reset_task(task);
        self.semaphore.reset_task(task);
    }

    pub(crate) fn request_mutex(&mut self, task: usize, resource: usize) -> bool {
        self.mutex.request(task, resource, self.enabled)
    }

    pub(crate) fn request_semaphore(&mut self, task: usize, resource: usize) -> bool {
        self.semaphore.request(task, resource, self.enabled)
    }

    pub(crate) fn complete_mutex(&mut self, task: usize, resource: usize) {
        self.mutex.complete_request(task, resource);
    }

    pub(crate) fn complete_semaphore(&mut self, task: usize, resource: usize) {
        self.semaphore.complete_request(task, resource);
    }

    pub(crate) fn release_mutex(&mut self, task: usize, resource: usize, next_task: Option<usize>) {
        if self.mutex.release_allocation(task, resource) {
            self.mutex.handoff_or_release(resource, next_task);
        }
    }

    pub(crate) fn release_semaphore(
        &mut self,
        task: usize,
        resource: usize,
        next_task: Option<usize>,
    ) {
        self.semaphore.release_allocation(task, resource);
        self.semaphore.handoff_or_release(resource, next_task);
    }
}

fn current_tid() -> usize {
    crate::task::current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid
}

fn task_tid(task: &alloc::sync::Arc<crate::task::TaskControlBlock>) -> usize {
    task.inner_exclusive_access().res.as_ref().unwrap().tid
}

pub(crate) fn mutex_acquired(resource: usize) {
    let process = crate::task::current_process();
    process
        .inner_exclusive_access()
        .deadlock_detector
        .complete_mutex(current_tid(), resource);
}

pub(crate) fn mutex_released(
    resource: usize,
    next_task: Option<&alloc::sync::Arc<crate::task::TaskControlBlock>>,
) {
    let process = crate::task::current_process();
    process
        .inner_exclusive_access()
        .deadlock_detector
        .release_mutex(current_tid(), resource, next_task.map(task_tid));
}

pub(crate) fn semaphore_acquired(resource: usize) {
    let process = crate::task::current_process();
    process
        .inner_exclusive_access()
        .deadlock_detector
        .complete_semaphore(current_tid(), resource);
}

pub(crate) fn semaphore_released(
    resource: usize,
    next_task: Option<&alloc::sync::Arc<crate::task::TaskControlBlock>>,
) {
    let process = crate::task::current_process();
    process
        .inner_exclusive_access()
        .deadlock_detector
        .release_semaphore(current_tid(), resource, next_task.map(task_tid));
}
