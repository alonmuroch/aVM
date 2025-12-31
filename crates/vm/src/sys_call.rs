use crate::cpu::PrivilegeMode;
use crate::memory::Memory;
use crate::metering::Metering;
use core::any::Any;

/// System call IDs for the VM.
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
pub const SYSCALL_BRK: u32 = 214;       // brk(2): set program break (heap end)

/// Trait implemented by syscall handlers consumed by the VM.
pub trait SyscallHandler: std::fmt::Debug {
    fn handle_syscall(
        &mut self,
        call_id: u32,
        args: [u32; 6],
        caller_mode: PrivilegeMode,
        memory: Memory,
        regs: &mut [u32; 32],
        metering: &mut dyn Metering,
    ) -> (u32, bool);
    fn as_any(&self) -> &dyn Any;
}
