//! Process management syscalls
use crate::{
    mm::{PTEFlags, PageTable, PhysPageNum, VirtAddr},
    task::*,
    timer::get_time_us,
};

const TRACE_REQUEST_READ: usize = 0;
const TRACE_REQUEST_WRITE: usize = 1;
const TRACE_REQUEST_SYSCALL: usize = 2;

/// Translate one user byte and verify the permission required by this access.
fn user_byte_location(token: usize, addr: usize, writable: bool) -> Option<(PhysPageNum, usize)> {
    let va = VirtAddr::from(addr);
    let pte = PageTable::from_token(token).translate(va.floor())?;
    let flags = pte.flags();
    if (flags & PTEFlags::U) == PTEFlags::empty() || (writable && !pte.writable()) {
        return None;
    }
    if !writable && !pte.readable() {
        return None;
    }
    Some((pte.ppn(), va.page_offset()))
}

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let token = current_user_token();
    for (offset, byte) in bytes.iter().enumerate() {
        let Some(addr) = (ts as usize).checked_add(offset) else {
            return -1;
        };
        let Some((ppn, page_offset)) = user_byte_location(token, addr, true) else {
            return -1;
        };
        ppn.get_bytes_array()[page_offset] = *byte;
    }
    0
}

/// Inspect user memory or return the current task's syscall count.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    match trace_request {
        TRACE_REQUEST_READ => {
            let Some((ppn, offset)) = user_byte_location(token, id, false) else {
                return -1;
            };
            ppn.get_bytes_array()[offset] as isize
        }
        TRACE_REQUEST_WRITE => {
            let Some((ppn, offset)) = user_byte_location(token, id, true) else {
                return -1;
            };
            ppn.get_bytes_array()[offset] = data as u8;
            0
        }
        TRACE_REQUEST_SYSCALL => current_syscall_count(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
