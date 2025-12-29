#[unsafe(naked)]
pub(super) unsafe extern "C" fn save_trap_frame() -> ! {
    core::arch::naked_asm!(
        // Save all GPRs + sepc (PC). User sp lives in sscratch after the swap.
        "sw zero, 0(sp)",    // x0
        "sw ra, 4(sp)",      // x1
        "sw t1, 24(sp)",     // x6 (save before clobber)
        "csrr t1, sscratch", // user sp
        "sw t1, 8(sp)",      // x2
        "sw gp, 12(sp)",     // x3
        "sw tp, 16(sp)",     // x4
        "sw t0, 20(sp)",     // x5 (user satp from trap trampoline)
        "sw t2, 28(sp)",     // x7
        "sw s0, 32(sp)",     // x8
        "sw s1, 36(sp)",     // x9
        "sw a0, 40(sp)",     // x10
        "sw a1, 44(sp)",     // x11
        "sw a2, 48(sp)",     // x12
        "sw a3, 52(sp)",     // x13
        "sw a4, 56(sp)",     // x14
        "sw a5, 60(sp)",     // x15
        "sw a6, 64(sp)",     // x16
        "sw a7, 68(sp)",     // x17
        "sw s2, 72(sp)",     // x18
        "sw s3, 76(sp)",     // x19
        "sw s4, 80(sp)",     // x20
        "sw s5, 84(sp)",     // x21
        "sw s6, 88(sp)",     // x22
        "sw s7, 92(sp)",     // x23
        "sw s8, 96(sp)",     // x24
        "sw s9, 100(sp)",    // x25
        "sw s10, 104(sp)",   // x26
        "sw s11, 108(sp)",   // x27
        "sw t3, 112(sp)",    // x28
        "sw t4, 116(sp)",    // x29
        "sw t5, 120(sp)",    // x30
        "sw t6, 124(sp)",    // x31
        "csrr t1, sepc",
        "sw t1, 128(sp)",    // pc
        "mv a0, sp", // return saved-area pointer in a0
        "ret",
    );
}
