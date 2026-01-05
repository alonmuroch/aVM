use core::arch::asm;
use clibc::{log, logf};
use types::result::{Result as VmResult, RESULT_DATA_SIZE};

use crate::global::{
    CURRENT_TASK, KERNEL_TASK_SLOT, LAST_COMPLETED_TASK, MAX_RESULT_SIZE, RESULT_ADDR, TASKS,
};
use crate::memory::page_allocator as mmu;
use crate::syscall;
use crate::syscall::alloc::alloc_in_task;
use crate::syscall::storage::read_user_bytes;
use crate::task::TRAMPOLINE_VA;
use crate::Task;

mod save_trap_frame;
mod restore_trap_frame;

use restore_trap_frame::restore_trap_frame;
use save_trap_frame::save_trap_frame;

const SCAUSE_ECALL_FROM_U: usize = 8;
const SCAUSE_ECALL_FROM_S: usize = 9;
const SCAUSE_BREAKPOINT: usize = 3;
const SSTATUS_SPP: u32 = 1 << 8;
const REG_COUNT: usize = 32;
const TRAP_FRAME_WORDS: usize = REG_COUNT + 1; // regs + pc
const TRAP_FRAME_BYTES: i32 = (TRAP_FRAME_WORDS * 4) as i32;
const REG_RA: usize = 1;
const REG_A0: usize = 10;
const REG_A1: usize = 11;
const REG_A2: usize = 12;
const REG_A3: usize = 13;
const REG_A4: usize = 14;
const REG_A5: usize = 15;
const REG_A6: usize = 16;
const REG_A7: usize = 17;
const REG_SP: usize = 2;
const REG_PC: usize = 32;

#[repr(C)]
struct TrapReturn {
    sp: u32,
    kind: u32,
}

/// Install the kernel trap vector and set up the kernel stack for traps.
pub fn init_trap_vector(kstack_top: u32) {
    // Seed sscratch with the kernel stack top so trap entry can swap sp with
    // sscratch and immediately land on a known-good kernel stack.
    logf!("init_trap_vector: kstack_top=0x%x", kstack_top);
    unsafe {
        asm!("csrw sscratch, {0}", in(reg) kstack_top);
        asm!("csrw stvec, {0}", in(reg) trap_entry as usize);
    }
}

/// Trap entry stub:
/// - Switch to the kernel stack via sscratch.
/// - Save sepc, ra, a0-a7, and t0 (user satp).
/// - Call into the Rust trap handler with a pointer to the saved area.
/// - Restore registers and return via the shared trampoline.
// #[unsafe(naked)]
pub unsafe extern "C" fn trap_entry() -> ! {
    unsafe {
        core::arch::asm!(
            "call {swap} # switch to kernel stack and reserve trap frame",
            "call {save} # save regs on kernel stack",
            "mv s0, a0 # preserve trap frame pointer across handle_trap",
            "call {ensure} # restore kernel root if we trapped from user",
            "mv a0, s0 # restore trap frame pointer for handle_trap",
            "call {handler} # run Rust trap handler",
            "mv a2, a1 # stash return kind",
            "mv a1, s0 # restore trap frame pointer for restore",
            "j {restore}",
            swap = sym swap_to_kernel_stack,
            save = sym save_trap_frame,
            ensure = sym ensure_kernel_root_for_trap,
            handler = sym handle_trap,
            restore = sym restore_trap_frame,
            options(noreturn),
        );
    }
}

