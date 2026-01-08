#![no_std]
#![no_main]

extern crate alloc;

use clibc::{
    DataParser, entrypoint, require, types::address::Address, types::result::Result, vm_panic,
};

entrypoint!(program_entry);
/// Guest program that demonstrates heap allocation using VM syscalls
fn program_entry(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    // Need to import alloc types after entrypoint macro includes the allocator
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;

    // Expect at least 6 u32 values in little-endian form:
    // - first 3 populate the Vec
    // - next 3 populate the BTreeMap values for alice/bob/charlie
    let mut parser = DataParser::new(data);
    if parser.remaining() < 6 * 4 {
        vm_panic(b"insufficient data for allocator demo");
    }

    let n0 = parser.read_u32();
    let n1 = parser.read_u32();
    let n2 = parser.read_u32();

    let alice_score = parser.read_u32();
    let bob_score = parser.read_u32();
    let charlie_score = parser.read_u32();

    // Test Vec operations
    let numbers = Vec::from([n0, n1, n2]);

    match numbers.first() {
        Some(&n) => require(n == n0, b"First element mismatch"),
        None => vm_panic(b"Vec should not be empty"),
    }
    match numbers.get(1) {
        Some(&n) => require(n == n1, b"Second element mismatch"),
        None => vm_panic(b"Vec should not be empty"),
    }
    match numbers.get(2) {
        Some(&n) => require(n == n2, b"Third element mismatch"),
        None => vm_panic(b"Vec should not be empty"),
    }
    require(numbers.len() == 3, b"Vec should have 3 elements");

    // Test Vec with capacity
    let mut large_vec = Vec::with_capacity(50);
    for i in 0..20 {
        large_vec.push(i);
    }
    require(
        large_vec.len() == 20,
        b"Large vector should have 20 elements",
    );

    // Test BTreeMap operations
    let mut scores = BTreeMap::new();
    scores.insert("alice", alice_score);
    scores.insert("bob", bob_score);
    scores.insert("charlie", charlie_score);

    match scores.get("alice") {
        Some(&v) => require(v == alice_score, b"Alice's score mismatch"),
        None => vm_panic(b"Alice not found in BTreeMap"),
    }

    match scores.get("bob") {
        Some(&v) => require(v == bob_score, b"Bob's score mismatch"),
        None => vm_panic(b"Bob not found in BTreeMap"),
    }

    match scores.get("charlie") {
        Some(&v) => require(v == charlie_score, b"Charlie's score mismatch"),
        None => vm_panic(b"Charlie not found in BTreeMap"),
    }

    require(scores.len() == 3, b"BTreeMap should have 3 elements");

    Result::new(true, 0)
}
