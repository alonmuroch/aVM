#![no_std]
#![no_main]

extern crate clibc;

use clibc::types::address::Address;
use clibc::types::result::Result;
use clibc::{DataParser, entrypoint, require};

/// Demonstrates transferring the native AM token from the caller to a target
/// address using the VM's transfer syscall. The input payload is:
/// - 20 bytes: destination address
/// - 8 bytes: amount (little-endian u64)
fn transfer_entry(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    let mut parser = DataParser::new(data);
    // Need at least 20 bytes for the address and 8 bytes for the value
    require(parser.remaining() >= 28, b"transfer: need addr + amount");

    let to = parser.read_address();
    let amount = parser.read_u64();

    // Capture recipient balance before/after for return value
    let _before = clibc::balance!(&to);
    let ok = clibc::transfer!(&to, amount);
    let after = clibc::balance!(&to);

    // Encode success flag in data for easier assertions
    let mut result = Result::new(ok, if ok { 0 } else { 1 });
    let balance_bytes = after.to_le_bytes();
    result.set_data(&balance_bytes);
    // In a failure case, keep the "after" (unchanged) balance visible too.
    result
}

entrypoint!(transfer_entry);
