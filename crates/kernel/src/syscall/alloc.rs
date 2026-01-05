use clibc::{log, logf};

use crate::Task;
use crate::global::{CURRENT_TASK, KERNEL_TASK_SLOT, TASKS};

pub(crate) fn alloc_in_task(task: &mut Task, size: u32, align: u32) -> Option<u32> {
    if size == 0 {
        log!("sys_alloc: invalid size 0");
        return None;
    }
    if align == 0 || (align & (align - 1)) != 0 {
        logf!("sys_alloc: invalid alignment %d", align);
        return None;
    }

    let mask = align - 1;
    let start = match task.heap_ptr.checked_add(mask) {
        Some(addr) => addr & !mask,
        None => {
            log!("sys_alloc: heap ptr overflow");
            return None;
        }
    };
    let end = match start.checked_add(size) {
        Some(end) => end,
        None => {
            log!("sys_alloc: size overflow");
            return None;
        }
    };

    let window_base = task.addr_space.va_base;
    let window_limit = window_base.saturating_add(task.addr_space.va_len);
    if start < window_base || end > window_limit {
        logf!(
            "sys_alloc: heap range exceeds task window start=0x%x end=0x%x window=[0x%x,0x%x)",
            start,
            end,
            window_base,
            window_limit
        );
        return None;
    }
    task.heap_ptr = end;
    Some(start)
}

pub(crate) fn sys_alloc(args: [u32; 6]) -> u32 {
    let size = args[0];
    let align = args[1];

    let current = unsafe { *CURRENT_TASK.get_mut() };
    // Kernel task should never call sys_alloc.
    if current == KERNEL_TASK_SLOT {
        panic!("sys_alloc: kernel task cannot allocate memory");
    }

    let tasks = unsafe { TASKS.get_mut() };
    let task = match tasks.get_mut(current) {
        Some(task) => task,
        None => {
            logf!("sys_alloc: no current task for slot %d", current as u32);
            return 0;
        }
    };

    match alloc_in_task(task, size, align) {
        Some(addr) => addr,
        None => 0,
    }
}

pub(crate) fn sys_dealloc(_args: [u32; 6]) -> u32 {
    let current = unsafe { *CURRENT_TASK.get_mut() };
    // Kernel task should never call sys_alloc.
    if current == KERNEL_TASK_SLOT {
        panic!("sys_alloc: kernel task cannot allocate memory");
    }
    // No-op: kernel heap is bump-only for now.
    0
}
