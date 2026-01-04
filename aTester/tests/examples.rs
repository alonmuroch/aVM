use std::path::{Path, PathBuf};

use a_tests::{AvmRunner, RunOptions, Suite, TestCase, TestEvaluator, TestKind, TestOutcome};
use types::TransactionReceipt;

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
                return TestOutcome::Failed(format!(
                    "missing expected result for {}",
                    case.name
                ))
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
    let cases = all_example_cases()
        .expect("failed to build example bundles")
        .into_iter()
        .map(|case| TestCase {
            name: case.name.to_string(),
            kind: TestKind::Smoke,
            elf: target_dir.join("kernel.elf"),
            options: RunOptions {
                timeout_ms: None,
                vm_memory_size: None,
                verbose: false,
                input: vec![case.bundle.encode(), state_bytes.clone()],
            },
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
        panic!("example test failures:\n{}", details);
    }
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
