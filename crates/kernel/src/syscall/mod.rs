//! Kernel-owned syscall stubs. These mirror the bootloader syscalls but
//! are now dispatched from the kernel trap handler. Implementations will
//! land here; for now they panic to make missing pieces explicit.
use clibc::{log, logf};
use clibc::syscalls::{
    SYSCALL_ALLOC, SYSCALL_BALANCE, SYSCALL_BRK, SYSCALL_CALL_PROGRAM, SYSCALL_DEALLOC,
    SYSCALL_FIRE_EVENT, SYSCALL_PANIC, SYSCALL_STORAGE_GET, SYSCALL_STORAGE_SET,
    SYSCALL_TRANSFER,
};

pub mod alloc;
pub mod call_program;
pub mod fire_event;
pub mod panic;
pub mod storage;
pub mod balance;

use alloc::{sys_alloc, sys_dealloc};
use balance::{sys_balance, sys_transfer};
use call_program::sys_call_program;
use fire_event::sys_fire_event;
use panic::sys_panic;
use storage::{sys_storage_get, sys_storage_set};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallerMode {
    User,
    Supervisor,
}

pub struct SyscallContext<'a> {
    pub regs: &'a mut [u32],
    pub caller_mode: CallerMode,
}

pub trait SyscallHandler: core::fmt::Debug {
    fn handle_syscall(&mut self, call_id: u32, args: [u32; 6], ctx: &mut SyscallContext<'_>) -> u32;
}

pub fn dispatch_syscall(call_id: u32, args: [u32; 6], ctx: &mut SyscallContext<'_>) -> u32 {
    match call_id {
        SYSCALL_STORAGE_GET => sys_storage_get(args),
        SYSCALL_STORAGE_SET => sys_storage_set(args),
        SYSCALL_PANIC => sys_panic(args),
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

fn sys_brk(_args: [u32; 6]) -> u32 {
    log!("sys_brk: need implementation");
    0
}
