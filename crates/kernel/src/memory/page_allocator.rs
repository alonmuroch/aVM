use core::{cmp, marker::PhantomData, ptr};

use crate::global::{PAGE_ALLOC, ROOT_PPN};
use crate::BootInfo;
use types::{
    Sv32PagePerms, Sv32PageTable, SV32_DIRECT_MAP_BASE, SV32_PAGE_SIZE, SV32_VPN_MASK, map_allocating,
    SV32_PTE_R, SV32_PTE_W, SV32_PTE_X, SV32_PTE_V, SV32_PTE_U, map_to_physical,
};

const PAGE_SIZE: usize = SV32_PAGE_SIZE;
const DIRECT_MAP_BASE: usize = SV32_DIRECT_MAP_BASE as usize;

/// Permissions used by the kernel/user mapping helpers.
pub type PagePerms = Sv32PagePerms;

#[derive(Debug, Clone, Copy)]
pub struct PageAllocator {
    next_ppn: u32,
    limit_ppn: u32,
}

impl PageAllocator {
    /// Create a bump-frame allocator over [start_ppn, limit_ppn).
    pub const fn new(start_ppn: u32, limit_ppn: u32) -> Self {
        Self {
            next_ppn: start_ppn,
            limit_ppn,
        }
    }

    /// Allocate the next free physical page number, or None if exhausted.
    pub fn alloc(&mut self) -> Option<u32> {
        if self.next_ppn >= self.limit_ppn {
            return None;
        }
        let ppn = self.next_ppn;
        self.next_ppn += 1;
        Some(ppn)
    }

    /// Zero a 4 KiB page in guest physical memory via the direct map.
    fn zero_page(ppn: u32) {
        let base = (ppn as usize)
            .checked_mul(PAGE_SIZE)
            .expect("page offset overflow");
        let virt = direct_map_addr(base).expect("direct map overflow while zeroing page");
        unsafe {
            ptr::write_bytes(virt as *mut u8, 0, PAGE_SIZE);
        }
    }

    /// Advance the allocator so it will not hand out frames below `min_ppn`.
    pub fn bump_to(&mut self, min_ppn: u32) {
        if self.next_ppn < min_ppn {
            self.next_ppn = min_ppn;
        }
    }

    pub fn limit_ppn(&self) -> u32 {
        self.limit_ppn
    }

    pub fn next_ppn(&self) -> u32 {
        self.next_ppn
    }

    pub fn remaining_ppn(&self) -> u32 {
        self.limit_ppn.saturating_sub(self.next_ppn)
    }
}

/// Return the kernel's current root PPN (satp PPN field).
pub fn current_root() -> u32 {
    unsafe { *ROOT_PPN.get_mut() }
}

/// Update the current root PPN used by kernel helpers.
pub fn set_current_root(root_ppn: u32) {
    unsafe {
        *ROOT_PPN.get_mut() = root_ppn;
    }
}

/// Initialize the kernel MMU allocator state from bootloader handoff.
pub fn init(boot_info: &BootInfo) {
    unsafe {
        *ROOT_PPN.get_mut() = boot_info.root_ppn;
        let limit_ppn = (boot_info.memory_size as usize / PAGE_SIZE) as u32;
        *PAGE_ALLOC.get_mut() = Some(PageAllocator::new(boot_info.next_free_ppn, limit_ppn));
    }
}

/// Allocate and zero a fresh L1 root page table. Returns None if out of frames.
pub fn alloc_root() -> Option<u32> {
    let alloc = unsafe { PAGE_ALLOC.get_mut() };
    match alloc {
        Some(alloc) => {
            let root = alloc.alloc()?;
            PageAllocator::zero_page(root);
            Some(root)
        }
        None => None,
    }
}

/// Ensure the page allocator will not hand out frames below `min_ppn`.
pub fn bump_page_allocator(min_ppn: u32) {
    unsafe {
        if let Some(alloc) = PAGE_ALLOC.get_mut() {
            alloc.bump_to(min_ppn);
        }
    }
}

pub fn total_ppn() -> Option<u32> {
    unsafe { PAGE_ALLOC.get_mut().as_ref().map(|alloc| alloc.limit_ppn()) }
}

