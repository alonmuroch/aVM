use crate::{AddressSpace, Config, Task};
use crate::global::CURRENT_TASK;
use crate::memory::page_allocator as mmu;
use program::{log, logf};
use types::address::Address;

use super::{
    alloc_asid, FROM_PTR_ADDR, HEAP_BYTES, INPUT_BASE_ADDR, PAGE_SIZE, PROGRAM_VA_BASE,
    PROGRAM_WINDOW_BYTES, REG_A0, REG_A1, REG_A2, REG_A3, REG_SP, STACK_BYTES,
    TO_PTR_ADDR, TRAMPOLINE_CODE, TRAMPOLINE_VA, TRAP_TRAMPOLINE_OFFSET,
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

fn build_trap_trampoline(kernel_satp: u32, trap_entry: u32) -> [u32; TRAP_TRAMPOLINE_WORDS] {
    // Trampoline swaps to the kernel root and jumps to trap_entry, preserving user satp in t0.
    let (satp_hi, satp_lo) = split_imm(kernel_satp);
    let (entry_hi, entry_lo) = split_imm(trap_entry);
    [
        encode_csrr(REG_T0, 0x180), // csrr t0, satp
        encode_lui(REG_T1, satp_hi),
        encode_addi(REG_T1, REG_T1, satp_lo),
        0x1803_1073, // csrw satp, t1
        encode_lui(REG_T2, entry_hi),
        encode_addi(REG_T2, REG_T2, entry_lo),
        encode_jalr(0, REG_T2, 0), // jr t2
    ]
}

/// Create a new task for a program and map its virtual address window via syscalls.
///
/// This sets up:
/// - Maps a fixed VA window [PROGRAM_VA_BASE, PROGRAM_VA_BASE + PROGRAM_WINDOW_BYTES).
/// - Returns a Task with the new address space.
///
/// The caller is responsible for copying program bytes into the mapped window
/// and initializing the user trapframe (PC/SP/args) before running.
pub fn prep_program_task(
    to: &Address,
    from: &Address,
    code: &[u8],
    input: &[u8],
    entry_off: u32,
) -> Option<Task> {
    if input.len() > Config::MAX_INPUT_LEN {
        log!("launch_program: input too large");
        return None;
    }

    let asid = alloc_asid();
    let root_ppn = match mmu::alloc_root() {
        Some(ppn) => ppn,
        None => {
            logf!("launch_program: no free root PPN available");
            return None;
        }
    };

    let window_end = PROGRAM_VA_BASE.wrapping_add(PROGRAM_WINDOW_BYTES as u32);
    logf!(
        "launch_program: asid=%d root=0x%x map=[0x%x,0x%x)",
        asid as u32,
        root_ppn,
        PROGRAM_VA_BASE,
        window_end
    );
    let perms = mmu::PagePerms::user_rwx();
    if !mmu::map_user_range_for_root(root_ppn, PROGRAM_VA_BASE, PROGRAM_WINDOW_BYTES, perms) {
        panic!("launch_program: mapping failed (root=0x{:x})", root_ppn);
    }

    // Copy the full program image starting at VA 0 so section offsets (e.g. .text at 0x400)
    // land where the ELF expected them. Entry offset is provided by the caller.
    if entry_off as usize >= code.len() {
        panic!("launch_program: invalid entry offset");
    }
    if code.len() >= entry_off as usize + 8 {
        let head = u32::from_le_bytes([
            code[entry_off as usize],
            code[entry_off as usize + 1],
            code[entry_off as usize + 2],
            code[entry_off as usize + 3],
        ]);
        let head2 = u32::from_le_bytes([
            code[entry_off as usize + 4],
            code[entry_off as usize + 5],
            code[entry_off as usize + 6],
            code[entry_off as usize + 7],
        ]);
    }
    let nz_count = code.iter().filter(|&&b| b != 0).count();
    let local_first_nz = code.iter().position(|&b| b != 0).unwrap_or(code.len());

    if !mmu::copy_into_user(root_ppn, PROGRAM_VA_BASE, code) {
        logf!("launch_program: failed to copy code into root=0x%x", root_ppn);
        return None;
    }

    if !mmu::copy_into_user(root_ppn, TO_PTR_ADDR, &to.0) {
        logf!("launch_program: failed to copy 'to' address into root=0x%x", root_ppn);
        return None;
    }
    if !mmu::copy_into_user(root_ppn, FROM_PTR_ADDR, &from.0) {
        logf!("launch_program: failed to copy 'from' address into root=0x%x", root_ppn);
        return None;
    }
    if !mmu::copy_into_user(root_ppn, INPUT_BASE_ADDR, input) {
        panic!(
            "prep_program_task: failed to copy input into root=0x{:x}",
            root_ppn
        );
    }

    // Sanity check where the code landed in the user root.
    let entry_va = PROGRAM_VA_BASE.wrapping_add(entry_off);
    let user_phys = mmu::translate_user_va(root_ppn, entry_va).unwrap_or(usize::MAX);
    let user_word = mmu::peek_word(root_ppn, entry_va).unwrap_or(0);
    logf!(
        "prep_program_task: code VA=0x%x user_phys=0x%x user_word=0x%x code_start=0x%x",
        entry_va,
        user_phys as u32,
        user_word,
        entry_off
    );
    // Install a small trampoline page mapped in both roots so we can switch
    // satp safely before jumping into the user program.
    let tramp_perms = mmu::PagePerms::user_rwx();
    if !mmu::map_user_range_for_root(root_ppn, TRAMPOLINE_VA, PAGE_SIZE, tramp_perms) {
        panic!("prep_program_task: failed to map trampoline page in user root");
    }
    let _tramp_phys = match mmu::translate_user_va(root_ppn, TRAMPOLINE_VA) {
        Some(p) => p as u32,
        None => panic!("prep_program_task: trampoline VA not mapped"),
    };
    if !mmu::mirror_user_range_into_kernel(root_ppn, TRAMPOLINE_VA, PAGE_SIZE, tramp_perms) {
        panic!("prep_program_task: failed to mirror trampoline into kernel root");
    }
    let kernel_root = mmu::current_root();
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
    if !mmu::copy_into_user(root_ppn, TRAMPOLINE_VA, &tramp_bytes) {
        panic!("prep_program_task: failed to populate trampoline code");
    }

    let mut task = Task::new(
        AddressSpace::new(
            root_ppn,
            asid,
            PROGRAM_VA_BASE,
            PROGRAM_WINDOW_BYTES as u32,
        ),
        Config::HEAP_START_ADDR as u32,
    );
    let caller = unsafe { *CURRENT_TASK.get_mut() };
    task.caller_task_id = Some(caller);
    // Set up initial trapframe.
    let stack_top = PROGRAM_VA_BASE
        .wrapping_add((Config::CODE_SIZE_LIMIT + Config::RO_DATA_SIZE_LIMIT + STACK_BYTES) as u32);
    task.tf.pc = entry_va;
    task.tf.regs[REG_SP] = stack_top;
    task.tf.regs[REG_A0] = TO_PTR_ADDR;
    task.tf.regs[REG_A1] = FROM_PTR_ADDR;
    task.tf.regs[REG_A2] = INPUT_BASE_ADDR;
    task.tf.regs[REG_A3] = input.len() as u32;
    logf!(
        "prep_program_task: trapframe pc=0x%x sp=0x%x a0=0x%x a1=0x%x a2=0x%x a3=%d",
        task.tf.pc,
        task.tf.regs[REG_SP],
        task.tf.regs[REG_A0],
        task.tf.regs[REG_A1],
        task.tf.regs[REG_A2],
        task.tf.regs[REG_A3],
    );
    // Also log the expected user stack window for sanity.
    let stack_base = stack_top.saturating_sub(STACK_BYTES as u32);
    logf!(
        "prep_program_task: stack window=[0x%x,0x%x) heap_base=0x%x",
        stack_base,
        stack_top,
        Config::HEAP_START_ADDR as u32
    );

    Some(task)
}
