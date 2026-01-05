use super::{TRAP_FRAME_BYTES, return_from_trap};

#[unsafe(naked)]
pub(super) unsafe extern "C" fn restore_trap_frame(
    return_sp: u32, /* used via a0 reg */
    frame_ptr: *const u32,
) -> ! {
    core::arch::naked_asm!(
        // Restore sepc and registers using the provided trap-frame base (a1).
        "lw t1, 128(a1)",
        "csrw sepc, t1",
        "beqz a2, 1f",
        "csrw sscratch, a0", // kernel return: keep kernel sp for subsequent traps
        "j 2f",
        "1:",
        "addi t1, a1, {frame_bytes}",
        "csrw sscratch, t1", // user return: stash kernel sp for the next trap
        "2:",
        "lw ra, 4(a1)",
        "lw gp, 12(a1)",
        "lw tp, 16(a1)",
        "lw t0, 20(a1)",  // user satp from trap trampoline
        "lw t1, 24(a1)",
        "lw t2, 28(a1)",
        "lw s0, 32(a1)",
        "lw s1, 36(a1)",
        "lw a2, 48(a1)",
        "lw a3, 52(a1)",
        "lw a4, 56(a1)",
        "lw a5, 60(a1)",
        "lw a6, 64(a1)",
        "lw a7, 68(a1)",
        "lw s2, 72(a1)",
        "lw s3, 76(a1)",
        "lw s4, 80(a1)",
        "lw s5, 84(a1)",
        "lw s6, 88(a1)",
        "lw s7, 92(a1)",
        "lw s8, 96(a1)",
        "lw s9, 100(a1)",
        "lw s10, 104(a1)",
        "lw s11, 108(a1)",
        "lw t3, 112(a1)",
        "lw t4, 116(a1)",
        "lw t5, 120(a1)",
        "lw t6, 124(a1)",
        "mv sp, a0", // restore caller-selected sp (user or kernel), not sscratch
        "lw a0, 40(a1)",
        "lw a1, 44(a1)",
        "j {return}",
        return = sym return_from_trap,
        frame_bytes = const TRAP_FRAME_BYTES,
    );
}
