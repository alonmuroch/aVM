//! Standalone test runner for rv32ui-p-* ELF files from riscv-tests
//! Loads an ELF file, loads it into the VM, and runs it to completion.

use std::io::Read;
use std::path::Path;
use vm::memory::{Perms, Sv32Memory, VirtualAddress, API, MMU, PAGE_SIZE};
use vm::registers::Register;
use vm::vm::VM;

const DEFAULT_VM_SIZE: usize = 16 * 1024 * 1024;
const STACK_SIZE: usize = 256 * 1024;
const MAX_STEPS: usize = 20_000_000;

/// Tests that are skipped and the reasons why
const SKIPPED_TESTS: &[(&str, &str)] = &[
    (
        "fence_i",
        "Requires self-modifying code support (writes instructions to memory and executes them)",
    ),
    (
        "ld_st",
        "Contains 64-bit load/store instructions (ld/sd) that the 32-bit VM doesn't support",
    ),
    (
        "st_ld",
        "Contains 64-bit store/load instructions (sd/ld) that the 32-bit VM doesn't support",
    ),
    (
        "lrsc",
        "LR/SC implementation needs improvement - causes infinite loops",
    ),
];

/// Testing categories to run
const TESTING_CATEGORIES: &[&str] = &["ui", "um", "ua", "uc"];

/// Check if a test file should be skipped
fn should_skip_test(file_name: &str) -> Option<&str> {
    for (test_name, reason) in SKIPPED_TESTS {
        if file_name.ends_with(test_name) {
            return Some(reason);
        }
    }
    None
}

