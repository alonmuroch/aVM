#![allow(dead_code)]

use std::env;
use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use core::cell::RefCell;
use core::fmt::Write;
use bootloader::bootloader::Bootloader;
use state::State;
use crate::state_helper::test_state;

// File writer for logging to disk
struct FileWriter {
    file: File,
}

impl FileWriter {
    fn new(path: &str) -> std::io::Result<Self> {
        Ok(FileWriter {
            file: File::create(path)?,
        })
    }
}

impl Write for FileWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.file
            .write_all(s.as_bytes())
            .map_err(|_| core::fmt::Error)?;
        self.file.flush().map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}

/// Console writer that wraps println!
struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

/// Test runner that encapsulates test execution with configurable output
pub struct TestRunner {
    writer: Rc<RefCell<dyn Write>>,
    verbose: bool,
    vm_memory_size: usize,
    kernel_bytes: Option<Vec<u8>>,
    kernel_path: Option<String>,
}

impl TestRunner {
    /// Create a new test runner with console output (default)
    pub fn new() -> Self {
        Self::with_writer(Rc::new(RefCell::new(ConsoleWriter)))
    }

    /// Set VM memory size
    pub fn with_memory_size(mut self, size: usize) -> Self {
        self.vm_memory_size = size;
        self
    }

    /// Enable or disable verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Create a test runner with file output
    pub fn with_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file_writer = FileWriter::new(path.as_ref().to_str().unwrap())?;
        Ok(Self::with_writer(Rc::new(RefCell::new(file_writer))))
    }

    /// Create a test runner with a custom writer
    pub fn with_writer(writer: Rc<RefCell<dyn Write>>) -> Self {
        TestRunner {
            writer,
            verbose: false,
            vm_memory_size: 4 * 1024 * 1024, // larger default to accommodate bigger binaries without RVC
            kernel_bytes: Self::load_kernel_from_env(),
            kernel_path: env::var("KERNEL_ELF").ok(),
        }
    }

    /// Execute all test cases
    pub fn execute(&self) -> Result<(), String> {
        use super::TEST_CASES;

        writeln!(self.writer.borrow_mut(), "=== Starting Test Run ===").unwrap();
        writeln!(
            self.writer.borrow_mut(),
            "Verbose logging: {}",
            if self.verbose { "enabled" } else { "disabled" }
        )
        .unwrap();

        for case in TEST_CASES.iter() {
            self.run_test_case(case)?;
        }

        // Write test summary
        writeln!(self.writer.borrow_mut(), "\n=== Test Run Complete ===").unwrap();
        writeln!(
            self.writer.borrow_mut(),
            "Total test cases: {}",
            TEST_CASES.len()
        )
        .unwrap();

        Ok(())
    }

    /// Run a single test case
    fn run_test_case(&self, case: &super::TestCase) -> Result<(), String> {
        let mut bootloader = Bootloader::new(self.vm_memory_size);
        let state = Rc::new(RefCell::new(test_state()));

        // Write test case header
        writeln!(
            self.writer.borrow_mut(),
            "\n############################################"
        )
        .unwrap();
        writeln!(
            self.writer.borrow_mut(),
            "#### Running test case: {} ####",
            case.name
        )
        .unwrap();
        writeln!(
            self.writer.borrow_mut(),
            "############################################"
        )
        .unwrap();

        // Print address to binary mappings
        if !case.address_mappings.is_empty() {
            writeln!(self.writer.borrow_mut(), "\n📍 Address -> Binary Mappings:").unwrap();
            for (addr, binary) in &case.address_mappings {
                writeln!(self.writer.borrow_mut(), "  {} -> {}", addr, binary).unwrap();
            }
        }
        writeln!(self.writer.borrow_mut()).unwrap();

        // Execute the whole bundle via the bootloader/kernel path.
        let result = bootloader.execute_bundle(
            self.kernel_bytes.as_ref().ok_or_else(|| {
                "KERNEL_ELF not set or unreadable; bootloader path required".to_string()
            })?,
            &case.bundle,
            state,
            self.verbose,
            if self.verbose {
                Some(self.writer.clone())
            } else {
                None
            },
        );

        let result = match result {
            Some(result) => result,
            None => {
                return Err("Bootloader returned no receipts".to_string());
            }
        };
        if let Some(post_state) = result.state {
            println!("\n=== State After Execution ===");
            print_state(&post_state);
        }

        let receipt = result
            .receipts
            .last()
            .ok_or_else(|| "No receipts returned from kernel".to_string())?;
        writeln!(self.writer.borrow_mut(), "\n=== Receipt ===").unwrap();
        writeln!(self.writer.borrow_mut(), "{receipt}").unwrap();
        let result = receipt.result;
        if result.success != case.expected_success {
            return Err(format!(
                "Expected success={}, got {}",
                case.expected_success, result.success
            ));
        }
        let error_code = result.error_code;
        if error_code != case.expected_error_code {
            return Err(format!(
                "Expected error_code={}, got {}",
                case.expected_error_code, error_code
            ));
        }
        match &case.expected_data {
            Some(expected) => {
                let expected_len = expected.len();
                let data_len = result.data_len as usize;
                let actual_len = data_len;
                if actual_len != expected_len {
                    return Err(format!(
                        "Expected data_len={}, got {}",
                        expected_len, actual_len
                    ));
                }
                let actual = &result.data[..actual_len.min(result.data.len())];
                if actual != expected.as_slice() {
                    return Err(format!(
                        "Expected data {:?}, got {:?}",
                        expected, actual
                    ));
                }
            }
            None => {
                let data_len = result.data_len;
                if data_len != 0 {
                    return Err(format!(
                        "Expected empty data, got data_len={}",
                        data_len
                    ));
                }
            }
        }

        writeln!(
            self.writer.borrow_mut(),
            "✅ Test case '{}' passed",
            case.name
        )
        .unwrap();

        // For now we treat successful bootloader execution as a passed test.
        Ok(())
    }
}

fn print_state(state: &State) {
    println!("--- State Dump ---");
    for (addr, acc) in &state.accounts {
        println!("  Address: 0x{}", hex_encode(addr.0));
        println!("      - Balance: {}", acc.balance);
        println!("      - Nonce: {}", acc.nonce);
        println!("      - Is contract?: {}", acc.is_contract);
        println!("      - Code size: {} bytes", acc.code.len());
        println!("      - Storage:");
        for (key, value) in &acc.storage {
            let value_hex = hex_join(value);
            println!(
                "          Key: {:<20} | Value ({} bytes): {}",
                key,
                value.len(),
                value_hex
            );
        }
        println!();
    }
    println!("--------------------");
}

fn hex_encode(bytes: [u8; 20]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(40);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_join(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(3));
    for (idx, b) in bytes.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::with_writer(Rc::new(RefCell::new(ConsoleWriter)))
    }
}

impl TestRunner {
    fn load_kernel_from_env() -> Option<Vec<u8>> {
        let path = env::var("KERNEL_ELF")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bootloader/bin/kernel.elf"));
        fs::read(&path).ok()
    }
}
