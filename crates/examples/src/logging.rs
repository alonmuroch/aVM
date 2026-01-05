#![no_std]
#![no_main]

extern crate clibc;
use clibc::types::address::Address;
use clibc::{DataParser, concat_str, entrypoint, log, logf, types::result::Result};
use core::fmt;

/// Comprehensive logging demonstration showing all format specifiers
unsafe fn logging(program: Address, _caller: Address, data: &[u8]) -> Result {
    let _ = program;
    // Simple string logging
    logf!("=== Logging Demo Started ===");

    // Integer formats
    let num = 42;
    logf!("Decimal: %d", num);
    logf!("Unsigned: %u", num);
    logf!("Hexadecimal: %x", 0xDEADBEEF);

    // Multiple values in one log
    let x = 10;
    let y = 20;
    logf!("x=%d, y=%d, sum=%d", x, y, x + y);

    // Character logging
    let ch = 'A' as u32;
    logf!("Character: %c", ch);

    // Floating point
    let pi_bits = 3.14159f32.to_bits();
    logf!("Pi approximation: %f", pi_bits);

    // String logging - now simplified!
    let msg = b"Hello, VM!";
    log!("Message: %s", msg);

    // String concatenation - requires a buffer in no_std (even though we have an allocator)
    let mut buffer = [0u8; 64]; // Stack-allocated storage
    let greeting = concat_str!(buffer, b"Hello, ", b"World", b"!");
    log!("Concatenated: %s", greeting);

    // Byte array logging (hex format) - simplified!
    let bytes = [0xDE, 0xAD, 0xBE, 0xEF];
    log!("Bytes (hex): %b", bytes);

    // Array of u32s - simplified!
    let numbers = [1u32, 2, 3, 4, 5];
    log!("Numbers: %a", numbers);

    // Array of u8s (decimal format) - simplified!
    let bytes_decimal = [10u8, 20, 30, 40, 50];
    log!("Bytes (decimal): %A", bytes_decimal);

    // Process input data
    let mut parser = DataParser::new(data);
    if parser.remaining() >= 4 {
        let value = parser.read_u32();
        logf!("Input value: %d (0x%x)", value, value);

        // Log remaining bytes if any
        if parser.remaining() > 0 {
            let remaining = parser.read_bytes(parser.remaining());
            logf!("Remaining %d bytes: ", remaining.len() as u32);
            log!("%b", remaining);
        }
    } else {
        logf!("Input too short: %d bytes", data.len() as u32);
    }

    // Demonstrate escape sequence
    logf!("100%% complete!");

    // Complex example with mixed types
    let score = 95;
    let grade = 'A' as u32;
    let bonus = 5;
    logf!(
        "Score: %d + Bonus: %d = Total: %d",
        score,
        bonus,
        score + bonus
    );
    logf!("Grade: %c", grade);

    // Large array (partial display for efficiency) - simplified!
    let large_array = [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    log!("Large array: %a", large_array);

    // Debug and Display trait demonstrations
    logf!("=== Debug and Display Trait Logging ===");

    // Create a custom struct that implements Debug and Display
    let point = Point { x: 10, y: 20 };
    logf!("Point (Debug): %s", debug: point);
    logf!("Point (Display): %s", display: point);

    // Test with Option types (implements Debug)
    let some_value: Option<u32> = Some(42);
    let none_value: Option<u32> = None;
    logf!("Some value: %s", debug: some_value);
    logf!("None value: %s", debug: none_value);

    // Test with Result types (implements Debug)
    let ok_result: core::result::Result<u32, &str> = Ok(100);
    let err_result: core::result::Result<u32, &str> = Err("error message");
    logf!("Ok result: %s", debug: ok_result);
    logf!("Err result: %s", debug: err_result);

    // Test with arrays (Debug)
    let debug_array = [1, 2, 3, 4, 5];
    logf!("Array debug: %s", debug: debug_array);

    // Test with tuples (Debug)
    let tuple = (42, "hello", true);
    logf!("Tuple: %s", debug: tuple);

    logf!("=== Logging Demo Complete ===");

    Result::new(true, 0)
}

// Custom struct for demonstrating Debug and Display
#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

entrypoint!(logging);
