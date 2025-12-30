//! Kernel-to-bootloader handoff header for serialized receipts.

/// Pointer + length describing the receipts buffer in kernel memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct KernelResult {
    pub receipts_ptr: u32,
    pub receipts_len: u32,
}

/// Kernel VA where the handoff header is written.
pub const KERNEL_RESULT_ADDR: u32 = 0x100;
