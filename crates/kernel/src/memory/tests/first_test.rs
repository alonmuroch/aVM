#![no_std]
#![no_main]

extern crate alloc;

// Basic smoke test: init kernel test harness and emit logs.
use core::slice;
use clibc::log;
use kernel::BootInfo;

#[path = "../../tests/utils.rs"]
mod utils;
#[path = "../../tests/results.rs"]
mod results;

#[unsafe(no_mangle)]
pub extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel test boot");
    let _info = utils::init_test_kernel(boot_info_ptr);

    let input = unsafe { slice::from_raw_parts(input_ptr, input_len) };
    clibc::logf!("kernel test input len: %d", input.len() as u32);
    log!("kernel test log-only");

    utils::pass();
}
