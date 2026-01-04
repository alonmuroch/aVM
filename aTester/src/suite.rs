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
            let (outcome, exit_code, stdout, stderr) = match runner.run(&elf, &case.options) {
                Ok(result) => {
                    let outcome = self.evaluator.evaluate(case, &result);
                    (outcome, result.exit_code, result.stdout, result.stderr)
                }
                Err(err) => (TestOutcome::Failed(err.message.clone()), -1, String::new(), err.message),
            };
            reports.push(TestReport {
                name: case.name.clone(),
                outcome,
                runner: runner.name().to_string(),
                exit_code,
                stdout,
                stderr,
            });
        }
        reports
    }
}
