extern crate alloc;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use state::State;
use types::TransactionReceipt;
use types::transaction::TransactionBundle;

use crate::Task;
use crate::memory::heap::BumpAllocator;
use crate::memory::page_allocator::PageAllocator;

/// Minimal wrapper to store non-`Sync` types in statics.
///
/// Safety: Callers must guarantee exclusive access when mutating.
pub struct Global<T> {
    inner: UnsafeCell<T>,
}

impl<T> Global<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    /// # Safety
    /// Callers must ensure exclusive access or otherwise serialize mutations.
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl<T> Sync for Global<T> {}

/// Max number of task slots the kernel tracks at once.
pub const MAX_TASKS: usize = 16;
/// Reserved slot index for the kernel/supervisor task.
pub const KERNEL_TASK_SLOT: usize = 0;
/// Currently running task slot index (kernel or user).
pub static CURRENT_TASK: Global<usize> = Global::new(KERNEL_TASK_SLOT);
/// Index of the bundle transaction currently being executed.
pub static CURRENT_TX: Global<usize> = Global::new(0);
/// Task slot that most recently completed and returned to the kernel.
/// Used to attach the correct program result to the current receipt.
pub static LAST_COMPLETED_TASK: Global<Option<usize>> = Global::new(None);
/// Active receipts buffer being filled while processing a bundle.
pub static RECEIPTS: Global<Option<Vec<TransactionReceipt>>> = Global::new(None);
/// Currently decoded bundle, if any.
pub static BUNDLE: Global<Option<TransactionBundle>> = Global::new(None);

pub struct TaskList {
    len: usize,
    slots: MaybeUninit<[Task; MAX_TASKS]>,
}

impl TaskList {
    pub const fn new() -> Self {
        Self {
            len: 0,
            slots: MaybeUninit::uninit(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, task: Task) -> Result<&Task, Task> {
        if self.len >= MAX_TASKS {
            return Err(task);
        }
        let idx = self.len;
        unsafe {
            let base = self.slots.as_mut_ptr() as *mut Task;
            base.add(idx).write(task);
        }
        self.len += 1;
        Ok(unsafe { &*(self.slots.as_ptr() as *const Task).add(idx) })
    }

    pub fn get(&self, idx: usize) -> Option<&Task> {
        if idx < self.len {
            Some(unsafe { &*(self.slots.as_ptr() as *const Task).add(idx) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Task> {
        if idx < self.len {
            Some(unsafe { &mut *(self.slots.as_mut_ptr() as *mut Task).add(idx) })
        } else {
            None
        }
    }

    pub fn kernel_task(&self) -> Option<&Task> {
        self.get(KERNEL_TASK_SLOT)
    }

    pub fn set_at(&mut self, idx: usize, task: Task) -> Result<&Task, Task> {
        if idx >= MAX_TASKS {
            return Err(task);
        }
        if idx > self.len {
            return Err(task);
        }
        if idx < self.len {
            unsafe { ptr::drop_in_place((self.slots.as_mut_ptr() as *mut Task).add(idx)) };
        } else {
            self.len += 1;
        }
        unsafe {
            let base = self.slots.as_mut_ptr() as *mut Task;
            base.add(idx).write(task);
            Ok(&*base.add(idx))
        }
    }

    pub fn last(&self) -> Option<&Task> {
        if self.len == 0 {
            None
        } else {
            self.get(self.len - 1)
        }
    }
}

impl Drop for TaskList {
    fn drop(&mut self) {
        for idx in 0..self.len {
            unsafe {
                ptr::drop_in_place((self.slots.as_mut_ptr() as *mut Task).add(idx));
            }
        }
    }
}

#[allow(dead_code)]
pub static TASKS: Global<TaskList> = Global::new(TaskList::new());
pub static STATE: Global<Option<State>> = Global::new(None);
pub static NEXT_ASID: Global<u16> = Global::new(1);
pub static ROOT_PPN: Global<u32> = Global::new(0);
pub static PAGE_ALLOC: Global<Option<PageAllocator>> = Global::new(None);
pub static KERNEL_HEAP: Global<BumpAllocator> = Global::new(BumpAllocator::empty());
