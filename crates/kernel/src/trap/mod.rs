use core::arch::asm;
use program::{log, logf};

use crate::syscall;
use crate::task::TRAMPOLINE_VA;

mod save_trap_frame;
mod restore_trap_frame;

use restore_trap_frame::restore_trap_frame;
use save_trap_frame::save_trap_frame;

const SCAUSE_ECALL_FROM_U: usize = 8;
const SCAUSE_ECALL_FROM_S: usize = 9;
const SSTATUS_SPP: u32 = 1 << 8;
const REG_COUNT: usize = 32;
const TRAP_FRAME_WORDS: usize = REG_COUNT + 1; // regs + pc
const TRAP_FRAME_BYTES: i32 = (TRAP_FRAME_WORDS * 4) as i32;
const REG_A0: usize = 10;
const REG_A1: usize = 11;
const REG_A2: usize = 12;
const REG_A3: usize = 13;
const REG_A4: usize = 14;
const REG_A5: usize = 15;
const REG_A6: usize = 16;
const REG_A7: usize = 17;
const REG_PC: usize = 32;

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
            "call {handler} # run Rust trap handler",
            "j {restore} # restore regs and return",
            swap = sym swap_to_kernel_stack,
            save = sym save_trap_frame,
            handler = sym handle_trap,
            restore = sym restore_trap_frame,
            options(noreturn),
        );
    }
}

#[unsafe(naked)]
unsafe extern "C" fn swap_to_kernel_stack() -> ! {
    core::arch::naked_asm!(
        "csrrw sp, sscratch, sp",
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
pub extern "C" fn handle_trap(saved: *mut u32) {
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
            let ret = syscall::dispatch_syscall(call_id, args, caller_mode);
            regs[REG_A0] = ret; // a0 return value
            regs[REG_PC] = regs[REG_PC].wrapping_add(4); // Advance past ecall
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
