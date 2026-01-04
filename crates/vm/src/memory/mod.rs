use std::cell::Ref;
use std::rc::Rc;

use crate::metering::{MemoryAccessKind, Metering};

mod sv32;

pub use sv32::Sv32Memory;
pub use types::mmu::*;

pub const HEAP_PTR_OFFSET: u32 = 0x100;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: u32 = 12;
pub const VPN_MASK: u32 = 0x3ff;
pub const PAGE_OFFSET_MASK: u32 = 0xfff;

/// Simple permission bits for page mappings (mirrors Sv32 R/W/X/U).
#[derive(Clone, Copy, Debug)]
pub struct Perms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub user: bool,
}

impl Perms {
    pub const fn new(read: bool, write: bool, exec: bool, user: bool) -> Self {
        Self {
            read,
            write,
            exec,
            user,
        }
    }

    pub fn rwx_kernel() -> Self {
        Self::new(true, true, true, false)
    }

    pub fn rw_kernel() -> Self {
        Self::new(true, true, false, false)
    }
}

/// Sv32 virtual address helper newtype.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtualAddress(pub u32);

impl VirtualAddress {
    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn offset(self) -> u32 {
        self.0 & PAGE_OFFSET_MASK
    }

    pub fn vpn0(self) -> u32 {
        (self.0 >> PAGE_SHIFT) & VPN_MASK
    }

    pub fn vpn1(self) -> u32 {
        (self.0 >> (PAGE_SHIFT + 10)) & VPN_MASK
    }

    pub fn align_down(self) -> Self {
        VirtualAddress(self.0 & !(PAGE_OFFSET_MASK))
    }

    pub fn wrapping_add(self, value: u32) -> Self {
        VirtualAddress(self.0.wrapping_add(value))
    }

    pub fn checked_add(self, value: u32) -> Option<Self> {
        self.0.checked_add(value).map(VirtualAddress)
    }
}

impl From<u32> for VirtualAddress {
    fn from(value: u32) -> Self {
        VirtualAddress(value)
    }
}

impl From<usize> for VirtualAddress {
    fn from(value: usize) -> Self {
        VirtualAddress(value as u32)
    }
}

impl From<VirtualAddress> for usize {
    fn from(value: VirtualAddress) -> Self {
        value.as_usize()
    }
}

pub trait MMU: std::fmt::Debug {
    // --- CPU-facing data access (loads/stores/fetches) ---
    fn mem(&self) -> Ref<Vec<u8>>;
    fn mem_slice(&self, start: VirtualAddress, end: VirtualAddress) -> Option<std::cell::Ref<[u8]>>;
    fn store_u16(&self, addr: VirtualAddress, val: u16, metering: &mut dyn Metering, kind: MemoryAccessKind) -> bool;
    fn store_u32(&self, addr: VirtualAddress, val: u32, metering: &mut dyn Metering, kind: MemoryAccessKind) -> bool;
    fn store_u8(&self, addr: VirtualAddress, val: u8, metering: &mut dyn Metering, kind: MemoryAccessKind) -> bool;
    fn load_u32(&self, addr: VirtualAddress, metering: &mut dyn Metering, kind: MemoryAccessKind) -> Option<u32>;
    fn load_byte(&self, addr: VirtualAddress, metering: &mut dyn Metering, kind: MemoryAccessKind) -> Option<u8>;
    fn load_halfword(&self, addr: VirtualAddress, metering: &mut dyn Metering, kind: MemoryAccessKind) -> Option<u16>;
    fn load_word(&self, addr: VirtualAddress, metering: &mut dyn Metering, kind: MemoryAccessKind) -> Option<u32>;
}

pub trait API: std::fmt::Debug {
    fn map_range(&self, start: VirtualAddress, len: usize, perms: Perms);
    /// Get the current page-table root (index/identifier).
    fn current_root(&self) -> usize;
    /// Read the current satp value.
    fn satp(&self) -> u32;
    /// Set satp (PPN field is used for the root in this emulator).
    fn set_satp(&self, satp: u32);
    /// Top of the stack for this memory layout.
    fn stack_top(&self) -> VirtualAddress;
    fn size(&self) -> usize;
    fn offset(&self, addr: VirtualAddress) -> usize;
}

pub trait Mmu: MMU + API {}

impl<T: MMU + API> Mmu for T {}

pub type Memory = Rc<dyn Mmu>;
