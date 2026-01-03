use clibc::log;
use kernel::memory::{heap, page_allocator};
use kernel::{trap, BootInfo};

#[path = "../init.rs"]
mod init;

pub fn init_test_kernel(boot_info_ptr: *const BootInfo) {
    let boot_info = unsafe { boot_info_ptr.as_ref() };
    if let Some(info) = init::init_boot_info(boot_info) {
        unsafe {
            page_allocator::init(info);
            heap::init(info.heap_ptr, info.va_base, info.va_len);
        }
        trap::init_trap_vector(info.kstack_top);
    } else {
        panic!("init_test_kernel: missing boot info");
    }
    log!("kernel initialized");
}
