#![no_std]
#![no_main]

extern crate alloc;

use core::slice;
use clibc::log;
use kernel::BootInfo;

mod utils;
mod results;

#[unsafe(no_mangle)]
pub extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel test boot");
    utils::init_test_kernel(boot_info_ptr);

    let input = unsafe { slice::from_raw_parts(input_ptr, input_len) };
    clibc::logf!("kernel test input len: %d", input.len() as u32);
    log!("kernel test log-only");

    unsafe { results::write_results(results::TestResults::pass(0)) };
    halt();
}

#[inline(never)]
fn halt() -> ! {
    unsafe { core::arch::asm!("ebreak") };
    loop {}
}
