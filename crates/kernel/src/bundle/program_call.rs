use alloc::format;

use kernel::{prep_program_task, run_task, Config, PROGRAM_WINDOW_BYTES};
use kernel::global::{STATE, TASKS};
use program::{log, logf};
use program::parser::HexCodec;
use state::State;
use types::transaction::Transaction;

pub(crate) fn program_call(tx: &Transaction) {
    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    let account = match state.get_account(&tx.to) {
        Some(acc) => acc,
        None => {
            logf!(
                "%s",
                display: format!("Program call failed: account {} does not exist", tx.to)
            );
            return;
        }
    };

    if !account.is_contract {
        logf!(
            "%s",
            display: format!(
                "Program call failed: target {} is not a contract (code_len={})",
                tx.to,
                account.code.len()
            )
        );
        return;
    }

    let first_nz = account
        .code
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(account.code.len());
    let nz_count = account.code.iter().filter(|&&b| b != 0).count();
    logf!(
        "%s",
        display: format!(
            "Program code stats: len={} first_nz={} nz_count={}",
            account.code.len(),
            first_nz,
            nz_count
        )
    );

    let code_len = account.code.len();
    let max = Config::CODE_SIZE_LIMIT + Config::RO_DATA_SIZE_LIMIT;
    if code_len > max {
        panic!(
            "❌ Program call rejected: code size ({}) exceeds limit ({})",
            code_len, max
        );
    }

    let mut from_buf = [0u8; 40];
    let mut to_buf = [0u8; 40];
    let from_hex = HexCodec::encode(tx.from.as_ref(), &mut from_buf);
    let to_hex = HexCodec::encode(tx.to.as_ref(), &mut to_buf);
    logf!(
        "Program call: from=%s to=%s input_len=%d code_len=%d",
        from_hex.as_ptr() as u32,
        from_hex.len() as u32,
        to_hex.as_ptr() as u32,
        to_hex.len() as u32,
        tx.data.len() as u32,
        code_len as u32
    );

    let entry_off = first_nz as u32;

    if let Some(task) =
        prep_program_task(&tx.to, &tx.from, &account.code, &tx.data, entry_off)
    {
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
            run_task(current);
        }
    } else {
        log!("Program call skipped: no memory manager installed");
    }
}
