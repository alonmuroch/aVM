use clibc::log;
use kernel::global::{CURRENT_TX, RECEIPTS, STATE};
use state::State;
use types::Result;
use types::transaction::Transaction;

const TRANSFER_ERROR: u32 = 1;

pub(crate) fn transfer(tx: &Transaction) {
    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    let ok = state.transfer(&tx.from, &tx.to, tx.value);
    if !ok {
        log!("transfer failed");
        set_receipt(false, TRANSFER_ERROR);
    }
}

fn set_receipt(success: bool, error_code: u32) {
    let tx_idx = unsafe { *CURRENT_TX.get_mut() };
    unsafe {
        if let Some(receipts) = RECEIPTS.get_mut().as_mut()
            && let Some(receipt) = receipts.get_mut(tx_idx)
        {
            receipt.result = Result::new(success, error_code);
        }
    }
}
