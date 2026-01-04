#![no_std]
#![no_main]

extern crate clibc;
use clibc::types::address::Address;
use clibc::{entrypoint, require, types::result::Result};

// Import the sha2 library for hashing
use sha2::{Digest, Sha256};

/// Example program that imports and uses an external library (sha2)
///
/// This demonstrates importing and using an external cryptographic library
/// within a smart contract environment.
///
/// CONTRACT BEHAVIOR:
/// - Takes arbitrary input data
/// - Computes SHA-256 hash of the input
/// - Returns the 32-byte hash
///
/// INPUT FORMAT: Any arbitrary bytes
///
/// OUTPUT FORMAT: Returns a Result struct with:
/// - success: true (always succeeds if input is valid)
/// - error_code: 0 (no error)
/// - data_len: 32 (size of SHA-256 hash)
/// - data: The SHA-256 hash as 32 bytes
///
/// REAL-WORLD USAGE:
/// - Data integrity verification
/// - Creating commitments for reveal schemes
/// - Generating deterministic IDs from data
/// - Proof of data existence at a point in time
fn hasher_entry(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    // Validate that we have some input data
    require(data.len() > 0, b"Input data cannot be empty");

    // Create a new SHA-256 hasher instance
    let mut hasher = Sha256::new();

    // Feed the input data to the hasher
    hasher.update(data);

    // Compute the hash and get the result as a fixed array
    let hash_result = hasher.finalize();

    // Create a result with the hash data
    let mut result = Result::new(true, 0);

    // Copy the 32-byte hash into the result's data field
    result.data[..32].copy_from_slice(&hash_result[..]);
    result.data_len = 32;

    result
}

// Register the function as the entrypoint
entrypoint!(hasher_entry);
