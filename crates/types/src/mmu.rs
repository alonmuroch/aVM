#![allow(dead_code)]

use core::convert::TryFrom;
use core::mem;

/// Sv32 page size in bytes (4 KiB).
pub const SV32_PAGE_SIZE: usize = 4096;
/// Number of bits in a VPN index.
pub const SV32_VPN_MASK: u32 = 0x3ff;

/// Sv32 PTE flag bits.
pub const SV32_PTE_V: u32 = 1 << 0;
pub const SV32_PTE_R: u32 = 1 << 1;
pub const SV32_PTE_W: u32 = 1 << 2;
pub const SV32_PTE_X: u32 = 1 << 3;
pub const SV32_PTE_U: u32 = 1 << 4;

/// Sv32 satp PPN mask (bits 21:0). Mode bits are ignored in this emulator.
pub const SV32_SATP_PPN_MASK: u32 = 0x003f_ffff;

/// Virtual base used to directly map all guest physical memory for the kernel.
/// This keeps physical page-table frames accessible even after paging is enabled.
pub const SV32_DIRECT_MAP_BASE: u32 = 0x4000_0000;

/// Simple permission descriptor for Sv32 mappings.
#[derive(Clone, Copy, Debug)]
pub struct Sv32PagePerms {
    /// Allow load access (read data from this page).
    pub read: bool,
    /// Allow store access (write data to this page).
    pub write: bool,
    /// Allow instruction fetches from this page.
    pub exec: bool,
    /// Mark the page as user-accessible (U bit set).
    pub user: bool,
}

impl Sv32PagePerms {
    pub const fn new(read: bool, write: bool, exec: bool, user: bool) -> Self {
        Self {
            read,
            write,
            exec,
            user,
        }
    }

    pub const fn user_rwx() -> Self {
        Self::new(true, true, true, true)
    }

    pub const fn kernel_rw() -> Self {
        Self::new(true, true, false, false)
    }

    pub const fn kernel_rwx() -> Self {
        Self::new(true, true, true, false)
    }

    fn to_pte_flags(self) -> u32 {
        let mut flags = SV32_PTE_V;
        if self.read {
            flags |= SV32_PTE_R;
        }
        if self.write {
            flags |= SV32_PTE_W;
        }
        if self.exec {
            flags |= SV32_PTE_X;
        }
        if self.user {
            flags |= SV32_PTE_U;
        }
        flags
    }
}

/// Abstraction for Sv32 page-table manipulation.
///
/// Implementations provide raw PTE reads/writes at physical addresses, frame
/// allocation, and zeroing. Mapping helpers below drive the common Sv32 walk
/// for both the bootloader and kernel.
pub trait Sv32PageTable {
    fn page_size(&self) -> usize {
        SV32_PAGE_SIZE
    }

    fn read_pte(&self, phys_addr: usize) -> Option<u32>;
    fn write_pte(&self, phys_addr: usize, val: u32);
    fn alloc_frame(&self) -> Option<u32>;
    fn zero_frame(&self, ppn: u32);
}

/// Map a virtual range by allocating fresh physical frames for leaves.
///
/// Returns false on allocation/overflow failures or unsupported superpage cases.
pub fn map_allocating<T: Sv32PageTable>(
    pt: &T,
    root_ppn: u32,
    va_start: u32,
    len: usize,
    perms: Sv32PagePerms,
) -> bool {
    map_range_internal(pt, root_ppn, va_start, len, perms, LeafStrategy::Allocate)
}

/// Map a virtual range to an existing physical range (no leaf allocation).
///
/// `phys_start` must be page aligned. Returns false on failure.
pub fn map_to_physical<T: Sv32PageTable>(
    pt: &T,
    root_ppn: u32,
    va_start: u32,
    phys_start: u32,
    len: usize,
    perms: Sv32PagePerms,
) -> bool {
    if !(phys_start as usize).is_multiple_of(pt.page_size()) {
        return false;
    }
    map_range_internal(
        pt,
        root_ppn,
        va_start,
        len,
        perms,
        LeafStrategy::PhysCursor {
            next_phys: phys_start,
        },
    )
}

