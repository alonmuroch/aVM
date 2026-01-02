extern crate alloc;

use alloc::format;
use clibc::logf;
use state::State;
use types::address::Address;

use crate::global::{CODE_SIZE_LIMIT, RO_DATA_SIZE_LIMIT, STATE};

pub struct ProgramImage<'a> {
    pub code: &'a [u8],
    pub entry_off: u32,
}

// Load a program image from STATE, validate it, and pass a borrowed view to a caller.
// This centralizes code lookup, contract checks, and entry offset calculation.
pub fn with_program_image<R>(
    to: &Address,
    f: impl FnOnce(ProgramImage<'_>) -> Option<R>,
) -> Option<R> {
    // Fetch the account from state (or log and bail if it is missing).
    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    let account = match state.get_account(to) {
        Some(acc) => acc,
        None => {
            logf!(
                "%s",
                display: format!("Program call failed: account {} does not exist", to)
            );
            return None;
        }
    };

    // Ensure the target is a contract; non-contract accounts cannot be executed.
    if !account.is_contract {
        logf!(
            "%s",
            display: format!(
                "Program call failed: target {} is not a contract (code_len={})",
                to,
                account.code.len()
            )
        );
        return None;
    }

    // Find the first non-zero byte to infer the entry offset and log code stats.
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

    // Enforce the code size limit to prevent oversized binaries.
    let code_len = account.code.len();
    let max = CODE_SIZE_LIMIT + RO_DATA_SIZE_LIMIT;
    if code_len > max {
        panic!(
            "❌ Program call rejected: code size ({}) exceeds limit ({})",
            code_len, max
        );
    }

    // Provide the borrowed code slice and entry offset to the caller.
    let entry_off = first_nz as u32;
    f(ProgramImage {
        code: &account.code,
        entry_off,
    })
}
