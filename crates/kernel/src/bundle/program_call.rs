use kernel::{kernel_run_task, prep_program_task, PROGRAM_WINDOW_BYTES};
use kernel::global::TASKS;
use kernel::user_program::with_program_image;
use clibc::{log, logf};
use clibc::parser::HexCodec;
use types::transaction::Transaction;

pub(crate) fn program_call(tx: &Transaction, resume: extern "C" fn() -> !) {
    let mut from_buf = [0u8; 40];
    let mut to_buf = [0u8; 40];
    let from_hex = HexCodec::encode(tx.from.as_ref(), &mut from_buf);
    let to_hex = HexCodec::encode(tx.to.as_ref(), &mut to_buf);
    let task = with_program_image(&tx.to, |image| {
        logf!(
            "Program call: from=%s to=%s input_len=%d code_len=%d",
            from_hex.as_ptr() as u32,
            from_hex.len() as u32,
            to_hex.as_ptr() as u32,
            to_hex.len() as u32,
            tx.data.len() as u32,
            image.code.len() as u32
        );
        prep_program_task(&tx.to, &tx.from, image.code, &tx.data, image.entry_off)
    });

    if let Some(task) = task {
        logf!(
            "Program task created: root=0x%x asid=%d window_size=%d",
            task.addr_space.root_ppn,
            task.addr_space.asid as u32,
            PROGRAM_WINDOW_BYTES as u32
        );
        unsafe {
            let tasks_slot = TASKS.get_mut();
            if tasks_slot.push(task).is_err() {
                log!("program task list full; skipping run");
                return;
            }
            let current = tasks_slot.len().saturating_sub(1);
            core::arch::asm!(
                "mv ra, {resume}",
                "j {run}",
                run = sym kernel_run_task,
                resume = in(reg) resume as usize,
                in("a0") current,
                options(noreturn),
            );
        }
    } else {
        panic!("program_call: no memory manager installed; cannot create program task");
    }
}
