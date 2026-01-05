use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

#[derive(Clone, Copy)]
pub(crate) struct BumpAllocator {
    next: usize,
    end: usize,
}

impl BumpAllocator {
    pub(crate) const fn empty() -> Self {
        Self { next: 0, end: 0 }
    }

    fn init(&mut self, start: usize, end: usize) {
        self.next = start;
        self.end = end;
    }

    fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        if size == 0 || align == 0 || (align & (align - 1)) != 0 {
            return None;
        }
        let start = align_up(self.next, align)?;
        let end = start.checked_add(size)?;
        if end > self.end {
            return None;
        }
        self.next = end;
        Some(start as *mut u8)
    }
}

/// Initialize the kernel bump allocator using the bootloader-provided heap pointer
/// and the mapped kernel VA window.
pub fn init(heap_ptr: u32, va_base: u32, va_len: u32) {
    let start = heap_ptr as usize;
    let end = (va_base as usize).saturating_add(va_len as usize);
    unsafe {
        crate::global::KERNEL_HEAP.get_mut().init(start, end);
    }
}

/// Allocate a kernel buffer from the bump allocator.
///
/// Returns a kernel virtual address on success, or None on exhaustion/invalid args.
pub fn alloc(size: usize, align: usize) -> Option<*mut u8> {
    unsafe { crate::global::KERNEL_HEAP.get_mut().alloc(size, align) }
}

/// Deallocate a kernel buffer. Bump allocator does not reclaim memory yet.
pub fn dealloc(_ptr: *mut u8, _size: usize, _align: usize) {}

fn align_up(value: usize, align: usize) -> Option<usize> {
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

struct KernelAlloc;

unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout.size(), layout.align()).unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static KERNEL_ALLOC: KernelAlloc = KernelAlloc;

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!(
        "kernel alloc error: size={} align={}",
        layout.size(),
        layout.align()
    );
}
