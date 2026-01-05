#![no_std]
#![no_main]

extern crate alloc;

use core::slice;
use kernel::BootInfo;
use clibc::log;

mod init;
mod init_boot;
mod bundle;
use crate::bundle::{decode_bundle, process_bundle};
use crate::init::init_kernel;

#[allow(dead_code)]
const KERNEL_TASK_IDX: usize = 0;

/// Kernel entrypoint. Receives:
/// - `bundle_ptr`/`bundle_len`: encoded `TransactionBundle` prepared by the bootloader.
/// - `state_ptr`/`state_len`: optional state blob (currently unused).
/// - `boot_info_ptr`: bootloader handoff with stack + page-table root info.
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    bundle_ptr: *const u8,
    bundle_len: usize,
    state_ptr: *const u8,
    state_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel boot");

    init_kernel(state_ptr, state_len, boot_info_ptr);

    let encoded_bundle = unsafe { slice::from_raw_parts(bundle_ptr, bundle_len) };
    if decode_bundle(encoded_bundle) {
        process_bundle();
    } else {
        log!("bundle decode failed");
    }

    log!("finished bundle execution");
    halt();
}

#[inline(never)]
fn halt() -> ! {
    unsafe { core::arch::asm!("ebreak") };
    loop {}
}