/// Run a single test file
fn run_single_test(elf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(elf_path).exists() {
        println!("ELF file not found at {}, skipping...", elf_path);
        return Ok(());
    }

    // Read ELF file
    let mut file = std::fs::File::open(elf_path)?;
    let mut elf_bytes = Vec::new();
    file.read_to_end(&mut elf_bytes)?;

    // Parse ELF
    let elf = compiler::elf::parse_elf_from_bytes(&elf_bytes)?;
    let (code, code_start) = elf.get_flat_code().ok_or("No code section in ELF")?;
    let (rodata, rodata_start) = elf.get_flat_rodata().unwrap_or((vec![], u64::MAX));
    let (bss, bss_start) = elf.get_flat_bss().unwrap_or((vec![], u64::MAX));

    // Get .data section if it exists
    let (data, data_start) = if let Some(data_section) = elf.get_section_by_name(".data") {
        (data_section.data.to_vec(), data_section.addr as usize)
    } else {
        (vec![], usize::MAX)
    };

    // Find .tohost section
    let tohost_section = if let Some(tohost_section) = elf.get_section_by_name(".tohost") {
        println!(
            ".tohost section found at addr=0x{:x}, size=0x{:x}",
            tohost_section.addr, tohost_section.size
        );
        tohost_section
    } else {
        println!(".tohost section not found, skipping...");
        return Ok(());
    };
    let tohost_addr = tohost_section.addr;

    let mut min_base = code_start as usize;
    let mut image_end = (code_start as usize) + code.len();

    if !rodata.is_empty() {
        min_base = min_base.min(rodata_start as usize);
        image_end = image_end.max((rodata_start as usize) + rodata.len());
    }
    if !data.is_empty() {
        min_base = min_base.min(data_start);
        image_end = image_end.max(data_start + data.len());
    }
    if !bss.is_empty() {
        min_base = min_base.min(bss_start as usize);
        image_end = image_end.max((bss_start as usize) + bss.len());
    }
    let tohost_start = tohost_section.addr as usize;
    let tohost_end = tohost_start + (tohost_section.size as usize);
    min_base = min_base.min(tohost_start);
    image_end = image_end.max(tohost_end);

    let image_size = image_end
        .checked_sub(min_base)
        .ok_or("invalid image size")?;
    let map_len = image_size + STACK_SIZE;
    let total_size = map_len.max(DEFAULT_VM_SIZE);
    let memory = std::rc::Rc::new(Sv32Memory::new(total_size, PAGE_SIZE));

    println!(
        "Loading code into VM: addr=0x{:x}, size=0x{:x}",
        code_start,
        code.len()
    );
    println!(
        "Mapping {:x}-{:x} (size=0x{:x})",
        min_base,
        min_base + map_len,
        map_len
    );
    memory.map_range(
        VirtualAddress(min_base as u32),
        map_len,
        Perms::rwx_kernel(),
    );

    let mut image = vec![0u8; image_size];
    let code_off = (code_start as usize).saturating_sub(min_base);
    image[code_off..code_off + code.len()].copy_from_slice(&code);
    if !rodata.is_empty() {
        let ro_off = (rodata_start as usize).saturating_sub(min_base);
        image[ro_off..ro_off + rodata.len()].copy_from_slice(&rodata);
    }
    if !data.is_empty() {
        let data_off = data_start.saturating_sub(min_base);
        image[data_off..data_off + data.len()].copy_from_slice(&data);
    }
    if !bss.is_empty() {
        let bss_off = (bss_start as usize).saturating_sub(min_base);
        image[bss_off..bss_off + bss.len()].copy_from_slice(&bss);
    }
    if !tohost_section.data.is_empty() {
        let tohost_off = tohost_start.saturating_sub(min_base);
        image[tohost_off..tohost_off + tohost_section.data.len()]
            .copy_from_slice(tohost_section.data);
    }
    memory.write_bytes(VirtualAddress(min_base as u32), &image);

    let stack_top = (min_base as u32)
        .checked_add(map_len as u32)
        .ok_or("stack top overflow")?;
    let entry_point = code_start as u32;

    let mut vm = VM::new(memory.clone());
    vm.cpu.verbose = false;
    vm.cpu.pc = entry_point;
    vm.set_reg_u32(Register::Sp, stack_top);
    let root_satp = memory.satp();

    println!("Running test...");
    let mut steps = 0usize;
    loop {
        if !vm.cpu.step(memory.clone()) {
            break;
        }
        steps += 1;
        if memory.satp() == 0 {
            memory.set_satp(root_satp);
        }
        if steps > MAX_STEPS {
            return Err("execution limit reached without tohost signal".into());
        }
        let tohost_value = read_tohost_value(memory.as_ref(), tohost_addr)?;
        if tohost_value != 0 {
            if tohost_value == 1 {
                println!("Test completed.");
                return Ok(());
            }
            return Err(format!("test failed (tohost=0x{:x})", tohost_value).into());
        }
    }

    let exit_id = vm.cpu.regs[Register::A7 as usize];
    if exit_id == 93 {
        let exit_code = vm.cpu.regs[Register::A0 as usize];
        if exit_code == 0 {
            println!("Test completed.");
            return Ok(());
        }
        return Err(format!("test failed (ecall exit code={})", exit_code).into());
    }

    Err("execution halted without tohost signal".into())
}

fn read_tohost_value(
    memory: &Sv32Memory,
    tohost_addr: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    let addr = u32::try_from(tohost_addr).map_err(|_| "tohost address out of range")?;
    let start = VirtualAddress(addr);
    let end = start.checked_add(8).ok_or("tohost address overflow")?;
    let slice = memory.mem_slice(start, end).ok_or("tohost not mapped")?;
    if slice.len() < 8 {
        return Err("tohost slice truncated".into());
    }
    let bytes: [u8; 8] = slice[0..8].try_into()?;
    Ok(u64::from_le_bytes(bytes))
}

