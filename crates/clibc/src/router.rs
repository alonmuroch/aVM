use types::result::Result;
use types::address::Address;
use types::{O};
use crate::logf;

/// Represents a function call with a selector (function ID) and arguments.
/// This is the core data structure for routing function calls in our VM.
/// 
/// EDUCATIONAL NOTE: In virtual machines, we often need to call different functions
/// based on a selector (like a function ID). This struct holds the selector and
/// the raw bytes that represent the function arguments. The lifetime 'a ensures
/// the args reference stays valid as long as the input data exists.
/// 
/// REAL-WORLD ANALOGY: Think of this like a function pointer in C, but with
/// additional metadata. Instead of just having a pointer to a function, we have
/// both the function identifier (selector) and its arguments bundled together.
/// This is similar to how web APIs work - you send a request with a method name
/// (like "GET" or "POST") and some data, and the server routes it to the right handler.
/// 
/// BLOCKCHAIN CONTEXT: In smart contracts, this pattern is used extensively.
/// When you call a smart contract, you don't call a specific function by name.
/// Instead, you send a transaction with a selector (usually the first 4 bytes
/// of the function signature hash) and the encoded arguments. The contract's
/// router then uses this selector to determine which function to execute.
/// 
/// MEMORY EFFICIENCY: Using a reference (&'a [u8]) instead of owned data (Vec<u8>)
/// means we don't need to copy the argument data. This is crucial in embedded
/// systems and VMs where memory is limited and copying can be expensive.
#[derive(Debug, Clone, Copy)]
pub struct FuncCall<'a> {
    /// The function selector (0-255) that identifies which function to call
    pub selector: u8,
    /// Raw bytes containing the function arguments
    pub args: &'a [u8],
}

use crate::{vm_panic};

/// Decodes a sequence of function calls from a binary input buffer.
/// 
/// EDUCATIONAL PURPOSE: This function demonstrates how to parse a binary protocol
/// where multiple function calls are packed into a single buffer. This is common
/// in blockchain VMs and RPC systems.
/// 
/// BINARY FORMAT EXPLANATION:
/// Each function call is encoded as: [selector: u8][arg_len: u8][args: arg_len bytes]
/// - selector: Which function to call (0-255)
/// - arg_len: How many bytes of arguments follow
/// - args: The actual argument data
/// 
/// EXAMPLE: If you want to call function 0x01 with arguments [1, 2, 3, 4], the
/// binary encoding would be: [0x01][0x04][0x01][0x02][0x03][0x04]
/// 
/// BATCH PROCESSING: This function can decode multiple function calls from a
/// single buffer. This is useful for batch transactions where you want to
/// execute several operations atomically (all succeed or all fail).
/// 
/// MEMORY LAYOUT: The function uses a pre-allocated buffer to avoid dynamic
/// memory allocation. This is important in embedded systems and VMs where
/// heap allocation can be expensive or unavailable.
/// 
/// SAFETY CONSIDERATIONS:
/// - We validate buffer bounds to prevent out-of-bounds access
/// - We check for incomplete headers and malformed data
/// - We limit the number of calls to prevent buffer overflow
/// - We use Rust's type system to ensure memory safety
/// 
/// ERROR HANDLING: When invalid data is encountered, the function calls vm_panic
/// to halt execution. This is appropriate for VM environments where continuing
/// with invalid data could lead to undefined behavior.
/// 
/// PARAMETERS:
/// - input: The binary buffer containing encoded function calls
/// - buffer: Pre-allocated array to store decoded FuncCall structs
/// 
/// RETURNS: Number of successfully decoded function calls
pub fn decode_calls<'a>(mut input: &'a [u8], buffer: &mut [FuncCall<'a>]) -> usize {
    let mut count = 0;

    // Process each function call in the input buffer
    while !input.is_empty() {
        // EDUCATIONAL: Always check buffer bounds before accessing data
        // This prevents segmentation faults and security vulnerabilities
        if input.len() < 2 {
            vm_panic(b"decode: incomplete header");
        }

        // Extract the function selector and argument length
        let selector = input[0];
        let arg_len = input[1] as usize;

        // EDUCATIONAL: Validate that we have enough data for the arguments
        // This is crucial for preventing buffer overruns
        if input.len() < 2 + arg_len {
            vm_panic(b"decode: args too short");
        }

        // EDUCATIONAL: Prevent buffer overflow by checking our output buffer capacity
        if count >= buffer.len() {
            vm_panic(b"decode: too many calls");
        }

        // Extract the argument bytes (slice from position 2 to 2+arg_len)
        let args = &input[2..2 + arg_len];

        // Store the decoded function call in our buffer
        buffer[count] = FuncCall {
            selector,
            args,
        };

        count += 1;
        // Move to the next function call in the input buffer
        input = &input[2 + arg_len..];
    }

    count
}

