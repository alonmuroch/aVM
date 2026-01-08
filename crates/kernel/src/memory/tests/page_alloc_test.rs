#![no_std]
#![no_main]

// Page allocator tests: root zeroing and bump behavior.
use clibc::log;
use kernel::BootInfo;
use kernel::memory::page_allocator;

#[path = "../../tests/fail.rs"]
mod fail;
#[path = "../../tests/results.rs"]
mod results;
#[path = "../../tests/utils.rs"]
mod utils;

/// # Safety
/// The pointers must be valid for the provided lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel page allocator test boot");
    let info = utils::init_test_kernel(boot_info_ptr);

    clibc::logf!("kernel test input len: %d", input_len as u32);
    let _input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

    if let Err(code) = test_alloc_root_zeroed(info) {
        fail::fail(code);
    }
    if let Err(code) = test_bump_allocator_behavior() {
        fail::fail(code);
    }

    log!("kernel page allocator test done");
    utils::pass();
}

fn test_alloc_root_zeroed(info: BootInfo) -> Result<(), u32> {
    // Description: freshly allocated roots should be zeroed (no valid mappings).
    log!("test: alloc_root yields zeroed page table");
    log!("subtest: ensure translation is absent for a fresh root");

    let root = page_allocator::alloc_root().unwrap_or(0);
    if root == 0 {
        return Err(10);
    }
    let va = info.va_base.saturating_add(0x1000);
    if page_allocator::translate(root, va).is_some() {
        return Err(11);
    }
    Ok(())
}

fn test_bump_allocator_behavior() -> Result<(), u32> {
    // Description: bumping should skip page frames and allow exhaustion testing.
    log!("test: bump_page_allocator skips frames");
    log!("subtest: bump to a higher ppn and verify allocations skip ahead");

    let first = page_allocator::alloc_root().unwrap_or(0);
    if first == 0 {
        return Err(20);
    }
    let bump_to = first.saturating_add(4);
    page_allocator::bump_page_allocator(bump_to);
    let second = page_allocator::alloc_root().unwrap_or(0);
    if second < bump_to {
        return Err(21);
    }

    log!("subtest: bump to limit and verify allocator is exhausted");
    let limit = page_allocator::total_ppn().unwrap_or(0);
    if limit == 0 {
        return Err(22);
    }
    page_allocator::bump_page_allocator(limit);
    if page_allocator::alloc_root().is_some() {
        return Err(23);
    }
    Ok(())
}
