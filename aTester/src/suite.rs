use std::path::PathBuf;

use crate::arch::{ArchRunner, RunResult};
use crate::types::{ElfTarget, RunOptions, TestOutcome};

#[derive(Debug, Clone)]
pub enum TestKind {
    Smoke,
    OutputMatch,
    InstructionTrace,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub kind: TestKind,
    pub elf: PathBuf,
    pub options: RunOptions,
}

#[derive(Debug, Clone)]
pub struct TestReport {
    pub name: String,
    pub outcome: TestOutcome,
    pub runner: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub instruction_count: u64,
    pub duration_ms: u128,
    pub stack_used_bytes: u64,
    pub heap_used_bytes: u64,
    pub code_size_bytes: u64,
    pub jit_stats: Option<vm::jit::JitStats>,
}

pub trait TestEvaluator {
    fn evaluate(&self, case: &TestCase, result: &RunResult) -> TestOutcome;
}

pub struct Suite<'a> {
    pub name: String,
    pub cases: Vec<TestCase>,
    pub evaluator: &'a dyn TestEvaluator,
}

impl<'a> Suite<'a> {
    pub fn run(&self, runner: &dyn ArchRunner) -> Vec<TestReport> {
        let mut reports = Vec::new();
        for case in &self.cases {
            let elf = ElfTarget {
                path: case.elf.clone(),
            };
            let start = std::time::Instant::now();
            let (
                outcome,
                exit_code,
                stdout,
                stderr,
                instruction_count,
                stack_used_bytes,
                heap_used_bytes,
                code_size_bytes,
                jit_stats,
            ) = match runner.run(&elf, &case.options) {
                Ok(result) => {
                    let outcome = self.evaluator.evaluate(case, &result);
                    (
                        outcome,
                        result.exit_code,
                        result.stdout,
                        result.stderr,
                        result.instruction_count,
                        result.stack_used_bytes,
                        result.heap_used_bytes,
                        result.code_size_bytes,
                        result.jit_stats,
                    )
                }
                Err(err) => (
                    TestOutcome::Failed(err.message.clone()),
                    -1,
                    String::new(),
                    err.message,
                    0,
                    0,
                    0,
                    0,
                    None,
                ),
            };
            let duration_ms = start.elapsed().as_millis();
            reports.push(TestReport {
                name: case.name.clone(),
                outcome,
                runner: runner.name().to_string(),
                exit_code,
                stdout,
                stderr,
                instruction_count,
                duration_ms,
                stack_used_bytes,
                heap_used_bytes,
                code_size_bytes,
                jit_stats,
            });
        }
        reports
    }
}
