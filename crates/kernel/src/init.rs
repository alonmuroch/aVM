use core::slice;

use clibc::{log, logf};
use state::State;

use kernel::global::STATE;
use kernel::memory::{heap, page_allocator};
use kernel::{BootInfo, trap};

/// Initialize kernel state from the bootloader handoff and optional state blob.
pub fn init_kernel(state_ptr: *const u8, state_len: usize, boot_info_ptr: *const BootInfo) {
    let boot_info = unsafe { boot_info_ptr.as_ref() };
    if let Some(info) = crate::init_boot::init_boot_info(boot_info) {
        page_allocator::init(info);
        heap::init(info.heap_ptr, info.va_base, info.va_len);
        trap::init_trap_vector(info.kstack_top);
        init_state(state_ptr, state_len);
    } else {
        panic!("init_kernel: missing boot info");
    }
    log!("kernel initialized");
}

fn init_state(state_ptr: *const u8, state_len: usize) {
    unsafe {
        let state_slot = STATE.get_mut();
        if !state_ptr.is_null() && state_len > 0 {
            let bytes = slice::from_raw_parts(state_ptr, state_len);
            *state_slot = State::decode(bytes).or_else(|| {
                log!("state decode failed; starting empty state");
                Some(State::new())
            });
            if state_slot.is_some() {
                logf!("state initialized (len=%d)", state_len as u32);
            }
        } else {
            *state_slot = Some(State::new());
        }
    }
}
