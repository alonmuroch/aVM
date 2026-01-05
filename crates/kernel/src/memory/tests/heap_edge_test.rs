#![no_std]
#![no_main]

// Heap edge tests: invalid layouts and monotonic bump behavior.
use clibc::log;
use kernel::BootInfo;
use kernel::memory::heap;

#[path = "../../tests/fail.rs"]
mod fail;
#[path = "../../tests/results.rs"]
mod results;
#[path = "../../tests/utils.rs"]
mod utils;

#[unsafe(no_mangle)]
pub extern "C" fn _start(input_ptr: *const u8, input_len: usize, boot_info_ptr: *const BootInfo) {
    log!("kernel heap edge test boot");
    let _info = utils::init_test_kernel(boot_info_ptr);

    clibc::logf!("kernel test input len: %d", input_len as u32);
    let _input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

    if let Err(code) = test_invalid_layouts() {
        fail::fail(code);
    }
    if let Err(code) = test_monotonic_bump_and_data() {
        fail::fail(code);
    }

    log!("kernel heap edge test done");
    utils::pass();
}

fn test_invalid_layouts() -> Result<(), u32> {
    // Description: heap::alloc must reject invalid size/align values.
    log!("test: invalid heap layouts are rejected");
    log!("subtest: zero size and zero align are rejected");

    if heap::alloc(0, 8).is_some() {
        return Err(10);
    }
    if heap::alloc(16, 0).is_some() {
        return Err(11);
    }

    log!("subtest: non power-of-two alignment is rejected");
    if heap::alloc(16, 3).is_some() {
        return Err(12);
    }

    log!("subtest: size overflow is rejected");
    if heap::alloc(usize::MAX, 16).is_some() {
        return Err(13);
    }
    Ok(())
}

fn test_monotonic_bump_and_data() -> Result<(), u32> {
    // Description: allocations should be monotonic and memory is usable.
    log!("test: monotonic bump allocator behavior");
    log!("subtest: allocations are ordered and writable");

    let a = heap::alloc(32, 8).unwrap_or(core::ptr::null_mut());
    let b = heap::alloc(32, 8).unwrap_or(core::ptr::null_mut());
    if a.is_null() || b.is_null() {
        return Err(20);
    }
    if (b as usize) <= (a as usize) {
        return Err(21);
    }
    unsafe {
        a.write_bytes(0xab, 32);
        b.write_bytes(0xcd, 32);
    }
    let a_first = unsafe { a.read() };
    let b_first = unsafe { b.read() };
    if a_first != 0xab || b_first != 0xcd {
        return Err(22);
    }

    log!("subtest: dealloc is a no-op and allocations keep increasing");
    heap::dealloc(a, 32, 8);
    let c = heap::alloc(16, 8).unwrap_or(core::ptr::null_mut());
    if c.is_null() || (c as usize) <= (b as usize) {
        return Err(23);
    }
    Ok(())
}
