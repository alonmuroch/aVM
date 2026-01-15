use std::collections::HashMap;
use std::path::{Path, PathBuf};

use a_tests::{AvmRunner, RunOptions, Suite, TestCase, TestEvaluator, TestKind, TestOutcome};
use types::TransactionReceipt;
use types::transaction::TransactionType;

#[path = "fixtures/examples.rs"]
mod fixtures;

use fixtures::{all_example_cases, expected_for, test_state_bytes};

struct ExampleEvaluator;

impl TestEvaluator for ExampleEvaluator {
    fn evaluate(&self, case: &TestCase, result: &a_tests::RunResult) -> TestOutcome {
        let receipts_slice = match kernel_receipts_slice(&result.output) {
            Some(slice) => slice,
            None => return TestOutcome::Failed("kernel receipts not in dump".to_string()),
        };
        let receipts = match TransactionReceipt::decode_list(receipts_slice) {
            Some(receipts) => receipts,
            None => return TestOutcome::Failed("failed to decode receipts".to_string()),
        };
        let receipt = match receipts.last() {
            Some(receipt) => receipt,
            None => return TestOutcome::Failed("missing transaction receipt".to_string()),
        };
        let expected = match expected_for(case.name.as_str()) {
            Some(expected) => expected,
            None => {
                return TestOutcome::Failed(format!("missing expected result for {}", case.name));
            }
        };
        let success = receipt.result.success;
        let error_code = receipt.result.error_code;
        let data_len = receipt.result.data_len;
        let data = receipt.result.data;
        if success != expected.success {
            return TestOutcome::Failed(format!(
                "expected success={}, got {}",
                expected.success, success
            ));
        }
        if error_code != expected.error_code {
            return TestOutcome::Failed(format!(
                "expected error_code={}, got {}",
                expected.error_code, error_code
            ));
        }
        let data_len = data_len as usize;
        let actual = &data[..data_len.min(data.len())];
        if actual != expected.data.as_slice() {
            return TestOutcome::Failed(format!(
                "expected data {:?}, got {:?}",
                expected.data, actual
            ));
        }
        TestOutcome::Passed
    }
}

#[test]
fn examples_tests() {
    build_kernel().expect("failed to build kernel");
    build_examples().expect("failed to build example programs");

    let target_dir = kernel_elf_dir();
    let state_bytes = test_state_bytes();
    let example_cases = all_example_cases().expect("failed to build example bundles");
    let code_sizes = example_cases
        .iter()
        .map(|case| (case.name.to_string(), bundle_code_size(&case.bundle)))
        .collect::<HashMap<_, _>>();
    let cases = example_cases
        .into_iter()
        .map(|case| {
            println!("Running example: {} - {}", case.name, case.description);
            TestCase {
                name: case.name.to_string(),
                kind: TestKind::Smoke,
                elf: target_dir.join("kernel.elf"),
                options: RunOptions {
                    timeout_ms: None,
                    vm_memory_size: None,
                    verbose: false,
                    input: vec![case.bundle.encode(), state_bytes.clone()],
                },
            }
        })
        .collect::<Vec<_>>();

    let evaluator = ExampleEvaluator;
    let suite = Suite {
        name: "examples_tests".to_string(),
        cases,
        evaluator: &evaluator,
    };

    let runner = AvmRunner::new();
    let reports = suite.run(&runner);

    for report in &reports {
        if !report.stdout.is_empty() {
            println!("--- {} stdout ---\n{}", report.name, report.stdout);
        }
        if !report.stderr.is_empty() {
            eprintln!("--- {} stderr ---\n{}", report.name, report.stderr);
        }
    }

    print_summary(&reports, &code_sizes);

    let failures: Vec<_> = reports
        .iter()
        .filter(|report| matches!(report.outcome, TestOutcome::Failed(_)))
        .collect();

    if !failures.is_empty() {
        let mut details = String::new();
        for report in failures {
            if let TestOutcome::Failed(detail) = &report.outcome {
                details.push_str(&format!("{}: {}\n", report.name, detail));
            }
        }
        panic!("example test failures:\n{details}");
    }
}

