use core::mem::forget;

use program::{log, logf};
use types::transaction::{Transaction, TransactionBundle, TransactionType};

use crate::create_account::create_account;
use crate::program_call::program_call;

pub(crate) fn process_bundle(encoded_bundle: &[u8]) {
    log!("processing transaction bundle");
    if let Some(bundle) = TransactionBundle::decode(encoded_bundle) {
        let count = bundle.transactions.len();
        logf!("decoded tx count=%d", count as u32);
        for i in 0..count {
            logf!("processing tx %d/%d", (i + 1) as u32, count as u32);
            if let Some(tx) = bundle.transactions.get(i) {
                execute_transaction(tx);
            } else {
                logf!("missing tx at index %d", i as u32);
            }
        }
        // Avoid drop-time teardown that can allocate/deallocate; we halt immediately.
        forget(bundle);
    } else {
        log!("bundle decode failed");
    }
}

fn execute_transaction(tx: &Transaction) {
    match tx.tx_type {
        TransactionType::CreateAccount => create_account(tx),
        TransactionType::ProgramCall => program_call(tx),
        _ => panic!("unsupported transaction type"),
    }
}
