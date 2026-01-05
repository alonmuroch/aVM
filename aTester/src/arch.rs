use std::fmt;

use crate::types::{ElfTarget, RunOptions};

#[derive(Debug, Clone)]
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub output: Vec<u8>,
}

#[derive(Debug)]
pub struct RunError {
    pub message: String,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RunError {}

pub trait ArchRunner {
    fn name(&self) -> &str;
    fn run(&self, elf: &ElfTarget, options: &RunOptions) -> Result<RunResult, RunError>;
}

pub struct ArchRegistry {
    runners: Vec<Box<dyn ArchRunner>>,
}

impl ArchRegistry {
    pub fn new() -> Self {
        Self {
            runners: Vec::new(),
        }
    }

    pub fn register(&mut self, runner: Box<dyn ArchRunner>) {
        self.runners.push(runner);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ArchRunner> {
        self.runners
            .iter()
            .find(|runner| runner.name() == name)
            .map(|runner| runner.as_ref())
    }
}
