#![allow(dead_code)]

// Program launch flow (kernel side)
// ---------------------------------
// Goals:
// - Create a fresh address space for each program call (new root PPN + ASID).
// - Map a fixed, contiguous user window starting at VA 0x0 that holds:
//     * Code/rodata (program bytes copied starting at VA 0x0; entry at `entry_off`)
//     * A user stack (STACK_BYTES)
//     * A user heap (HEAP_BYTES) with input placed at INPUT_BASE_ADDR
// - Copy call arguments (to/from addresses + input buffer) into that window.
// - Prepare a trapframe with PC/SP/args and transfer control to user code.
//
// Key pieces:
// - PROGRAM_WINDOW_BYTES covers code + rodata + stack + heap: a single map call per program.
// - TRAMPOLINE_VA is one page immediately after the user window, mapped into both
//   the kernel root and the new user root. It contains:
//     * an entry trampoline that switches satp and sret's into user mode
//     * a trap trampoline that switches satp back to the kernel root and jumps
//       to the real trap_entry
//   This keeps trap entry valid even when the current root is the user page table.
//
// prep_program_task(to, from, code, input, entry_off):
// 1) Allocate ASID and a fresh root PPN; map the user window with user_rwx perms.
// 2) Copy program code starting at VA 0 (so section offsets are preserved), copy args (to/from/input).
// 3) Map the trampoline page into the user root and mirror the same physical page
//    into the current kernel root; write trampoline code into it.
// 4) Build a Task with AddressSpace {root_ppn, asid} and set trapframe:
//       pc = PROGRAM_VA_BASE + entry_off
//       sp = top of user stack within the window
//       a0..a3 = to/from/input_base/input_len
//    Caller can push the task into TASKS for bookkeeping.
//
// kernel_run_task(task):
// - Save the current kernel register file (x0-x31 + pc) into TASKS[0].
// - Run the task (same behavior as run_task).
//
// run_task(task):
// - Preload t0 with the task root (satp value); load user sp and a0..a3; clear ra.
// - Set sepc to the user PC and clear sstatus.SPP so sret enters user mode.
// - Set stvec to the trap trampoline VA.
// - jr TRAMPOLINE_VA. The trampoline executes under the old root, writes satp
//   to the new root, and executes sret into user code. There is no return
//   path yet; this is a one-way handoff.
//
// Notes:
// - The window and trampoline VAs are low for simplicity; nothing here relocates.
// - We currently do not touch sstatus/mstatus or perform sfence.vma; add those
//   when modeling fuller privilege transitions.

use crate::Config;
use crate::global::NEXT_ASID;
use types::ADDRESS_LEN;

pub mod task;
pub mod prep;
pub mod run;

pub use task::{AddressSpace, Task, TrapFrame};
pub use prep::prep_program_task;
pub use run::{kernel_run_task, run_task};

const PAGE_SIZE: usize = 4096;
const STACK_BYTES: usize = 0x4000; // 16 KiB user stack
pub const HEAP_BYTES: usize = 0x8000; // 32 KiB user heap
pub const PROGRAM_VA_BASE: u32 = 0x0;
// Location of the page that hosts the satp-switch trampolines. Kept just past
// the user window so it does not collide with program text/stack/heap. This VA
// is mapped into both roots so satp can be switched without invalidating the
// instruction stream mid-flight.
pub const TRAMPOLINE_VA: u32 =
    (PROGRAM_VA_BASE as usize + PROGRAM_WINDOW_BYTES) as u32; // Shared page just past user window.
const TRAP_TRAMPOLINE_OFFSET: usize = 0x10; // Offset for the trap-entry stub within the page.
pub const TRAP_TRAMPOLINE_VA: u32 =
    TRAMPOLINE_VA + TRAP_TRAMPOLINE_OFFSET as u32; // stvec target for user-mode traps.
const fn align_up(val: usize, align: usize) -> usize {
    (val + (align - 1)) & !(align - 1)
}

/// Total mapped window for a program: code/rodata, stack, and heap.
pub const PROGRAM_WINDOW_BYTES: usize = align_up(
    Config::CODE_SIZE_LIMIT + Config::RO_DATA_SIZE_LIMIT + STACK_BYTES + HEAP_BYTES,
    PAGE_SIZE,
);

const REG_SP: usize = 2;
const REG_RA: usize = 1;
const REG_A0: usize = 10;
const REG_A1: usize = 11;
const REG_A2: usize = 12;
const REG_A3: usize = 13;
// Raw RISC-V words for the entry trampoline used to switch satp safely while
// executing from a page mapped in both the kernel and user roots. The kernel
// loads t0 = target satp before entering this stub so we can change roots
// and return to user mode at sepc without returning to unmapped kernel text.
// t0: target satp value.
const TRAMPOLINE_CODE: [u32; 2] = [
    0x1802_9073, // csrw satp, t0
    0x1020_0073, // sret
];

const TO_PTR_ADDR: u32 = 0x120;
const FROM_PTR_ADDR: u32 = TO_PTR_ADDR + ADDRESS_LEN as u32;
const INPUT_BASE_ADDR: u32 = Config::HEAP_START_ADDR as u32;

pub(super) fn alloc_asid() -> u16 {
    unsafe {
        let counter = NEXT_ASID.get_mut();
        let asid = if *counter == 0 { 1 } else { *counter };
        *counter = asid.wrapping_add(1);
        asid
    }
}
