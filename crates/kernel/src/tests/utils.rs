use clibc::log;
use kernel::memory::{heap, page_allocator};
use kernel::{trap, BootInfo};
use crate::results;

#[path = "../init_boot.rs"]
mod init_boot;

pub fn init_test_kernel(boot_info_ptr: *const BootInfo) -> BootInfo {
    let boot_info = unsafe { boot_info_ptr.as_ref() };
    if let Some(info) = init_boot::init_boot_info(boot_info) {
        page_allocator::init(info);
        heap::init(info.heap_ptr, info.va_base, info.va_len);
        trap::init_trap_vector(info.kstack_top);
        let info_copy = *info;
        log!("kernel initialized");
        info_copy
    } else {
        panic!("init_test_kernel: missing boot info");
    }
}

pub fn pass() -> ! {
    unsafe { results::write_results(results::TestResults::pass(0)) };
    halt();
}

#[inline(never)]
pub fn halt() -> ! {
    unsafe { core::arch::asm!("ebreak") };
    loop {}
}
