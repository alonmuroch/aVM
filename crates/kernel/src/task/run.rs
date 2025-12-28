use crate::global::{CURRENT_TASK, KERNEL_TASK_SLOT, TASKS};
use crate::mmu;
use program::logf;

use super::{REG_A0, REG_A1, REG_A2, REG_A3, REG_RA, REG_SP, TRAMPOLINE_VA, TRAP_TRAMPOLINE_VA};

const SSTATUS_SPP: u32 = 1 << 8;

/// One-way context switch into a user task:
/// - Saves the current kernel frame into TASKS[0]
/// - Loads the task's satp/regs/pc and jumps to user code (no return path yet)
pub fn run_task(task_idx: usize) {
    let (target_root, asid, pc, sp, a0, a1, a2, a3) = unsafe {
        let tasks = TASKS.get_mut();
        let task = match tasks.get(task_idx) {
            Some(task) => task,
            None => {
                logf!("run_task: invalid task slot %d", task_idx as u32);
                return;
            }
        };
        (
            task.addr_space.root_ppn,
            task.addr_space.asid,
            task.tf.pc,
            task.tf.regs[REG_SP],
            task.tf.regs[REG_A0],
            task.tf.regs[REG_A1],
            task.tf.regs[REG_A2],
            task.tf.regs[REG_A3],
        )
    };
    unsafe {
        *CURRENT_TASK.get_mut() = task_idx;
    }
    let kernel_root = mmu::current_root();
    logf!(
        "run_task: switching satp 0x%x -> 0x%x asid=%d pc=0x%x sp=0x%x",
        kernel_root,
        target_root,
        asid as u32,
        pc,
        sp,
    );
    // Save the current kernel frame (SP/RA/PC) into the kernel task slot (index 0).
    let mut saved_sp: u32;
    let mut saved_ra: u32;
    let mut saved_pc: u32;
    unsafe {
        core::arch::asm!("mv {out}, sp", out = out(reg) saved_sp);
        core::arch::asm!("mv {out}, ra", out = out(reg) saved_ra);
        core::arch::asm!("auipc {out}, 0", out = out(reg) saved_pc);
    }
    logf!(
        "run_task: saved kernel frame sp=0x%x ra=0x%x pc=0x%x",
        saved_sp,
        saved_ra,
        saved_pc
    );
    // Stash the kernel context so a future return path could restore it.
    unsafe {
        let tasks = TASKS.get_mut();
        if let Some(kernel_task) = tasks.get_mut(KERNEL_TASK_SLOT) {
            kernel_task.addr_space.root_ppn = kernel_root;
            kernel_task.tf.regs[REG_SP] = saved_sp;
            kernel_task.tf.regs[REG_RA] = saved_ra;
            kernel_task.tf.pc = saved_pc;
        }
    }
    // Prepare to enter user mode via sret: set sepc and clear sstatus.SPP.
    let mut sstatus: u32;
    unsafe {
        core::arch::asm!("csrr {0}, sstatus", out(reg) sstatus);
    }
    sstatus &= !SSTATUS_SPP;
    unsafe {
        core::arch::asm!("csrw sstatus, {0}", in(reg) sstatus);
        core::arch::asm!("csrw sepc, {0}", in(reg) pc);
        core::arch::asm!("csrw stvec, {0}", in(reg) TRAP_TRAMPOLINE_VA);
    }
    // Update the helper's view of the current root before switching.
    mmu::set_current_root(target_root);
    // Set up registers and jump to the shared trampoline page (mapped in both
    // the kernel and user roots). The trampoline will write satp and transfer
    // control to the user PC.
    unsafe {
        core::arch::asm!(
            "mv ra, zero",
            "mv sp, t2",
            "jr t3",
            in("t0") target_root,
            in("a0") a0,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("t2") sp,
            in("t3") TRAMPOLINE_VA,
            options(noreturn)
        );
    }
}
