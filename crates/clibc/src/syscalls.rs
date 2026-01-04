/// System call IDs shared between the guest program ABI and the runtime.
pub const SYSCALL_STORAGE_GET: u32 = 1;
pub const SYSCALL_STORAGE_SET: u32 = 2;
pub const SYSCALL_PANIC: u32 = 3;
pub const SYSCALL_CALL_PROGRAM: u32 = 5;
pub const SYSCALL_FIRE_EVENT: u32 = 6;
pub const SYSCALL_ALLOC: u32 = 7;
pub const SYSCALL_DEALLOC: u32 = 8;
pub const SYSCALL_TRANSFER: u32 = 9;
pub const SYSCALL_BALANCE: u32 = 10;
pub const SYSCALL_BRK: u32 = 214; // brk(2): set program break (heap end)
