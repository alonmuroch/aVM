use core::fmt;
use types::result::Result as VmResult;

/// Minimal trapframe capturing user-visible registers on trap/return.
/// This mirrors RISC-V general-purpose regs plus PC.
#[derive(Clone, Copy, Default)]
pub struct TrapFrame {
    /// General-purpose registers x0-x31 (x0 is always zero when restored).
    pub regs: [u32; 32],
    /// Program counter to resume at when returning to user.
    pub pc: u32,
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("pc", &format_args!("0x{:08x}", self.pc))
            .finish()
    }
}

/// Describes a process/thread address space.
/// In a real kernel this would own the page table root PPN and ASID.
#[derive(Debug, Clone, Copy)]
pub struct AddressSpace {
    /// Root page-table PPN (satp PPN field) for this address space.
    pub root_ppn: u32,
    /// Optional address-space identifier (ASID); zero if unused.
    pub asid: u16,
    /// Base virtual address for this address space's mapped window.
    pub va_base: u32,
    /// Size in bytes of the mapped virtual window.
    pub va_len: u32,
}

impl AddressSpace {
    pub fn new(root_ppn: u32, asid: u16, va_base: u32, va_len: u32) -> Self {
        Self {
            root_ppn,
            asid,
            va_base,
            va_len,
        }
    }
}

/// Kernel-owned per-task state. This is where the kernel stores the
/// current address-space root and the saved trapframe.
#[derive(Debug)]
pub struct Task {
    /// Saved user trapframe (regs + pc) to restore on return.
    pub tf: TrapFrame,
    /// Address space for this task (page-table root/asid).
    pub addr_space: AddressSpace,
    /// Next heap pointer for this task (virtual address).
    pub heap_ptr: u32,
    /// Task slot that initiated this task, if any.
    pub caller_task_id: Option<usize>,
    /// Last decoded program result for this task, if any.
    pub last_result: Option<VmResult>,
}

impl Task {
    pub fn new(addr_space: AddressSpace, heap_ptr: u32) -> Self {
        Self {
            tf: TrapFrame::default(),
            addr_space,
            heap_ptr,
            caller_task_id: None,
            last_result: None,
        }
    }

    /// Create the initial kernel task. This represents the supervisor itself:
    /// - `root_ppn` is the kernel page-table root PPN that will be loaded into satp.
    pub fn kernel(root_ppn: u32, heap_ptr: u32, va_base: u32, va_len: u32) -> Self {
        Task::new(AddressSpace::new(root_ppn, 0, va_base, va_len), heap_ptr)
    }
}
