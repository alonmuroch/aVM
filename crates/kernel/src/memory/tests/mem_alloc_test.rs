#![no_std]
#![no_main]

extern crate alloc;

// Memory allocation test: heap alignment, heap window exhaustion, page allocator roots.
use clibc::log;
use kernel::BootInfo;
use kernel::memory::{heap, page_allocator};

#[path = "../../tests/results.rs"]
mod results;
#[path = "../../tests/utils.rs"]
mod utils;

#[unsafe(no_mangle)]
pub extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel mem alloc test boot");
    let info = utils::init_test_kernel(boot_info_ptr);

    clibc::logf!("kernel test input len: %d", input_len as u32);
    let _input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

    if let Err(code) = test_heap_alignment() {
        utils::fail(code);
    }
    if let Err(code) = test_heap_exhaustion(info) {
        utils::fail(code);
    }
    if let Err(code) = test_page_allocator_roots() {
        utils::fail(code);
    }
    if let Err(code) = test_heap_too_large() {
        utils::fail(code);
    }

    utils::pass();
}

fn test_heap_alignment() -> Result<(), u32> {
    let ptr = heap::alloc(32, 16).unwrap_or(core::ptr::null_mut());
    if ptr.is_null() {
        return Err(1);
    }
    if (ptr as usize) & 0x0f != 0 {
        return Err(2);
    }
    heap::dealloc(ptr, 32, 16);
    Ok(())
}

fn test_heap_exhaustion(info: BootInfo) -> Result<(), u32> {
    let window_end = info.va_base.saturating_add(info.va_len) as usize;
    let available = window_end.saturating_sub(info.heap_ptr as usize);
    if heap::alloc(available.saturating_add(16), 8).is_some() {
        return Err(3);
    }
    Ok(())
}

fn test_page_allocator_roots() -> Result<(), u32> {
    let root1 = page_allocator::alloc_root().unwrap_or(0);
    let root2 = page_allocator::alloc_root().unwrap_or(0);
    if root1 == 0 || root2 == 0 || root1 == root2 {
        return Err(4);
    }
    Ok(())
}

fn test_heap_too_large() -> Result<(), u32> {
    if heap::alloc(usize::MAX, 8).is_some() {
        return Err(5);
    }
    Ok(())
}
