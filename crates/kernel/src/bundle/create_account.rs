use kernel::global::STATE;
use kernel::Config;
use program::logf;
use program::parser::HexCodec;
use state::State;
use types::transaction::Transaction;

pub(crate) fn create_account(tx: &Transaction) {
    let code_size = tx.data.len();
    let is_contract = code_size > 0;

    let mut addr_buf = [0u8; 40];
    let addr_hex = HexCodec::encode(tx.to.as_ref(), &mut addr_buf);
    logf!(
        "Tx creating account at address %s. Is contract: %d. Code size: %d bytes.",
        addr_hex.as_ptr() as u32,
        addr_hex.len() as u32,
        is_contract as u32,
        code_size as u32
    );

    let max = Config::CODE_SIZE_LIMIT + Config::RO_DATA_SIZE_LIMIT;
    if code_size > max {
        panic!(
            "❌ Code size ({}) exceeds CODE_SIZE_LIMIT ({} bytes)",
            code_size, max
        );
    }

    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    let account = state.get_account_mut(&tx.to);
    account.code = tx.data.clone();
    account.is_contract = is_contract;
    logf!(
        "account created in kernel state: addr=%s is_contract=%d code_len=%d",
        addr_hex.as_ptr() as u32,
        addr_hex.len() as u32,
        is_contract as u32,
        code_size as u32
    );
}
