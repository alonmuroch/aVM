use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use state::{Account, State};
use types::address::Address;

fn assert_account_eq(expected: &Account, actual: &Account) {
    assert_eq!(expected.nonce, actual.nonce);
    assert_eq!(expected.balance, actual.balance);
    assert_eq!(expected.code, actual.code);
    assert_eq!(expected.is_contract, actual.is_contract);
    assert_eq!(expected.storage, actual.storage);
}

#[test]
fn encode_decode_empty_state() {
    let state = State::new();
    let encoded = state.encode();
    let decoded = State::decode(&encoded).expect("decode empty state");
    assert!(decoded.accounts.is_empty());
}

#[test]
fn encode_decode_with_account_and_storage() {
    let mut state = State::new();
    let addr = Address([
        0x01, 0x02, 0x03, 0x04, 0x05,
        0x06, 0x07, 0x08, 0x09, 0x0a,
        0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14,
    ]);
    let mut storage = BTreeMap::new();
    storage.insert(String::from("key"), vec![0xde, 0xad, 0xbe, 0xef]);
    storage.insert(String::from("empty"), Vec::new());
    let account = Account {
        nonce: 42,
        balance: 123_456_789,
        code: vec![0xaa, 0xbb, 0xcc],
        is_contract: true,
        storage,
    };
    state.accounts.insert(addr, account.clone());

    let encoded = state.encode();
    let decoded = State::decode(&encoded).expect("decode populated state");

    let decoded_account = decoded.accounts.get(&addr).expect("account exists");
    assert_account_eq(&account, decoded_account);
}

#[test]
fn decode_truncated_bytes_returns_none() {
    let mut state = State::new();
    let addr = Address([0x11; 20]);
    state.accounts.insert(
        addr,
        Account {
            nonce: 1,
            balance: 2,
            code: vec![0x42],
            is_contract: false,
            storage: BTreeMap::new(),
        },
    );
    let encoded = state.encode();
    let truncated = &encoded[..encoded.len().saturating_sub(1)];
    assert!(State::decode(truncated).is_none());
}

#[test]
fn decode_zero_count_header_returns_empty_state() {
    let bytes = [0u8; 4];
    let decoded = State::decode(&bytes).expect("decode zero-count header");
    assert!(decoded.accounts.is_empty());
}