/// Map a user-visible virtual range with the provided permissions into a specific root.
pub fn map_user_range_for_root(root_ppn: u32, va_start: u32, len: usize, perms: PagePerms) -> bool {
    if len == 0 {
        return true;
    }
    let alloc = unsafe { PAGE_ALLOC.get_mut() };
    match alloc {
        Some(alloc) => {
            let page_size = PAGE_SIZE;
            let start = align_down_local(va_start as usize, page_size);
            let end = match (va_start as usize).checked_add(len) {
                Some(v) => align_up_local(v, page_size),
                None => return false,
            };
            let page_count = (end - start) / page_size;
            let vpn1_start = ((start as u32) >> 22) & SV32_VPN_MASK;
            let vpn1_end = (((end - 1) as u32) >> 22) & SV32_VPN_MASK;
            let l2_tables = vpn1_end
                .checked_sub(vpn1_start)
                .map(|v| v as usize + 1)
                .unwrap_or(0);
            let needed = page_count + l2_tables;
            let available = alloc.remaining_ppn() as usize;
            if needed > available {
                panic!(
                    "map_user_range_for_root: out of physical memory (need {} pages, have {})",
                    needed, available
                );
            }
            let mapper = KernelMapper::new(alloc);
            map_allocating(&mapper, root_ppn, va_start, len, perms)
        }
        None => false,
    }
}

/// Map a user-visible virtual range with the provided permissions into the current root.
pub fn map_user_range(va_start: u32, len: usize, perms: PagePerms) -> bool {
    let root = unsafe { *ROOT_PPN.get_mut() };
    map_user_range_for_root(root, va_start, len, perms)
}

/// Map a kernel-only virtual range with the provided permissions into a specific root.
pub fn map_kernel_range_for_root(root_ppn: u32, va_start: u32, len: usize, perms: PagePerms) -> bool {
    let alloc = unsafe { PAGE_ALLOC.get_mut() };
    match alloc {
        Some(alloc) => {
            let mapper = KernelMapper::new(alloc);
            map_allocating(&mapper, root_ppn, va_start, len, perms)
        }
        None => false,
    }
}

/// Map a kernel-only virtual range with the provided permissions into the current root.
pub fn map_kernel_range(va_start: u32, len: usize, perms: PagePerms) -> bool {
    let root = unsafe { *ROOT_PPN.get_mut() };
    map_kernel_range_for_root(root, va_start, len, perms)
}

/// Map a VA range in `root_ppn` to an explicit physical range (no allocation).
pub fn map_physical_range_for_root(
    root_ppn: u32,
    va_start: u32,
    phys_start: u32,
    len: usize,
    perms: PagePerms,
) -> bool {
    let alloc = unsafe { PAGE_ALLOC.get_mut() };
    match alloc {
        Some(alloc) => {
            let mapper = KernelMapper::new(alloc);
            map_to_physical(&mapper, root_ppn, va_start, phys_start, len, perms)
        }
        None => false,
    }
}

/// Mirror a mapped user range from `user_root` into the current kernel root so the
/// kernel can execute the user program without switching satp.
pub fn mirror_user_range_into_kernel(user_root: u32, va_start: u32, len: usize, perms: PagePerms) -> bool {
    if len == 0 {
        return true;
    }
    let page_size = PAGE_SIZE;
    let start = align_down_local(va_start as usize, page_size) as u32;
    let end = match (va_start as usize).checked_add(len) {
        Some(v) => align_up_local(v, page_size) as u32,
        None => return false,
    };
    let kernel_root = current_root();
    let alloc = unsafe { PAGE_ALLOC.get_mut() };
    let mapper_alloc = match alloc {
        Some(a) => a,
        None => return false,
    };
    let mut va = start;
    while va < end {
        let phys = match translate_user_va(user_root, va) {
            Some(p) => p as u32,
            None => return false,
        };
        if !overwrite_map_page(kernel_root, va, phys, perms, mapper_alloc) {
            return false;
        }
        va = va.wrapping_add(page_size as u32);
    }
    true
}

/// Walk Sv32 to translate a VA in the given root to a physical address.
pub fn translate_user_va(root_ppn: u32, va: u32) -> Option<usize> {
    let vpn1 = (va >> 22) & SV32_VPN_MASK;
    let vpn0 = (va >> 12) & SV32_VPN_MASK;
    let offset = (va & 0xfff) as usize;

    let l1_base = (root_ppn as usize)
        .checked_mul(PAGE_SIZE)?;
    let l1_addr = l1_base + vpn1 as usize * core::mem::size_of::<u32>();
    let l1_pte = read_pte(l1_addr)?;
    if l1_pte & SV32_PTE_V == 0 || l1_pte & (SV32_PTE_R | SV32_PTE_W | SV32_PTE_X) != 0 {
        return None;
    }

    let l2_base = ((l1_pte >> 10) as usize)
        .checked_mul(PAGE_SIZE)?;
    let l2_addr = l2_base + vpn0 as usize * core::mem::size_of::<u32>();
    let l2_pte = read_pte(l2_addr)?;
    if l2_pte & SV32_PTE_V == 0 {
        return None;
    }

    let ppn = (l2_pte >> 10) as usize;
    ppn.checked_mul(PAGE_SIZE)?.checked_add(offset)
}

/// Peek a 32-bit value at a VA in a given root using the direct-map window.
pub fn peek_word(root_ppn: u32, va: u32) -> Option<u32> {
    let phys = translate_user_va(root_ppn, va)?;
    let va_ptr = direct_map_addr(phys)?;
    Some(unsafe { (va_ptr as *const u32).read_volatile() })
}

