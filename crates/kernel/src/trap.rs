use core::arch::asm;
use program::{log, logf};

use crate::syscall;
use crate::task::TRAMPOLINE_VA;

const SCAUSE_ECALL_FROM_U: usize = 8;
const SCAUSE_ECALL_FROM_S: usize = 9;
const SSTATUS_SPP: u32 = 1 << 8;
const SAVED_REG_COUNT: usize = 11;

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
            "call {save} # save regs on kernel stack",
            "call {handler} # run Rust trap handler",
            "j {restore} # restore regs and return",
            save = sym save_trap_frame,
            handler = sym handle_trap,
            restore = sym restore_trap_frame,
            options(noreturn),
        );
    }
}

#[unsafe(naked)]
unsafe extern "C" fn save_trap_frame() -> ! {
    core::arch::naked_asm!(
        // Switch to kernel stack and make room for saved registers.
        "csrrw sp, sscratch, sp",
        "addi sp, sp, -44",
        // Save caller-saved registers we clobber and sepc.
        "sw t0, 40(sp)", // t0 holds user satp from trap trampoline
        "csrr t0, sepc",
        "sw t0, 0(sp)", // saved sepc
        "sw ra, 4(sp)",
        "sw a0, 8(sp)",
        "sw a1, 12(sp)",
        "sw a2, 16(sp)",
        "sw a3, 20(sp)",
        "sw a4, 24(sp)",
        "sw a5, 28(sp)",
        "sw a6, 32(sp)",
        "sw a7, 36(sp)",
        "mv a0, sp", // return saved-area pointer in a0
        "ret",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn restore_trap_frame() -> ! {
    core::arch::naked_asm!(
        // Restore sepc and registers.
        "lw t1, 0(sp)",
        "csrw sepc, t1",
        "lw ra, 4(sp)",
        "lw a0, 8(sp)",
        "lw a1, 12(sp)",
        "lw a2, 16(sp)",
        "lw a3, 20(sp)",
        "lw a4, 24(sp)",
        "lw a5, 28(sp)",
        "lw a6, 32(sp)",
        "lw a7, 36(sp)",
        "lw t0, 40(sp)", // user satp from trap trampoline
        "addi sp, sp, 44",
        "csrrw sp, sscratch, sp",
        "j {return}",
        return = sym return_from_trap,
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
/// [0] sepc, [1] ra, [2] a0, [3] a1, [4] a2, [5] a3, [6] a4, [7] a5,
/// [8] a6, [9] a7, [10] t0 (user satp from trap trampoline).
#[unsafe(no_mangle)]
pub extern "C" fn handle_trap(saved: *mut u32) {
    let regs = unsafe { core::slice::from_raw_parts_mut(saved, SAVED_REG_COUNT) };
    let scause = read_scause();
    let stval = read_stval();
    let sepc = regs[0];
    let sp: u32;
    unsafe { asm!("mv {0}, sp", out(reg) sp); }
    
    let is_interrupt = (scause >> 31) != 0;
    if is_interrupt {
        panic!(
            "unexpected interrupt trap: scause=0x{:x} stval=0x{:x} sepc=0x{:08x}",
            scause, stval, sepc
        );
    }

    let code = scause & 0xfff;
    match code {
        SCAUSE_ECALL_FROM_U | SCAUSE_ECALL_FROM_S => {
            let args = [
                regs[3], // a1
                regs[4], // a2
                regs[5], // a3
                regs[6], // a4
                regs[7], // a5
                regs[8], // a6
            ];
            let call_id = regs[9]; // a7
            let caller_mode = if read_sstatus() & SSTATUS_SPP != 0 {
                syscall::CallerMode::Supervisor
            } else {
                syscall::CallerMode::User
            };
            let ret = syscall::dispatch_syscall(call_id, args, caller_mode);
            regs[2] = ret; // a0 return value
            regs[0] = regs[0].wrapping_add(4); // Advance past ecall
        }
        _ => log!("unhandled trap"),
    }
}

#[inline(always)]
fn read_scause() -> usize {
    let value: usize;
    unsafe { asm!("csrr {0}, scause", out(reg) value); }
    value
}

#[inline(always)]
fn read_satp() -> u32 {
    let value: u32;
    unsafe { asm!("csrr {0}, satp", out(reg) value); }
    value
}

#[inline(always)]
fn read_sstatus() -> u32 {
    let value: u32;
    unsafe { asm!("csrr {0}, sstatus", out(reg) value); }
    value
}

#[inline(always)]
fn read_stval() -> usize {
    let value: usize;
    unsafe { asm!("csrr {0}, stval", out(reg) value); }
    value
}