#[derive(Clone, Copy)]
enum LeafStrategy {
    Allocate,
    PhysCursor { next_phys: u32 },
}

fn map_range_internal<T: Sv32PageTable>(
    pt: &T,
    root_ppn: u32,
    va_start: u32,
    len: usize,
    perms: Sv32PagePerms,
    mut strategy: LeafStrategy,
) -> bool {
    if len == 0 {
        return true;
    }

    let page_size = pt.page_size();
    let start = align_down(va_start as usize, page_size);
    let end = match (va_start as usize).checked_add(len) {
        Some(v) => align_up(v, page_size),
        None => return false,
    };

    let mut va = start as u32;
    while (va as usize) < end {
        let phys_override = match &mut strategy {
            LeafStrategy::Allocate => None,
            LeafStrategy::PhysCursor { next_phys } => {
                let phys = *next_phys;
                *next_phys = next_phys.wrapping_add(page_size as u32);
                Some(phys)
            }
        };

        if !map_page(pt, root_ppn, va, perms, phys_override) {
            return false;
        }
        va = va.wrapping_add(page_size as u32);
    }
    true
}

fn map_page<T: Sv32PageTable>(
    pt: &T,
    root_ppn: u32,
    va: u32,
    perms: Sv32PagePerms,
    phys_override: Option<u32>,
) -> bool {
    let page_size = pt.page_size();
    let vpn1 = (va >> 22) & SV32_VPN_MASK;
    let vpn0 = (va >> 12) & SV32_VPN_MASK;

    let root_base = match (root_ppn as usize).checked_mul(page_size) {
        Some(base) => base,
        None => return false,
    };
    let l1_entry_addr = root_base + vpn1 as usize * mem::size_of::<u32>();
    let mut l1_pte = match pt.read_pte(l1_entry_addr) {
        Some(pte) => pte,
        None => return false,
    };

    if l1_pte & SV32_PTE_V == 0 {
        let l2 = match pt.alloc_frame() {
            Some(ppn) => ppn,
            None => return false,
        };
        pt.zero_frame(l2);
        l1_pte = (l2 << 10) | SV32_PTE_V;
        pt.write_pte(l1_entry_addr, l1_pte);
    } else if l1_pte & (SV32_PTE_R | SV32_PTE_W | SV32_PTE_X) != 0 {
        // Superpages are not supported.
        return false;
    }

    let l2_base = match usize::try_from(l1_pte >> 10)
        .ok()
        .and_then(|ppn| ppn.checked_mul(page_size))
    {
        Some(base) => base,
        None => return false,
    };
    let l2_entry_addr = l2_base + vpn0 as usize * mem::size_of::<u32>();

    let existing = pt.read_pte(l2_entry_addr).unwrap_or(0);
    let existing_valid = (existing & SV32_PTE_V) != 0;
    let existing_ppn = existing >> 10;

    let leaf_ppn = match (existing_valid, phys_override) {
        (true, Some(phys)) => {
            if !(phys as usize).is_multiple_of(page_size) {
                return false;
            }
            phys / page_size as u32
        }
        (true, None) => existing_ppn,
        (false, Some(phys)) => {
            if !(phys as usize).is_multiple_of(page_size) {
                return false;
            }
            phys / page_size as u32
        }
        (false, None) => match pt.alloc_frame() {
            Some(ppn) => {
                pt.zero_frame(ppn);
                ppn
            }
            None => return false,
        },
    };

    let leaf_pte = (leaf_ppn << 10) | perms.to_pte_flags();
    pt.write_pte(l2_entry_addr, leaf_pte);
    true
}

const fn align_up(val: usize, align: usize) -> usize {
    (val + (align - 1)) & !(align - 1)
}

const fn align_down(val: usize, align: usize) -> usize {
    val & !(align - 1)
}
