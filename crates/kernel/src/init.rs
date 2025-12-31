use core::{cmp, slice};

use clibc::{log, logf};
use state::State;

use kernel::global::{CURRENT_TASK, KERNEL_TASK_SLOT, STATE, TASKS};
use kernel::{BootInfo, Task, trap};
use kernel::memory::{heap, page_allocator};

/// Initialize kernel state from the bootloader handoff and optional state blob.
pub fn init_kernel(state_ptr: *const u8, state_len: usize, boot_info_ptr: *const BootInfo) {
    let boot_info = unsafe { boot_info_ptr.as_ref() };
    if let Some(info) = init_boot_info(boot_info) {
        unsafe {
            page_allocator::init(info);
            heap::init(info.heap_ptr, info.va_base, info.va_len);
        }
        trap::init_trap_vector(info.kstack_top);
        init_state(state_ptr, state_len);
    } else {
        panic!("init_kernel: missing boot info");
    }
    log!("kernel initialized");
}

fn init_state(state_ptr: *const u8, state_len: usize) {
    unsafe {
        let state_slot = STATE.get_mut();
        if !state_ptr.is_null() && state_len > 0 {
            let bytes = slice::from_raw_parts(state_ptr, state_len);
            *state_slot = State::decode(bytes).or_else(|| {
                log!("state decode failed; starting empty state");
                Some(State::new())
            });
            if state_slot.is_some() {
                logf!("state initialized (len=%d)", state_len as u32);
            }
        } else {
            *state_slot = Some(State::new());
        }
    }
}

fn init_boot_info(boot_info: Option<&BootInfo>) -> Option<&BootInfo> {
    logf!(
        "init_boot_info: boot_info_ptr=0x%x",
        boot_info
            .map(|info| info as *const BootInfo as usize as u32)
            .unwrap_or(0)
    );
    if let Some(info) = boot_info {
        let task = Task::kernel(
            info.root_ppn,
            info.heap_ptr,
            info.va_base,
            info.va_len,
        );
        unsafe {
            let tasks_slot = TASKS.get_mut();
            if tasks_slot.set_at(KERNEL_TASK_SLOT, task).is_err() {
                log!("kernel task slot unavailable; kernel task not recorded");
            }
            *CURRENT_TASK.get_mut() = KERNEL_TASK_SLOT;
        }
        logf!(
            "boot_info: root_ppn=0x%x kstack_top=0x%x heap_ptr=0x%x mem_size=%d",
            info.root_ppn,
            info.kstack_top,
            info.heap_ptr,
            info.memory_size
        );
        Some(info)
    } else {
        log!("boot_info missing; kernel task not initialized");
        None
    }
}