/// Copy data into a user VA range for a specific root using the direct-map window.
pub fn copy_into_user(root_ppn: u32, va_start: u32, data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }
    let mut remaining = data.len();
    let mut src_off = 0usize;
    let mut va = va_start;
    while remaining > 0 {
        let phys = match translate_user_va(root_ppn, va) {
            Some(p) => p,
            None => return false,
        };
        let page_off = (va as usize) & (PAGE_SIZE - 1);
        let to_copy = cmp::min(remaining, PAGE_SIZE - page_off);
        let dst = match direct_map_addr(phys) {
            Some(v) => v,
            None => return false,
        };
        unsafe {
            ptr::copy_nonoverlapping(
                data.as_ptr().add(src_off),
                dst as *mut u8,
                to_copy,
            );
        }
        remaining -= to_copy;
        src_off += to_copy;
        va = va.wrapping_add(to_copy as u32);
    }
    true
}

/// Sv32 page-table accessor that routes PTE traffic through the kernel's direct map.
struct KernelMapper<'a> {
    alloc: *mut PageAllocator,
    _marker: PhantomData<&'a mut PageAllocator>,
}

impl<'a> KernelMapper<'a> {
    fn new(alloc: &'a mut PageAllocator) -> Self {
        Self {
            alloc: alloc as *mut PageAllocator,
            _marker: PhantomData,
        }
    }
}

impl<'a> Sv32PageTable for KernelMapper<'a> {
    fn page_size(&self) -> usize {
        PAGE_SIZE
    }

    fn read_pte(&self, phys_addr: usize) -> Option<u32> {
        let va = direct_map_addr(phys_addr)?;
        Some(unsafe { (va as *const u32).read_volatile() })
    }

    fn write_pte(&self, phys_addr: usize, val: u32) {
        if let Some(va) = direct_map_addr(phys_addr) {
            unsafe { (va as *mut u32).write_volatile(val) };
        }
    }

    fn alloc_frame(&self) -> Option<u32> {
        let alloc = unsafe { &mut *self.alloc };
        alloc.alloc()
    }

    fn zero_frame(&self, ppn: u32) {
        PageAllocator::zero_page(ppn);
    }
}

fn direct_map_addr(phys: usize) -> Option<usize> {
    DIRECT_MAP_BASE.checked_add(phys)
}

fn read_pte(phys_addr: usize) -> Option<u32> {
    let va = direct_map_addr(phys_addr)?;
    Some(unsafe { (va as *const u32).read_volatile() })
}

fn write_pte(phys_addr: usize, val: u32) {
    if let Some(va) = direct_map_addr(phys_addr) {
        unsafe { (va as *mut u32).write_volatile(val) };
    }
}

const fn align_up_local(val: usize, align: usize) -> usize {
    (val + (align - 1)) & !(align - 1)
}

const fn align_down_local(val: usize, align: usize) -> usize {
    val & !(align - 1)
}

fn overwrite_map_page(
    root_ppn: u32,
    va: u32,
    phys_start: u32,
    perms: PagePerms,
    alloc: &mut PageAllocator,
) -> bool {
    let page_size = PAGE_SIZE;
    let vpn1 = (va >> 22) & SV32_VPN_MASK;
    let vpn0 = (va >> 12) & SV32_VPN_MASK;

    let root_base = (root_ppn as usize)
        .checked_mul(page_size)
        .unwrap();
    let l1_addr = root_base + vpn1 as usize * core::mem::size_of::<u32>();
    let mut l1_pte = read_pte(l1_addr).unwrap_or(0);
    if l1_pte & SV32_PTE_V == 0 {
        let l2 = match alloc.alloc() {
            Some(ppn) => ppn,
            None => return false,
        };
        PageAllocator::zero_page(l2);
        l1_pte = (l2 << 10) | SV32_PTE_V;
        write_pte(l1_addr, l1_pte);
    } else if l1_pte & (SV32_PTE_R | SV32_PTE_W | SV32_PTE_X) != 0 {
        return false;
    }

    let l2_base = ((l1_pte >> 10) as usize)
        .checked_mul(page_size)
        .unwrap();
    let l2_addr = l2_base + vpn0 as usize * core::mem::size_of::<u32>();

    let leaf_ppn = phys_start / page_size as u32;
    let mut flags = SV32_PTE_V;
    if perms.read {
        flags |= SV32_PTE_R;
    }
    if perms.write {
        flags |= SV32_PTE_W;
    }
    if perms.exec {
        flags |= SV32_PTE_X;
    }
    if perms.user {
        flags |= types::SV32_PTE_U;
    }
    let leaf = (leaf_ppn << 10) | flags;
    write_pte(l2_addr, leaf);
    true
}