/// Generic router that takes the input buffer and a closure to dispatch each call.
/// 
/// EDUCATIONAL PURPOSE: This function demonstrates the power of Rust's generic
/// programming and closures. It's a higher-order function that can work with
/// any handler function you provide.
/// 
/// DESIGN PATTERN: This implements the "Strategy Pattern" where the routing
/// logic is separated from the actual function handling logic. This makes the
/// code more modular and testable.
/// 
/// TWO-PHASE EXECUTION: The function works in two phases:
/// 1. DECODE PHASE: Parse all function calls from the input buffer
/// 2. EXECUTE PHASE: Run each function call through the provided handler
/// 
/// This separation allows for better error handling and ensures that all
/// calls are properly decoded before any execution begins.
/// 
/// CLOSURE PATTERN: The handler parameter is a closure (impl FnMut), which means
/// it can capture variables from its environment. This is powerful because it
/// allows the router to work with any context it needs (like a VM state, database
/// connection, etc.) without hardcoding those dependencies.
/// 
/// MEMORY MANAGEMENT: We use a fixed-size array to avoid heap allocations,
/// which is important in embedded systems and VMs where memory is constrained.
/// 
/// BATCH SEMANTICS: All function calls in a batch are processed, even if some fail.
/// This is different from traditional transaction semantics where one failure
/// would roll back the entire batch. This design choice allows for partial
/// success scenarios and better error reporting.
/// 
/// ERROR HANDLING: Each function call can succeed or fail, but we continue
/// processing all calls and return the result of the last one. This ensures
/// that one bad function call doesn't stop the entire batch.
/// 
/// REAL-WORLD USAGE: This pattern is used in many systems:
/// - Web servers that process multiple requests in a batch
/// - Database systems that execute multiple queries atomically
/// - Blockchain VMs that process multiple contract calls
/// 
/// PARAMETERS:
/// - input: Binary buffer containing encoded function calls
/// - max_calls: Maximum number of calls to process (safety limit)
/// - handler: Closure that processes each individual function call
/// 
/// RETURNS: Result of the last processed function call
/// Routes function calls from an encoded input buffer to the given handler closure.
/// Passes in the `to` (contract address) and `from` (caller address) to each call.
pub fn route<'a>(
    input: &'a [u8],
    to: Address,
    from: Address,
    mut handler: impl FnMut(Address, Address, &FuncCall<'a>) -> Result,
) -> Result {
    // Fixed-size buffer: max 8 calls
    let mut buf: [O<FuncCall<'a>>; 8] = [O::None, O::None, O::None, O::None, O::None, O::None, O::None, O::None];
    let mut input = input;
    let mut count = 0;

    // Phase 1: Decode calls into buffer
    while !input.is_empty() && count < buf.len() {
        if input.len() < 2 {
            vm_panic(b"router: bad header");
        }

        let selector = input[0];
        let arg_len = input[1] as usize;

        if input.len() < 2 + arg_len {
            vm_panic(b"router: bad arg len");
        }

        let args = &input[2..2 + arg_len];
        buf[count] = O::Some(FuncCall { selector, args });
        count += 1;
        input = &input[2 + arg_len..];
    }

    // Phase 2: Execute calls
    let mut last_result = Result::new(true, 0);

    for i in 0..count {
        if let O::Some(call) = &buf[i] {
            last_result = handler(to, from, call);
        }
    }

    last_result
}