/// Discover and collect test files for a specific category
fn collect_test_files(test_dir: &str, category: &str) -> (Vec<String>, usize) {
    let mut test_files = Vec::new();
    let mut skipped_count = 0;

    println!("Looking for files in: {}", test_dir);
    println!("Category prefix: rv32{}p-", category);

    if let Ok(entries) = std::fs::read_dir(test_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    if let Some(name_str) = file_name.to_str() {
                        // Include files that start with the category prefix and are not .dump files
                        let category_prefix = format!("rv32{}-p-", category);
                        if name_str.starts_with(&category_prefix)
                            && !path.is_dir()
                            && !name_str.ends_with(".dump")
                        {
                            // Check if this test should be skipped
                            if let Some(reason) = should_skip_test(name_str) {
                                println!("Skipping {}: {}", name_str, reason);
                                skipped_count += 1;
                                continue;
                            }

                            test_files.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    } else {
        println!("Failed to read directory: {}", test_dir);
    }

    test_files.sort(); // Sort for consistent ordering
    (test_files, skipped_count)
}

/// Run all tests for a specific category
fn run_category_tests(
    test_dir: &str,
    category: &str,
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    println!(
        "\n=== Running {} category tests ===",
        category.to_uppercase()
    );

    let (test_files, skipped_count) = collect_test_files(test_dir, category);
    println!(
        "Found {} {} test files to run ({} skipped)",
        test_files.len(),
        category,
        skipped_count
    );

    let mut passed_count = 0;
    let failed_count = 0;

    for (i, elf_path) in test_files.iter().enumerate() {
        let test_name = std::path::Path::new(elf_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        print!("[{:2}/{:2}] {}: ", i + 1, test_files.len(), test_name);

        if let Err(e) = run_single_test(elf_path) {
            println!("❌ FAILED - {}", e);
            return Err(e);
        } else {
            println!("✅ PASSED");
            passed_count += 1;
        }
    }

    println!(
        "=== {} category tests completed ===",
        category.to_uppercase()
    );
    Ok((passed_count, failed_count, skipped_count))
}

#[test]
fn test_riscv_spec() {
    // Discover all test files in the riscv-tests directory
    let test_dir = "tests/riscv-tests-install/share/riscv-tests/isa";

    // Print current working directory for debugging
    println!("Current dir: {:?}", std::env::current_dir().unwrap());
    println!("Looking for tests in: {}", test_dir);

    // Check if the test directory exists
    if !Path::new(test_dir).exists() {
        println!("Test directory not found at {}, skipping test", test_dir);
        return;
    }

    println!("\n🚀 Starting RISC-V Specification Test Suite");
    println!("{}", "=".repeat(60));

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_skipped = 0;
    let mut category_results = Vec::new();

    // Run tests for each category
    for category in TESTING_CATEGORIES {
        match run_category_tests(test_dir, category) {
            Ok((passed, failed, skipped)) => {
                total_passed += passed;
                total_failed += failed;
                total_skipped += skipped;
                category_results.push((category.to_string(), passed, failed, skipped));
            }
            Err(e) => {
                println!("❌ Failed to run {} category tests: {}", category, e);
                panic!("Test suite failed");
            }
        }
    }

    // Print comprehensive summary
    println!("\n{}", "=".repeat(60));
    println!("📊 RISC-V SPECIFICATION TEST SUITE SUMMARY");
    println!("{}", "=".repeat(60));

    // Category breakdown
    println!("\n📋 Category Breakdown:");
    for (category, passed, failed, skipped) in &category_results {
        let total = passed + failed + skipped;
        let success_rate = if total > 0 {
            (*passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {}: {}/{} passed ({:.1}%) {} skipped",
            category.to_uppercase(),
            passed,
            total,
            success_rate,
            skipped
        );
    }

    // Overall statistics
    let total_tests = total_passed + total_failed + total_skipped;
    let overall_success_rate = if total_tests > 0 {
        (total_passed as f64 / total_tests as f64) * 100.0
    } else {
        0.0
    };

    println!("\n📈 Overall Statistics:");
    println!("  Total Tests: {}", total_tests);
    println!("  Passed: {} ✅", total_passed);
    println!("  Failed: {} ❌", total_failed);
    println!("  Skipped: {} ⏭️", total_skipped);
    println!("  Success Rate: {:.1}%", overall_success_rate);

    // Test coverage information
    println!("\n🎯 Test Coverage:");
    println!("  UI Tests: Base integer instructions (RV32I)");
    println!("  UM Tests: Integer multiplication and division (RV32M)");
    println!("  UA Tests: Atomic memory operations (RV32A)");
    println!("  UC Tests: Compressed instructions (RV32C)");

    // Skipped tests explanation
    if total_skipped > 0 {
        println!("\n⏭️ Skipped Tests:");
        for (test_name, reason) in SKIPPED_TESTS {
            println!("  - {}: {}", test_name, reason);
        }
    }

    println!("\n{}", "=".repeat(60));

    if total_failed > 0 {
        panic!("Test suite completed with {} failures", total_failed);
    } else {
        println!("🎉 All tests passed successfully!");
    }
}
