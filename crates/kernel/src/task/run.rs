use crate::global::{CURRENT_TASK, KERNEL_TASK_SLOT, TASKS};
use crate::memory::page_allocator as mmu;
use clibc::logf;

use super::{REG_A0, REG_A1, REG_A2, REG_A3, REG_SP, TRAMPOLINE_VA, TRAP_TRAMPOLINE_VA};

const SSTATUS_SPP: u32 = 1 << 8;
const REG_COUNT: usize = 32;
const REG_PC: usize = 32;
const TRAP_FRAME_WORDS: usize = REG_COUNT + 1; // regs + pc
const TRAP_FRAME_BYTES: i32 = (TRAP_FRAME_WORDS * 4) as i32;
const REG_RA: usize = 1;

/// One-way context switch into a user task:
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
    unsafe {
        if let Some(task) = TASKS.get_mut().get(task_idx) {
            if let Some(caller_idx) = task.caller_task_id {
                if let Some(caller_task) = TASKS.get_mut().get(caller_idx) {
                    logf!(
                        "run_task: return ra=0x%x sp=0x%x for caller %d",
                        caller_task.tf.regs[REG_RA],
                        caller_task.tf.regs[REG_SP],
                        caller_idx as u32
                    );
                }
            }
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
            "mv ra, zero # clear return address for one-way jump",
            "mv sp, t2 # load user stack pointer",
            "jr t3 # jump to shared trampoline",
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

/// Save the full kernel register set into TASKS[0] and then run the task.
#[unsafe(naked)]
pub unsafe extern "C" fn kernel_run_task(task_idx: usize) -> ! {
    core::arch::naked_asm!(
        "addi sp, sp, -{frame_bytes} # reserve space for regs + pc",
        "sw zero, 0(sp) # save x0",
        "sw ra, 4(sp) # save x1",
        "sw t1, 24(sp) # save x6 before clobber",
        "addi t1, sp, {frame_bytes} # compute original sp",
        "sw t1, 8(sp) # save x2 (original sp)",
        "sw gp, 12(sp) # save x3",
        "sw tp, 16(sp) # save x4",
        "sw t0, 20(sp) # save x5",
        "sw t2, 28(sp) # save x7",
        "sw s0, 32(sp) # save x8",
        "sw s1, 36(sp) # save x9",
        "sw a0, 40(sp) # save x10",
        "sw a1, 44(sp) # save x11",
        "sw a2, 48(sp) # save x12",
        "sw a3, 52(sp) # save x13",
        "sw a4, 56(sp) # save x14",
        "sw a5, 60(sp) # save x15",
        "sw a6, 64(sp) # save x16",
        "sw a7, 68(sp) # save x17",
        "sw s2, 72(sp) # save x18",
        "sw s3, 76(sp) # save x19",
        "sw s4, 80(sp) # save x20",
        "sw s5, 84(sp) # save x21",
        "sw s6, 88(sp) # save x22",
        "sw s7, 92(sp) # save x23",
        "sw s8, 96(sp) # save x24",
        "sw s9, 100(sp) # save x25",
        "sw s10, 104(sp) # save x26",
        "sw s11, 108(sp) # save x27",
        "sw t3, 112(sp) # save x28",
        "sw t4, 116(sp) # save x29",
        "sw t5, 120(sp) # save x30",
        "sw t6, 124(sp) # save x31",
        "auipc t1, 0 # read current pc",
        "sw t1, 128(sp) # save pc",
        "mv a1, a0 # move task_idx into a1",
        "mv a0, sp # pass saved regs pointer in a0",
        "call {helper} # save into kernel task and run",
        frame_bytes = const TRAP_FRAME_BYTES,
        helper = sym kernel_run_task_inner,
    );
}

/// Save the kernel trapframe into TASKS[0] and then jump into the requested task.
extern "C" fn kernel_run_task_inner(saved: *const u32, task_idx: usize) -> ! {
    // Interpret the saved trap-frame as regs[0..31] + pc and copy it into TASKS[0].
    let regs = unsafe { core::slice::from_raw_parts(saved, TRAP_FRAME_WORDS) };
    let kernel_root = mmu::current_root();
    unsafe {
        let tasks = TASKS.get_mut();
        if let Some(kernel_task) = tasks.get_mut(KERNEL_TASK_SLOT) {
            kernel_task.addr_space.root_ppn = kernel_root;
            for (idx, value) in regs.iter().take(REG_COUNT).enumerate() {
                kernel_task.tf.regs[idx] = *value;
            }
            kernel_task.tf.pc = regs[REG_PC];
        }
    }
    run_task(task_idx);
    unsafe { core::hint::unreachable_unchecked() }
}
