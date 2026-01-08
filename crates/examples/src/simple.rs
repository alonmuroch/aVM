#![no_std]
#![no_main]

extern crate clibc;
use clibc::types::address::Address;
use clibc::{DataParser, entrypoint, require, types::result::Result};

/// Simple smart contract that compares two 32-bit integers.
///
/// EDUCATIONAL PURPOSE: This demonstrates a basic smart contract that:
/// - Accepts input data (two 32-bit integers)
/// - Performs a simple comparison operation
/// - Returns a result with the larger number stored in data
///
/// CONTRACT BEHAVIOR:
/// - Takes 8 bytes of input data (two 32-bit integers)
/// - Compares the first integer with the second
/// - Returns success=true if first > second, success=false otherwise
/// - Stores the larger value in the data field
///
/// INPUT FORMAT: The contract expects exactly 8 bytes:
/// - Bytes 0-3: First 32-bit integer (little-endian)
/// - Bytes 4-7: Second 32-bit integer (little-endian)
///
/// OUTPUT FORMAT: Returns a Result struct with:
/// - success: true if first > second, false otherwise
/// - error_code: 0 (no error)
/// - data_len: 4 (size of u32)
/// - data: The larger of the two input values stored as 4 bytes
///
/// REAL-WORLD USAGE: This type of contract could be used for:
/// - Simple validation logic
/// - Conditional execution based on input values
/// - Basic decision-making in decentralized applications
///
/// SECURITY CONSIDERATIONS:
/// - Input validation prevents buffer overflows
/// - No external calls or state modifications
/// - Deterministic execution for all inputs
fn program_entry(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    // EDUCATIONAL: Validate input data length to prevent buffer overflows
    // This is a critical security practice in smart contracts
    require(data.len() >= 8, b"Input data must be at least 8 bytes long");

    // EDUCATIONAL: Extract two 32-bit integers from the input data
    // This demonstrates how to parse binary data in smart contracts
    let mut parser = DataParser::new(data);
    let first = parser.read_u32();
    let second = parser.read_u32();

    // EDUCATIONAL: Perform the comparison and return appropriate result
    // This demonstrates conditional logic in smart contracts
    if first > second {
        // Return success with the larger number (first) stored in data
        Result::with_u32(first)
    } else {
        // Return success with the larger number (second) stored in data
        Result::with_u32(second)
    }
}

// EDUCATIONAL: Register the function as the entrypoint
// This macro tells the VM which function to call when the contract is invoked
entrypoint!(program_entry);
