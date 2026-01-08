use std::fs;
use std::path::{Path, PathBuf};

use a_tests::{AvmRunner, RunOptions, Suite, TestCase, TestEvaluator, TestKind, TestOutcome};

struct ExitCodeEvaluator;

impl TestEvaluator for ExitCodeEvaluator {
    fn evaluate(&self, case: &TestCase, result: &a_tests::RunResult) -> TestOutcome {
        match read_test_results_from_output(&result.output) {
            Ok(results) => {
                if results.status == 0 {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed(format!(
                        "{} failed with detail {}",
                        case.name, results.detail
                    ))
                }
            }
            Err(err) => TestOutcome::Failed(format!("{} failed: {}", case.name, err)),
        }
    }
}

#[test]
fn kernel_tests() {
    build_kernel().expect("failed to build kernel test bins");
    let bins = kernel_bins().expect("failed to discover kernel bins");
    if bins.is_empty() {
        panic!("no kernel test bins found");
    }

    let target_dir = kernel_elf_dir();
    let cases = bins
        .into_iter()
        .map(|name| TestCase {
            name: name.clone(),
            kind: TestKind::Smoke,
            elf: target_dir.join(format!("{name}.elf")),
            options: RunOptions {
                timeout_ms: None,
                vm_memory_size: None,
                verbose: false,
                input: Vec::new(),
            },
        })
        .collect::<Vec<_>>();

    let evaluator = ExitCodeEvaluator;
    let suite = Suite {
        name: "kernel_tests".to_string(),
        cases,
        evaluator: &evaluator,
    };

    let runner = AvmRunner::new();
    for case in &suite.cases {
        println!("running kernel test: {}", case.name);
    }
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
        panic!("kernel test failures:\n{details}");
    }
}

struct TestResults {
    status: u32,
    detail: u32,
}

fn read_test_results_from_output(output: &[u8]) -> Result<TestResults, String> {
    if output.len() < 8 {
        return Err("missing test results output".to_string());
    }
    let status = u32::from_le_bytes(output[0..4].try_into().unwrap());
    let detail = u32::from_le_bytes(output[4..8].try_into().unwrap());
    Ok(TestResults { status, detail })
}

fn kernel_bins() -> Result<Vec<String>, String> {
    let manifest_path = workspace_root().join("crates/kernel/Cargo.toml");
    let contents = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read kernel Cargo.toml: {e}"))?;

    let mut bins = Vec::new();
    let mut current_name: Option<String> = None;

    for line in contents.lines() {
        let line = line.trim();
        if line == "[[bin]]" {
            if let Some(name) = current_name.take()
                && name != "kernel"
            {
                bins.push(name);
            }
            continue;
        }
        if let Some(name) = line.strip_prefix("name = ") {
            let name = name.trim().trim_matches('"').to_string();
            current_name = Some(name);
        }
    }

    if let Some(name) = current_name
        && name != "kernel"
    {
        bins.push(name);
    }

    if bins.is_empty() {
        return Err("no [[bin]] entries found".to_string());
    }

    Ok(bins)
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

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(PathBuf::from)
        .expect("missing workspace root")
}
