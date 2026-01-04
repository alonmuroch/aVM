use crate::{AddressSpace, Task};
use crate::global::{
    CALL_ARGS_PAGE_BASE, CODE_SIZE_LIMIT, CURRENT_TASK, FROM_PTR_ADDR, HEAP_START_ADDR,
    INPUT_BASE_ADDR, MAX_INPUT_LEN, RO_DATA_SIZE_LIMIT, TO_PTR_ADDR,
};
use crate::memory::page_allocator as mmu;
use clibc::{log, logf};
use types::address::Address;
use types::SV32_PAGE_SIZE;

use super::{
    alloc_asid, trampoline::map_trampoline_page, PROGRAM_VA_BASE, PROGRAM_WINDOW_BYTES, REG_A0,
    REG_A1, REG_A2, REG_A3, REG_SP, STACK_BYTES,
};

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
    if input.len() > MAX_INPUT_LEN {
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
    let args_perms = mmu::PagePerms::new(true, false, false, true);
    if !mmu::map_range_for_root(root_ppn, CALL_ARGS_PAGE_BASE, SV32_PAGE_SIZE, args_perms) {
        panic!(
            "launch_program: failed to map call-args page (root=0x{:x})",
            root_ppn
        );
    }
    map_program_window(root_ppn, code.len());

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

    if !mmu::copy(root_ppn, PROGRAM_VA_BASE, code) {
        logf!("launch_program: failed to copy code into root=0x%x", root_ppn);
        return None;
    }

    if !mmu::copy(root_ppn, TO_PTR_ADDR, &to.0) {
        logf!("launch_program: failed to copy 'to' address into root=0x%x", root_ppn);
        return None;
    }
    if !mmu::copy(root_ppn, FROM_PTR_ADDR, &from.0) {
        logf!("launch_program: failed to copy 'from' address into root=0x%x", root_ppn);
        return None;
    }
    if !mmu::copy(root_ppn, INPUT_BASE_ADDR, input) {
        panic!(
            "prep_program_task: failed to copy input into root=0x{:x}",
            root_ppn
        );
    }

    // Sanity check where the code landed in the user root.
    let entry_va = PROGRAM_VA_BASE.wrapping_add(entry_off);
    let user_phys = mmu::translate(root_ppn, entry_va).unwrap_or(usize::MAX);
    let user_word = mmu::peek_word(root_ppn, entry_va).unwrap_or(0);
    logf!(
        "prep_program_task: code VA=0x%x user_phys=0x%x user_word=0x%x code_start=0x%x",
        entry_va,
        user_phys as u32,
        user_word,
        entry_off
    );
    map_trampoline_page(root_ppn);

    let mut task = Task::new(
        AddressSpace::new(
            root_ppn,
            asid,
            PROGRAM_VA_BASE,
            PROGRAM_WINDOW_BYTES as u32,
        ),
        HEAP_START_ADDR as u32,
    );
    let caller = unsafe { *CURRENT_TASK.get_mut() };
    task.caller_task_id = Some(caller);
    // Set up initial trapframe.
    let stack_top = PROGRAM_VA_BASE.wrapping_add(PROGRAM_WINDOW_BYTES as u32);
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
        HEAP_START_ADDR as u32
    );

    Some(task)
}

fn align_up(value: usize, align: usize) -> usize {
    if align == 0 {
        return value;
    }
    (value + (align - 1)) & !(align - 1)
}

/// Map the program window so code pages are RX and data/stack/heap are RW.
/// The first page stays RWX because the program writes its result at 0x100.
fn map_program_window(root_ppn: u32, code_len: usize) {
    let code_len = align_up(code_len, SV32_PAGE_SIZE);
    if code_len > PROGRAM_WINDOW_BYTES {
        panic!("launch_program: code window exceeds program window");
    }
    let first_page_len = core::cmp::min(code_len, SV32_PAGE_SIZE);
    let first_page_perms = mmu::PagePerms::user_rwx();
    // Page 0 hosts the result header at 0x100, so keep it writable.
    if !mmu::map_range_for_root(root_ppn, PROGRAM_VA_BASE, first_page_len, first_page_perms) {
        panic!("launch_program: first page mapping failed (root=0x{:x})", root_ppn);
    }
    if code_len > SV32_PAGE_SIZE {
        let code_perms = mmu::PagePerms::new(true, false, true, true);
        let code_start = PROGRAM_VA_BASE.wrapping_add(SV32_PAGE_SIZE as u32);
        let code_rest = code_len.saturating_sub(SV32_PAGE_SIZE);
        // Remaining code pages are RX-only to protect program text.
        if !mmu::map_range_for_root(root_ppn, code_start, code_rest, code_perms) {
            panic!("launch_program: code mapping failed (root=0x{:x})", root_ppn);
        }
    }
    let data_start = PROGRAM_VA_BASE.wrapping_add(code_len as u32);
    let data_len = PROGRAM_WINDOW_BYTES.saturating_sub(code_len);
    let data_perms = mmu::PagePerms::new(true, true, false, true);
    // Data/stack/heap region is RW, non-exec.
    if !mmu::map_range_for_root(root_ppn, data_start, data_len, data_perms) {
        panic!("launch_program: data mapping failed (root=0x{:x})", root_ppn);
    }
}
