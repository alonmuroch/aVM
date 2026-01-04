use crate::global::{KERNEL_TASK_SLOT, TASKS};
use crate::memory::page_allocator as mmu;

use super::{
    PAGE_SIZE, TRAMPOLINE_CODE, TRAMPOLINE_VA, TRAP_TRAMPOLINE_OFFSET,
};

const REG_T0: u32 = 5;
const REG_T1: u32 = 6;
const REG_T2: u32 = 7;
const TRAP_TRAMPOLINE_WORDS: usize = 7; // csrr + 2x(hi/lo) + csrw + jalr

fn split_imm(val: u32) -> (u32, i32) {
    // Build a LUI/ADDI pair for a full 32-bit immediate.
    let hi = ((val as u64 + 0x800) >> 12) as u32;
    let lo = val as i64 - ((hi as i64) << 12);
    (hi, lo as i32)
}

fn encode_lui(rd: u32, imm20: u32) -> u32 {
    (imm20 << 12) | (rd << 7) | 0x37
}

fn encode_addi(rd: u32, rs1: u32, imm12: i32) -> u32 {
    ((imm12 as u32 & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | 0x13
}

fn encode_jalr(rd: u32, rs1: u32, imm12: i32) -> u32 {
    ((imm12 as u32 & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | 0x67
}

fn encode_csrr(rd: u32, csr: u32) -> u32 {
    (csr << 20) | (0 << 15) | (0b010 << 12) | (rd << 7) | 0x73
}

/// Build the trap-entry trampoline instructions.
///
/// This stub runs at `TRAP_TRAMPOLINE_VA` while still in the user address space.
/// It saves the current user `satp` into `t0`, switches to the kernel root page
/// table, and jumps to the real kernel trap handler at `trap_entry`.
fn build_trap_trampoline(kernel_satp: u32, trap_entry: u32) -> [u32; TRAP_TRAMPOLINE_WORDS] {
    // Assemble the kernel satp and trap_entry as LUI/ADDI pairs.
    let (satp_hi, satp_lo) = split_imm(kernel_satp);
    let (entry_hi, entry_lo) = split_imm(trap_entry);
    [
        encode_csrr(REG_T0, 0x180), // csrr t0, satp: save user satp so kernel can restore later.
        encode_lui(REG_T1, satp_hi), // lui t1, %hi(kernel_satp): load upper bits.
        encode_addi(REG_T1, REG_T1, satp_lo), // addi t1, t1, %lo(kernel_satp): finish satp.
        0x1803_1073, // csrw satp, t1: switch to kernel page table.
        encode_lui(REG_T2, entry_hi), // lui t2, %hi(trap_entry): load trap handler addr.
        encode_addi(REG_T2, REG_T2, entry_lo), // addi t2, t2, %lo(trap_entry).
        encode_jalr(0, REG_T2, 0), // jalr x0, t2, 0: jump to trap handler.
    ]
}

pub(super) fn map_trampoline_page(root_ppn: u32) {
    // Install a small trampoline page mapped in both roots so we can switch
    // satp safely before jumping into the user program.
    let kernel_tramp_perms = mmu::PagePerms::kernel_rwx();
    let user_tramp_perms = mmu::PagePerms::new(true, false, true, true);
    let kernel_root = unsafe {
        TASKS
            .get_mut()
            .get(KERNEL_TASK_SLOT)
            .map(|task| task.addr_space.root_ppn)
            .unwrap_or_else(mmu::current_root)
    };
    let trap_entry = crate::trap::trap_entry as usize as u32;
    let trap_trampoline = build_trap_trampoline(kernel_root, trap_entry);
    // Stash both trampolines in a single shared page.
    let mut tramp_bytes =
        [0u8; TRAP_TRAMPOLINE_OFFSET + TRAP_TRAMPOLINE_WORDS * 4];
    for (i, word) in TRAMPOLINE_CODE.iter().enumerate() {
        tramp_bytes[i * 4..(i + 1) * 4].copy_from_slice(&word.to_le_bytes());
    }
    for (i, word) in trap_trampoline.iter().enumerate() {
        // Trap stub lives at TRAP_TRAMPOLINE_OFFSET for stvec to target.
        let base = TRAP_TRAMPOLINE_OFFSET + i * 4;
        tramp_bytes[base..base + 4].copy_from_slice(&word.to_le_bytes());
    }
    if !mmu::map_range_for_root(
        kernel_root,
        TRAMPOLINE_VA,
        PAGE_SIZE,
        kernel_tramp_perms,
    ) {
        panic!("prep_program_task: failed to map trampoline page in kernel root");
    }
    if !mmu::copy(kernel_root, TRAMPOLINE_VA, &tramp_bytes) {
        panic!("prep_program_task: failed to populate trampoline code");
    }
    let tramp_phys = match mmu::translate(kernel_root, TRAMPOLINE_VA) {
        Some(p) => p as u32,
        None => {
            panic!("prep_program_task: trampoline VA not mapped in kernel root");
        }
    };
    if !mmu::map_physical_range_for_root(
        root_ppn,
        TRAMPOLINE_VA,
        tramp_phys,
        PAGE_SIZE,
        user_tramp_perms,
    ) {
        panic!("prep_program_task: failed to map trampoline page in user root");
    }
}
