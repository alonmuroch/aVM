#![no_std]
#![no_main]

extern crate alloc;

// Memory mapping test: user range mapping, kernel mirroring, translate/copy/peek.
use clibc::log;
use kernel::BootInfo;
use kernel::memory::page_allocator::{self, PagePerms};

#[path = "../../tests/results.rs"]
mod results;
#[path = "../../tests/fail.rs"]
mod fail;
#[path = "../../tests/utils.rs"]
mod utils;

#[unsafe(no_mangle)]
pub extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel mem map test boot");
    let info = utils::init_test_kernel(boot_info_ptr);

    clibc::logf!("kernel test input len: %d", input_len as u32);
    let _input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

    let user_root = page_allocator::alloc_root().unwrap_or(0);
    if user_root == 0 {
        fail::fail(1);
    }

    let (va_start, len) = pick_user_range(info);

    if let Err(code) = test_user_map(user_root, va_start, len) {
        fail::fail(code);
    }
    if let Err(code) = test_read_only_mapping(user_root, info, va_start, len) {
        fail::fail(code);
    }
    if let Err(code) = test_exec_mapping(user_root, info, va_start, len) {
        fail::fail(code);
    }
    if let Err(code) = test_kernel_sees_different_phys_before_mirror(user_root, va_start) {
        fail::fail(code);
    }
    if let Err(code) = test_mirror(user_root, va_start, len) {
        fail::fail(code);
    }
    if let Err(code) = test_translate(user_root, va_start) {
        fail::fail(code);
    }
    if let Err(code) = test_copy_peek(user_root, va_start) {
        fail::fail(code);
    }
    if let Err(code) = test_user_cannot_translate_kernel_only(user_root, info) {
        fail::fail(code);
    }

    log!("kernel mem map test done");
    utils::pass();
}

fn pick_user_range(info: BootInfo) -> (u32, usize) {
    let window_start = info.va_base;
    let window_end = info.va_base.saturating_add(info.va_len);
    let mut va_start = window_start.saturating_add(0x20_000);
    let len = 0x1000usize;
    if va_start.saturating_add(len as u32) > window_end {
        va_start = window_start.saturating_add(0x1000);
    }
    (va_start, len)
}

fn test_user_map(user_root: u32, va_start: u32, len: usize) -> Result<(), u32> {
    // Map a user R/W range into the user root.
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, va_start, len, perms) {
        return Err(2);
    }
    // Verify the mapping exists and can be written via the user root.
    let phys = page_allocator::translate(user_root, va_start).unwrap_or(0);
    if phys == 0 {
        return Err(12);
    }
    let data = [0x0bu8, 0x0c, 0x0d, 0x0e];
    if !page_allocator::copy_user(user_root, va_start, &data) {
        return Err(13);
    }
    Ok(())
}

fn test_read_only_mapping(
    user_root: u32,
    info: BootInfo,
    base_va: u32,
    len: usize,
) -> Result<(), u32> {
    // Map a user read-only range and ensure writes are rejected by copy_user.
    let window_end = info.va_base.saturating_add(info.va_len);
    let mut ro_va = base_va.saturating_add(len as u32).saturating_add(0x1000);
    if ro_va.saturating_add(len as u32) > window_end {
        ro_va = info.va_base.saturating_add(0x2000);
    }
    let perms = PagePerms::new(true, false, false, true);
    if !page_allocator::map_range_for_root(user_root, ro_va, len, perms) {
        return Err(10);
    }
    let data = [0x5au8, 0xa5];
    if page_allocator::copy_user(user_root, ro_va, &data) {
        return Err(11);
    }
    Ok(())
}

fn test_exec_mapping(
    user_root: u32,
    info: BootInfo,
    base_va: u32,
    len: usize,
) -> Result<(), u32> {
    // Map a user exec-only range and ensure writes are rejected.
    let window_end = info.va_base.saturating_add(info.va_len);
    let mut exec_va = base_va.saturating_add(len as u32).saturating_add(0x2000);
    if exec_va.saturating_add(len as u32) > window_end {
        exec_va = info.va_base.saturating_add(0x3000);
    }
    let perms = PagePerms::new(false, false, true, true);
    if !page_allocator::map_range_for_root(user_root, exec_va, len, perms) {
        return Err(14);
    }
    let data = [0x7eu8, 0x7f];
    if page_allocator::copy_user(user_root, exec_va, &data) {
        return Err(15);
    }
    Ok(())
}

fn test_mirror(user_root: u32, va_start: u32, len: usize) -> Result<(), u32> {
    // Mirror the user range into the kernel root so it is accessible in kernel.
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::mirror_user_range_into_kernel(user_root, va_start, len, perms) {
        return Err(3);
    }
    Ok(())
}

fn test_kernel_sees_different_phys_before_mirror(user_root: u32, va_start: u32) -> Result<(), u32> {
    // Ensure kernel/user roots resolve the same VA to different physical pages pre-mirror.
    let kernel_root = page_allocator::current_root();
    let phys_user = page_allocator::translate(user_root, va_start).unwrap_or(0);
    let phys_kernel = page_allocator::translate(kernel_root, va_start).unwrap_or(0);
    if phys_user == 0 || phys_kernel == 0 {
        return Err(9);
    }
    if phys_user == phys_kernel {
        return Err(9);
    }
    Ok(())
}

fn test_translate(user_root: u32, va_start: u32) -> Result<(), u32> {
    // Ensure mirror caused kernel/user translations to resolve to the same physical page.
    let kernel_root = page_allocator::current_root();
    let phys_user = page_allocator::translate(user_root, va_start).unwrap_or(0);
    let phys_kernel = page_allocator::translate(kernel_root, va_start).unwrap_or(0);
    if phys_user == 0 || phys_kernel == 0 || phys_user != phys_kernel {
        return Err(4);
    }
    Ok(())
}

fn test_copy_peek(user_root: u32, va_start: u32) -> Result<(), u32> {
    // Copy via user root and read via kernel root to verify shared physical mapping.
    let kernel_root = page_allocator::current_root();
    let data = [0x11u8, 0x22, 0x33, 0x44];
    if !page_allocator::copy(user_root, va_start, &data) {
        return Err(5);
    }
    let word = page_allocator::peek_word(kernel_root, va_start).unwrap_or(0);
    if word != 0x4433_2211 {
        return Err(6);
    }
    Ok(())
}

fn test_user_cannot_translate_kernel_only(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Map kernel-only memory and ensure user root cannot translate it.
    let kernel_only_va = info.va_base.saturating_add(0x300_000);
    let len = 0x1000usize;
    if !page_allocator::map_kernel_range(kernel_only_va, len, PagePerms::kernel_rw()) {
        return Err(7);
    }
    if page_allocator::translate(user_root, kernel_only_va).is_some() {
        return Err(8);
    }
    Ok(())
}
