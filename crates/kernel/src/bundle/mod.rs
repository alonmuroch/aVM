use core::mem::forget;

use program::{log, logf};
use types::transaction::{Transaction, TransactionBundle, TransactionType};

use kernel::global::Global;

mod create_account;
mod program_call;

use self::create_account::create_account;
use self::program_call::program_call;

static BUNDLE: Global<Option<TransactionBundle>> = Global::new(None);
static CURRNET_BUNDLE_TX: Global<usize> = Global::new(0);

pub(crate) fn decode_bundle(encoded_bundle: &[u8]) -> bool {
    log!("processing transaction bundle");
    if let Some(bundle) = TransactionBundle::decode(encoded_bundle) {
        let count = bundle.transactions.len();
        logf!("decoded tx count=%d", count as u32);
        unsafe {
            *BUNDLE.get_mut() = Some(bundle);
            *CURRNET_BUNDLE_TX.get_mut() = 0;
        }
        true
    } else {
        false
    }
}

pub(crate) fn process_bundle() {
    let (idx, count) = unsafe {
        let count = BUNDLE
            .get_mut()
            .as_ref()
            .map(|bundle| bundle.transactions.len())
            .unwrap_or(0);
        (*CURRNET_BUNDLE_TX.get_mut(), count)
    };
    if idx >= count {
        bundle_complete();
    }
    logf!("processing tx %d/%d", (idx + 1) as u32, count as u32);
    let tx = unsafe {
        BUNDLE
            .get_mut()
            .as_ref()
            .and_then(|bundle| bundle.transactions.get(idx))
    };
    if let Some(tx) = tx {
        if execute_transaction(tx) {
            resume_bundle();
        }
    } else {
        logf!("missing tx at index %d", idx as u32);
        resume_bundle();
    }
}

pub(crate) extern "C" fn resume_bundle() -> ! {
    unsafe {
        let curr = *CURRNET_BUNDLE_TX.get_mut();
        *CURRNET_BUNDLE_TX.get_mut() = curr.wrapping_add(1);
    }
    process_bundle();
    loop {}
}

fn execute_transaction(tx: &Transaction) -> bool {
    match tx.tx_type {
        TransactionType::CreateAccount => {
            create_account(tx);
            true
        }
        TransactionType::ProgramCall => {
            program_call(tx, resume_bundle);
            false
        }
        _ => panic!("unsupported transaction type"),
    }
}

fn bundle_complete() -> ! {
    log!("transaction bundle complete");
    // Avoid drop-time teardown that can allocate/deallocate; we halt immediately.
    let bundle = unsafe { BUNDLE.get_mut().take() };
    if let Some(bundle) = bundle {
        forget(bundle);
    }
    unsafe { core::arch::asm!("ebreak") };
    loop {}
}
