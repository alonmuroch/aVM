use clibc::{log, logf};
use types::{SV32_DIRECT_MAP_BASE, SV32_PAGE_SIZE};

use crate::global::{CURRENT_TASK, TASKS};
use crate::memory::page_allocator as mmu;

pub(crate) fn sys_panic_with_message(msg_ptr: u32, msg_len: u32) -> u32 {
    if msg_ptr == 0 || msg_len == 0 {
        log!("sys_panic: empty message");
        halt();
    }

    let current = unsafe { *CURRENT_TASK.get_mut() };
    let tasks = unsafe { TASKS.get_mut() };
    let task = match tasks.get(current) {
        Some(task) => task,
        None => {
            logf!("sys_panic: no current task for slot %d", current as u32);
            halt();
        }
    };
    let root_ppn = task.addr_space.root_ppn;

    let mut buf = [0u8; 256];
    let mut remaining = core::cmp::min(msg_len as usize, buf.len());
    let mut dst_off = 0usize;
    let mut va = msg_ptr;
    while remaining > 0 {
        let phys = match mmu::translate(root_ppn, va) {
            Some(p) => p,
            None => {
                logf!("sys_panic: invalid msg ptr 0x%x", va);
                halt();
            }
        };
        let page_off = (va as usize) & (SV32_PAGE_SIZE - 1);
        let to_copy = core::cmp::min(remaining, SV32_PAGE_SIZE - page_off);
        let src = SV32_DIRECT_MAP_BASE as usize + phys;
        unsafe {
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                buf.as_mut_ptr().add(dst_off),
                to_copy,
            );
        }
        remaining -= to_copy;
        dst_off += to_copy;
        va = va.wrapping_add(to_copy as u32);
    }

    let msg = &buf[..dst_off];
    if let Ok(s) = core::str::from_utf8(msg) {
        logf!("guest panic: %s", s.as_ptr() as u32, s.len() as u32);
    } else {
        log!("guest panic");
    }
    halt();
}

pub(crate) fn sys_panic(args: [u32; 6]) -> u32 {
    // Legacy path: treat args as [ptr, len] when a0/a1 aren't forwarded.
    sys_panic_with_message(args[0], args[1])
}

#[inline(never)]
fn halt() -> ! {
    unsafe { core::arch::asm!("ebreak") };
    loop {
        core::hint::spin_loop();
    }
}
