extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

// System call numbers for memory allocation
const SYSCALL_ALLOC: u32 = 7;
const SYSCALL_DEALLOC: u32 = 8;

/// VM Global Allocator
/// 
/// This allocator uses system calls to request memory from the VM host.
/// It enables the use of `Vec`, `HashMap`, and other heap-allocated types
/// in guest programs.
pub struct VmAllocator;

unsafe impl GlobalAlloc for VmAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { syscall_alloc(layout.size(), layout.align()) }
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { syscall_dealloc(ptr, layout.size()); }
    }
}

/// RISC-V system call for memory allocation
#[cfg(target_arch = "riscv32")]
unsafe fn syscall_alloc(size: usize, align: usize) -> *mut u8 {
    unsafe {
        let mut result: usize;
        core::arch::asm!(
            "li a7, 7", // syscall_storage_read
            "ecall",
            in("a1") size, 
            in("a2") align,
            out("a0") result, 
        );
        
        result as *mut u8
    }
}

/// RISC-V system call for memory deallocation
#[cfg(target_arch = "riscv32")]
unsafe fn syscall_dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        core::arch::asm!(
            "li a7, 8", // syscall_storage_read
            "ecall",
            in("a1") ptr as usize, 
            in("a2") size,
            options(nostack, preserves_flags),
        );
    }
}

/// Mock system call for memory allocation (for testing on host architecture)
#[cfg(not(target_arch = "riscv32"))]
unsafe fn syscall_alloc(size: usize, align: usize) -> *mut u8 {
    unsafe {
        // For testing purposes, use the system allocator
        let layout = alloc::alloc::Layout::from_size_align(size, align).unwrap();
        alloc::alloc::alloc(layout)
    }
}

/// Mock system call for memory deallocation (for testing on host architecture)  
#[cfg(not(target_arch = "riscv32"))]
unsafe fn syscall_dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        // For testing purposes, use the system allocator
        let align = 8; // Assume default alignment
        let layout = alloc::alloc::Layout::from_size_align(size, align).unwrap();
        alloc::alloc::dealloc(ptr, layout);
    }
}

