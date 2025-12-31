//! Kernel-owned syscall stubs. These mirror the bootloader syscalls but
//! are now dispatched from the kernel trap handler. Implementations will
//! land here; for now they panic to make missing pieces explicit.
use program::{log, logf};

pub mod alloc;
pub mod call_program;
pub mod fire_event;
pub mod panic;
pub mod storage;

use alloc::{sys_alloc, sys_dealloc};
use call_program::sys_call_program;
use fire_event::sys_fire_event;
use panic::sys_panic;
use storage::{sys_storage_get, sys_storage_set};
pub(crate) use panic::sys_panic_with_message;

pub const SYSCALL_STORAGE_GET: u32 = 1;
pub const SYSCALL_STORAGE_SET: u32 = 2;
pub const SYSCALL_PANIC: u32 = 3;
pub const SYSCALL_LOG: u32 = 100;
pub const SYSCALL_CALL_PROGRAM: u32 = 5;
pub const SYSCALL_FIRE_EVENT: u32 = 6;
pub const SYSCALL_ALLOC: u32 = 7;
pub const SYSCALL_DEALLOC: u32 = 8;
pub const SYSCALL_TRANSFER: u32 = 9;
pub const SYSCALL_BALANCE: u32 = 10;
pub const SYSCALL_BRK: u32 = 214;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallerMode {
    User,
    Supervisor,
}

pub struct SyscallContext<'a> {
    pub regs: &'a mut [u32],
    pub caller_mode: CallerMode,
}

pub fn dispatch_syscall(call_id: u32, args: [u32; 6], ctx: &mut SyscallContext<'_>) -> u32 {
    match call_id {
        SYSCALL_STORAGE_GET => sys_storage_get(args),
        SYSCALL_STORAGE_SET => sys_storage_set(args),
        SYSCALL_PANIC => sys_panic(args),
        SYSCALL_LOG => sys_log(args, ctx.caller_mode),
        SYSCALL_CALL_PROGRAM => sys_call_program(args, ctx),
        SYSCALL_FIRE_EVENT => sys_fire_event(args),
        SYSCALL_ALLOC => sys_alloc(args),
        SYSCALL_DEALLOC => sys_dealloc(args),
        SYSCALL_TRANSFER => sys_transfer(args),
        SYSCALL_BALANCE => sys_balance(args),
        SYSCALL_BRK => sys_brk(args),
        _ => {
            logf!("unknown syscall id %d", call_id);
            0
        }
    }
}

fn sys_log(_args: [u32; 6], caller_mode: CallerMode) -> u32 {
    if caller_mode == CallerMode::Supervisor {
        log!("kernel: sys_log: need implementation");
    } else {
        log!("guest: sys_log: need implementation");
    }
    0
}


fn sys_transfer(_args: [u32; 6]) -> u32 {
    log!("sys_transfer: need implementation");
    0
}

fn sys_balance(_args: [u32; 6]) -> u32 {
    log!("sys_balance: need implementation");
    0
}

fn sys_brk(_args: [u32; 6]) -> u32 {
    log!("sys_brk: need implementation");
    0
}
