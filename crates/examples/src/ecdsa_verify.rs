#![no_std]
#![no_main]

extern crate clibc;
use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use clibc::{
    DataParser, HexCodec, entrypoint, log, logf, require, types::address::Address,
    types::result::Result, vm_panic,
};

/// ECDSA verification example using k256.
/// Input layout:
/// - 1 byte: pubkey length (33 or 65)
/// - N bytes: SEC1-encoded pubkey
/// - 64 bytes: signature (r||s)
/// - 32 bytes: message hash (already hashed)
fn my_vm_entry(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    let mut parser = DataParser::new(data);

    let pk_len = parser.read_bytes(1)[0] as usize;
    require(
        pk_len == 33 || pk_len == 65,
        b"pubkey must be 33 or 65 bytes",
    );
    let pk_bytes = parser.read_bytes(pk_len);
    let sig_bytes = parser.read_bytes(64);
    let hash = parser.read_bytes(32);

    let verifying_key =
        VerifyingKey::from_sec1_bytes(pk_bytes).unwrap_or_else(|_| vm_panic(b"invalid pubkey"));
    let signature =
        Signature::from_slice(sig_bytes).unwrap_or_else(|_| vm_panic(b"invalid signature"));

    // Log the received inputs in hex for visibility
    logf!("ecdsa_verify: pk_len=%d", pk_len as u32);
    let mut pk_hex = [0u8; 130]; // 65 bytes max -> 130 hex chars
    let mut sig_hex = [0u8; 128]; // 64 bytes -> 128 hex chars
    let mut hash_hex = [0u8; 64]; // 32 bytes -> 64 hex chars
    log!("pubkey: %s", HexCodec::encode(pk_bytes, &mut pk_hex));
    log!("signature: %s", HexCodec::encode(sig_bytes, &mut sig_hex));
    log!("hash: %s", HexCodec::encode(hash, &mut hash_hex));

    verifying_key
        .verify_prehash(hash, &signature)
        .unwrap_or_else(|_| vm_panic(b"signature verification failed"));

    Result::new(true, 0)
}

entrypoint!(my_vm_entry);
