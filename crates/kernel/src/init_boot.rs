use clibc::{log, logf};

use kernel::global::{CURRENT_TASK, KERNEL_TASK_SLOT, TASKS};
use kernel::{BootInfo, Task};

pub(crate) fn init_boot_info(boot_info: Option<&BootInfo>) -> Option<&BootInfo> {
    logf!(
        "init_boot_info: boot_info_ptr=0x%x",
        boot_info
            .map(|info| info as *const BootInfo as usize as u32)
            .unwrap_or(0)
    );
    if let Some(info) = boot_info {
        let task = Task::kernel(info.root_ppn, info.heap_ptr, info.va_base, info.va_len);
        unsafe {
            let tasks_slot = TASKS.get_mut();
            if !tasks_slot.set_at(KERNEL_TASK_SLOT, task) {
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