#[unsafe(naked)]
unsafe extern "C" fn swap_to_kernel_stack() -> ! {
    core::arch::naked_asm!(
        // Swap sp with sscratch:
        // - On trap entry, sscratch holds the kernel stack top.
        // - After the swap, sp points at the kernel stack and the previous sp
        //   (user sp) is saved in sscratch for later restoration.
        "csrrw sp, sscratch, sp",
        // Reserve space for the trap frame on the kernel stack.
        "addi sp, sp, -{frame_bytes}",
        "ret",
        frame_bytes = const TRAP_FRAME_BYTES,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn return_from_trap() -> ! {
    core::arch::naked_asm!(
        "csrr t1, sstatus",
        "andi t1, t1, {spp}",
        "bnez t1, 1f",
        "li t1, {tramp}",
        "jr t1",
        "1:",
        "sret",
        tramp = const TRAMPOLINE_VA,
        spp = const SSTATUS_SPP,
    );
}

/// Rust-level trap handler. Receives a pointer to the saved register block
/// laid out as:
/// regs[0..32] = x0..x31, regs[32] = pc.
#[unsafe(no_mangle)]
pub extern "C" fn handle_trap(saved: *mut u32) -> TrapReturn {
    let regs = unsafe { core::slice::from_raw_parts_mut(saved, TRAP_FRAME_WORDS) };
    let scause = read_scause();
    let stval = read_stval();
    let sepc = regs[REG_PC];

    let is_interrupt = (scause >> 31) != 0;
    if is_interrupt {
        panic!(
            "unexpected interrupt trap: scause=0x{:x} stval=0x{:x} sepc=0x{:08x}",
            scause, stval, sepc
        );
    }

    let code = scause & 0xfff;
    let mut return_kind = if read_sstatus() & SSTATUS_SPP != 0 { 1 } else { 0 };
    let mut return_sp = regs[REG_SP];
    match code {
        SCAUSE_ECALL_FROM_U | SCAUSE_ECALL_FROM_S => {
            let args = [
                regs[REG_A1],
                regs[REG_A2],
                regs[REG_A3],
                regs[REG_A4],
                regs[REG_A5],
                regs[REG_A6],
            ];
            let call_id = regs[REG_A7];
            let caller_mode = if read_sstatus() & SSTATUS_SPP != 0 {
                syscall::CallerMode::Supervisor
            } else {
                syscall::CallerMode::User
            };
            let ret = {
                let mut ctx = syscall::SyscallContext { regs, caller_mode };
                syscall::dispatch_syscall(call_id, args, &mut ctx)
            };
            regs[REG_A0] = ret; // a0 return value
            regs[REG_PC] = regs[REG_PC].wrapping_add(4); // Advance past ecall
            return_kind = 0;
            return_sp = regs[REG_SP];
        }
        SCAUSE_BREAKPOINT => {
            // Default to returning to the kernel task unless the current task has a caller.
            let mut caller_idx = KERNEL_TASK_SLOT;
            let mut result_for_caller: Option<VmResult> = None;
            unsafe {
                let current = *CURRENT_TASK.get_mut();
                let tasks = TASKS.get_mut();
                // If this is a user task, save its current trapframe so it can be resumed later.
                if current != KERNEL_TASK_SLOT {
                    if let Some(task) = tasks.get_mut(current) {
                        if let Some(result) = read_task_result(task) {
                            task.last_result = Some(result);
                            log_task_result(&result);
                            result_for_caller = Some(result);
                        } else {
                            log!("program result: failed to read result bytes");
                        }
                        for (idx, value) in regs.iter().take(REG_COUNT).enumerate() {
                            task.tf.regs[idx] = *value;
                        }
                        task.tf.pc = regs[REG_PC];
                        // Use the recorded caller task as the return target.
                        caller_idx = task.caller_task_id.unwrap_or(KERNEL_TASK_SLOT);
                        if caller_idx == KERNEL_TASK_SLOT {
                            // Only record tasks that return to the kernel so bundle resume can
                            // associate the completed task with the current transaction receipt.
                            *LAST_COMPLETED_TASK.get_mut() = Some(current);
                        }
                    }
                }
                // Restore the caller task's trapframe and address-space root.
                if let Some(caller_task) = tasks.get_mut(caller_idx) {
                    if caller_idx != KERNEL_TASK_SLOT {
                        let result_ptr = match result_for_caller {
                            Some(result) => write_result_to_caller(caller_task, &result).unwrap_or(0),
                            None => 0,
                        };
                        caller_task.tf.regs[REG_A0] = result_ptr;
                    }
                    for (idx, value) in caller_task.tf.regs.iter().take(REG_COUNT).enumerate() {
                        regs[idx] = *value;
                    }
                    // Resume at the caller's return address.
                    regs[REG_PC] = if caller_idx == KERNEL_TASK_SLOT {
                        caller_task.tf.regs[REG_RA]
                    } else {
                        caller_task.tf.pc
                    };
                    mmu::set_current_root(caller_task.addr_space.root_ppn);
                    return_sp = caller_task.tf.regs[REG_SP];
                    logf!(
                        "breakpoint return: caller=%d pc=0x%x ra=0x%x sp=0x%x",
                        caller_idx as u32,
                        caller_task.tf.pc,
                        caller_task.tf.regs[REG_RA],
                        caller_task.tf.regs[REG_SP]
                    );
                } else {
                    panic!("breakpoint trap: caller task missing");
                }
                // Mark the caller as the current task after the handoff.
                *CURRENT_TASK.get_mut() = caller_idx;
            }
            let mut sstatus = read_sstatus();
            // Set SPP so sret returns to the correct privilege level.
            if caller_idx == KERNEL_TASK_SLOT {
                // Return to supervisor when the caller is the kernel task.
                sstatus |= SSTATUS_SPP;
                return_kind = 1;
            } else {
                // Clear SPP to return to user mode for user callers.
                sstatus &= !SSTATUS_SPP;
                return_kind = 0;
            }
            unsafe { asm!("csrw sstatus, {0}", in(reg) sstatus); }
        }
        _ => log!("unhandled trap"),
    }
    TrapReturn {
        sp: return_sp,
        kind: return_kind,
    }
}

#[unsafe(no_mangle)]
/// Restore the kernel address-space root for traps arriving from user mode.
extern "C" fn ensure_kernel_root_for_trap() {
    if read_sstatus() & SSTATUS_SPP != 0 {
        return;
    }
    let kernel_root = unsafe {
        TASKS
            .get_mut()
            .get(KERNEL_TASK_SLOT)
            .map(|task| task.addr_space.root_ppn)
            .unwrap_or_else(mmu::current_root)
    };
    mmu::set_current_root(kernel_root);
}

#[inline(always)]
fn read_scause() -> usize {
    let value: usize;
    unsafe { asm!("csrr {0}, scause", out(reg) value); }
    value
}

#[inline(always)]
fn read_sstatus() -> u32 {
    let value: u32;
    unsafe { asm!("csrr {0}, sstatus", out(reg) value); }
    value
}

fn read_task_result(task: &Task) -> Option<VmResult> {
    let result_bytes =
        read_user_bytes(task.addr_space.root_ppn, RESULT_ADDR, MAX_RESULT_SIZE)?;
    if result_bytes.len() < 9 {
        return None;
    }
    let success = result_bytes[0] != 0;
    let error_code = u32::from_le_bytes(result_bytes[1..5].try_into().ok()?);
    let data_len = u32::from_le_bytes(result_bytes[5..9].try_into().ok()?);
    let data_len = (data_len as usize).min(RESULT_DATA_SIZE);
    if result_bytes.len() < 9 + data_len {
        return None;
    }
    let mut data = [0u8; RESULT_DATA_SIZE];
    data[..data_len].copy_from_slice(&result_bytes[9..9 + data_len]);
    Some(VmResult {
        success,
        error_code,
        data_len: data_len as u32,
        data,
    })
}

fn log_task_result(result: &VmResult) {
    let data_len = (result.data_len as usize).min(RESULT_DATA_SIZE);
    logf!(
        "program result: success=%d error=%d data_len=%d",
        result.success as u32,
        result.error_code,
        data_len as u32
    );
    if data_len > 0 {
        log!("program result data: %b", &result.data[..data_len]);
    }
}

fn write_result_to_caller(caller_task: &mut Task, result: &VmResult) -> Option<u32> {
    let addr = alloc_in_task(caller_task, MAX_RESULT_SIZE as u32, 4)?;
    let mut buf = [0u8; MAX_RESULT_SIZE];
    buf[0] = result.success as u8;
    buf[1..5].copy_from_slice(&result.error_code.to_le_bytes());
    buf[5..9].copy_from_slice(&result.data_len.to_le_bytes());
    let data_len = (result.data_len as usize).min(RESULT_DATA_SIZE);
    if data_len > 0 {
        buf[9..9 + data_len].copy_from_slice(&result.data[..data_len]);
    }
    if !mmu::copy(caller_task.addr_space.root_ppn, addr, &buf) {
        return None;
    }
    Some(addr)
}

#[inline(always)]
fn read_stval() -> usize {
    let value: usize;
    unsafe { asm!("csrr {0}, stval", out(reg) value); }
    value
}
