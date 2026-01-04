use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ElfTarget {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub timeout_ms: Option<u64>,
    pub vm_memory_size: Option<usize>,
    pub verbose: bool,
    pub input: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum TestOutcome {
    Passed,
    Failed(String),
    Skipped(String),
}
