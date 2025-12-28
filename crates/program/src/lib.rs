#![no_std]

#[cfg(test)]
extern crate std;

extern crate alloc;

/* --------------------------------- Imports --------------------------------- */

// External crates
pub extern crate types;

/* --------------------------------- Modules --------------------------------- */

// Integers func
pub mod integers;
pub use integers::*;

pub mod transfer;
pub use transfer::transfer;
pub use transfer::balance;

// StorageMap
pub mod storage_map;
pub use storage_map::StorageMap;
pub use storage_map::StorageKey;

// Events
pub mod event;
pub use event::*; 

// Logging macros
pub mod log;
pub use log::BufferWriter;

// Data parser
pub mod parser;
pub use parser::{DataParser, HexCodec};

// Contract call func
pub mod call;

// Entrypoint macro
#[macro_use]
pub mod entrypoint;

// Persistent storage system
#[macro_use]
pub mod storage;
pub use storage::Persistent; // Allow `$crate::Persistent` in macros
pub use storage::PERSISTENT_DOMAIN; // Allow `$crate::PERSISTENT_DOMAIN` in macros

// Router
pub mod router;
pub use router::{decode_calls, route, FuncCall};

// Panic helper (and panic handler when guest feature enabled)
pub mod panic;
pub use panic::vm_panic;

// Memory allocator
pub mod allocator;

#[cfg(all(target_arch = "riscv32", feature = "guest"))]
#[global_allocator]
static ALLOCATOR: allocator::VmAllocator = allocator::VmAllocator;

/* --------------------------- Assertion Utilities -------------------------- */

/// Aborts execution if condition is false, printing `msg`.
/// 
/// EDUCATIONAL PURPOSE: This function demonstrates defensive programming in VM environments.
/// In virtual machines, especially those running untrusted code, we need to validate
/// all inputs and assumptions to prevent security vulnerabilities and crashes.
/// 
/// DESIGN PATTERN: This is an "assertion" or "guard clause" pattern. Instead of
/// letting invalid conditions cause undefined behavior later, we fail fast with
/// a clear error message. This makes debugging much easier.
/// 
/// DEFENSIVE PROGRAMMING PRINCIPLES:
/// - Validate all inputs at function boundaries
/// - Check preconditions before performing operations
/// - Provide clear, actionable error messages
/// - Fail fast to prevent cascading errors
/// - Assume all external data is potentially malicious
/// 
/// SECURITY CONSIDERATIONS:
/// - Always validate inputs before processing them
/// - Provide clear error messages for debugging
/// - Fail fast to prevent cascading errors
/// - In production VMs, you might want to log these failures for monitoring
/// - Consider rate limiting to prevent denial-of-service attacks
/// 
/// REAL-WORLD ANALOGY: This is like a bouncer at a club checking IDs. Instead
/// of letting people in and then dealing with problems later, you check
/// everything upfront and turn away anyone who doesn't meet the requirements.
/// 
/// USAGE EXAMPLES:
/// - require(data.len() >= 8, b"insufficient data");
/// - require(account.balance >= amount, b"insufficient funds");
/// - require(selector < MAX_SELECTORS, b"invalid selector");
/// - require(caller == owner, b"unauthorized access");
/// 
/// PARAMETERS:
/// - condition: Boolean expression that must be true for execution to continue
/// - msg: Error message to display if condition is false (as bytes for no_std compatibility)
/// 
/// BEHAVIOR: If condition is false, the VM will panic with the provided message
/// and halt execution. This prevents the VM from continuing with invalid state.
pub fn require(condition: bool, msg: &[u8]) {
    if !condition {
        vm_panic(msg);
    }
}
