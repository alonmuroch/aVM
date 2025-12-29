use super::{return_from_trap, TRAP_FRAME_BYTES};

#[unsafe(naked)]
pub(super) unsafe extern "C" fn restore_trap_frame() -> ! {
    core::arch::naked_asm!(
        // Restore sepc and registers.
        "lw t1, 128(sp)",
        "csrw sepc, t1",
        "lw t2, 8(sp)",
        "csrw sscratch, t2", // restore user sp for swap
        "lw ra, 4(sp)",
        "lw gp, 12(sp)",
        "lw tp, 16(sp)",
        "lw t0, 20(sp)",  // user satp from trap trampoline
        "lw s0, 32(sp)",
        "lw s1, 36(sp)",
        "lw a0, 40(sp)",
        "lw a1, 44(sp)",
        "lw a2, 48(sp)",
        "lw a3, 52(sp)",
        "lw a4, 56(sp)",
        "lw a5, 60(sp)",
        "lw a6, 64(sp)",
        "lw a7, 68(sp)",
        "lw s2, 72(sp)",
        "lw s3, 76(sp)",
        "lw s4, 80(sp)",
        "lw s5, 84(sp)",
        "lw s6, 88(sp)",
        "lw s7, 92(sp)",
        "lw s8, 96(sp)",
        "lw s9, 100(sp)",
        "lw s10, 104(sp)",
        "lw s11, 108(sp)",
        "lw t3, 112(sp)",
        "lw t4, 116(sp)",
        "lw t5, 120(sp)",
        "lw t6, 124(sp)",
        "lw t1, 24(sp)",
        "lw t2, 28(sp)",
        "addi sp, sp, {frame_bytes}",
        "csrrw sp, sscratch, sp",
        "j {return}",
        return = sym return_from_trap,
        frame_bytes = const TRAP_FRAME_BYTES,
    );
}