fn print_summary(reports: &[a_tests::TestReport], code_sizes: &HashMap<String, u64>) {
    let total_tests = reports.len();
    let passed = reports
        .iter()
        .filter(|report| matches!(report.outcome, TestOutcome::Passed))
        .count();
    let failed = reports
        .iter()
        .filter(|report| matches!(report.outcome, TestOutcome::Failed(_)))
        .count();
    let skipped = reports
        .iter()
        .filter(|report| matches!(report.outcome, TestOutcome::Skipped(_)))
        .count();
    let instruction_count: u64 = reports.iter().map(|report| report.instruction_count).sum();
    let code_size_bytes: u64 = code_sizes.values().sum();

    println!("\n=== examples_tests summary ===");
    println!(
        "{:<32} {:<7} {:>16} {:>10} {:>12} {:>12} {:>10}",
        "Test", "Result", "Instructions", "Time(ms)", "Stack(B)", "Heap(B)", "Code(B)"
    );
    println!(
        "{:-<32} {:-<7} {:-<16} {:-<10} {:-<12} {:-<12} {:-<10}",
        "", "", "", "", "", "", ""
    );
    for report in reports {
        let result = match report.outcome {
            TestOutcome::Passed => "passed",
            TestOutcome::Failed(_) => "failed",
            TestOutcome::Skipped(_) => "skipped",
        };
        let instruction_count = format_u64(report.instruction_count);
        let duration_ms = format_u128(report.duration_ms);
        let stack_used = format_u64(report.stack_used_bytes);
        let heap_used = format_u64(report.heap_used_bytes);
        let code_size = format_u64(
            code_sizes
                .get(&report.name)
                .copied()
                .unwrap_or(report.code_size_bytes),
        );
        println!(
            "{:<32} {:<7} {:>16} {:>10} {:>12} {:>12} {:>10}",
            report.name, result, instruction_count, duration_ms, stack_used, heap_used, code_size
        );
    }
    println!(
        "{:-<32} {:-<7} {:-<16} {:-<10} {:-<12} {:-<12} {:-<10}",
        "", "", "", "", "", "", ""
    );
    let instruction_count = format_u64(instruction_count);
    let code_size_bytes = format_u64(code_size_bytes);
    println!(
        "{:<32} {:<7} {:>16} {:>10} {:>12} {:>12} {:>10}",
        "Total",
        format!("{passed}/{failed}/{skipped}/{total_tests}"),
        instruction_count,
        "",
        "",
        "",
        code_size_bytes
    );
}

fn bundle_code_size(bundle: &types::transaction::TransactionBundle) -> u64 {
    bundle
        .transactions
        .iter()
        .filter(|tx| matches!(tx.tx_type, TransactionType::CreateAccount))
        .map(|tx| tx.data.len() as u64)
        .sum()
}

fn format_u64(value: u64) -> String {
    format_number(value.to_string())
}

fn format_u128(value: u128) -> String {
    format_number(value.to_string())
}

fn format_number(mut digits: String) -> String {
    let mut out = String::new();
    let mut count = 0;
    while let Some(ch) = digits.pop() {
        if count == 3 {
            out.push(',');
            count = 0;
        }
        out.push(ch);
        count += 1;
    }
    out.chars().rev().collect()
}

fn kernel_elf_dir() -> PathBuf {
    std::env::var("KERNEL_ELF_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("crates/bootloader/bin"))
}

fn build_kernel() -> Result<(), String> {
    let status = std::process::Command::new("make")
        .args(["kernel"])
        .current_dir(workspace_root())
        .status()
        .map_err(|e| format!("failed to spawn kernel make: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("kernel build failed with status: {status}"))
    }
}

fn build_examples() -> Result<(), String> {
    let status = std::process::Command::new("make")
        .args(["-C", "crates/examples"])
        .current_dir(workspace_root())
        .status()
        .map_err(|e| format!("failed to spawn examples make: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("examples build failed with status: {status}"))
    }
}

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(PathBuf::from)
        .expect("missing workspace root")
}

fn kernel_receipts_slice(dump: &[u8]) -> Option<&[u8]> {
    if dump.len() < 16 {
        return None;
    }
    let receipts_ptr = u32::from_le_bytes(dump[0..4].try_into().ok()?);
    let receipts_len = u32::from_le_bytes(dump[4..8].try_into().ok()?);
    if receipts_ptr == 0 || receipts_len == 0 {
        return None;
    }
    let base = types::kernel_result::KERNEL_RESULT_ADDR;
    let start = receipts_ptr.checked_sub(base)? as usize;
    let end = start.checked_add(receipts_len as usize)?;
    if end > dump.len() {
        return None;
    }
    Some(&dump[start..end])
}
