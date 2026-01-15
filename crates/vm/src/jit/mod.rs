// JIT module entry point and orchestration.
// This keeps the runtime-facing API small while delegating heavy lifting
// (trace building, compilation, helpers) to submodules.
mod access;
mod compiler;
mod helpers;
mod trace;

use crate::cpu::{CPU, PrivilegeMode};
use crate::memory::Memory;
pub(crate) use access::JitAccess;
use compiler::JitCompiler;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use trace::Trace;

// JIT function ABI: returns 0 on halt, 1 on continue.
pub(super) type JitFn = unsafe extern "C" fn(*mut CPU, *const Memory) -> u32;

#[derive(Debug)]
pub(super) struct JitEntry {
    func: JitFn,
}

#[derive(Clone, Copy, Debug, Eq)]
struct CacheKey {
    root: usize,
    pc: u32,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.pc == other.pc
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root.hash(state);
        self.pc.hash(state);
    }
}

/// Minimal JIT scaffold to track hot PCs and dispatch compiled traces.
///
/// The JIT is intentionally conservative:
/// - Disabled by default.
/// - Builds short traces of supported instructions only.
/// - Falls back to the interpreter on any unsupported path.
#[derive(Debug, Default)]
pub struct Jit {
    enabled: bool,
    hot_threshold: u32,
    hits: HashMap<CacheKey, u32>,
    cache: HashMap<CacheKey, JitEntry>,
    failed: HashSet<CacheKey>,
    compiler: Option<JitCompiler>,
}

impl Jit {
    /// Create a new JIT controller; `enabled` gates all compilation/dispatch.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            hot_threshold: 1_000,
            hits: HashMap::new(),
            cache: HashMap::new(),
            failed: HashSet::new(),
            compiler: None,
        }
    }

    /// Toggle JIT execution.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Configure the number of hits before a PC is considered "hot".
    pub fn set_hot_threshold(&mut self, threshold: u32) {
        self.hot_threshold = threshold.max(1);
    }

    /// Attempt to execute JIT-compiled code at the current PC.
    ///
    /// Returns:
    /// - Some(true/false) if a compiled entry was run.
    /// - None if the interpreter should continue.
    pub fn maybe_execute(&mut self, cpu: &mut CPU, memory: &Memory) -> Option<bool> {
        if !self.enabled || cpu.verbose || cpu.priv_mode != PrivilegeMode::User {
            return None;
        }

        let pc = cpu.pc;
        let key = CacheKey {
            root: memory.current_root(),
            pc,
        };
        if self.failed.contains(&key) {
            return None;
        }
        if let Some(entry) = self.cache.get(&key) {
            return Some(run_entry(entry, cpu, memory));
        }

        if !self.record_hit(key) {
            return None;
        }

        let trace = Trace::build(pc, memory)?;
        let compiler = self.compiler.get_or_insert_with(JitCompiler::new);
        let entry = match compiler.compile_trace(&trace) {
            Some(entry) => entry,
            None => {
                self.failed.insert(key);
                return None;
            }
        };
        self.cache.insert(key, entry);
        self.cache
            .get(&key)
            .map(|entry| run_entry(entry, cpu, memory))
    }

    /// Track execution counts and return true once a PC crosses the threshold.
    fn record_hit(&mut self, key: CacheKey) -> bool {
        let count = self.hits.entry(key).or_insert(0);
        *count = count.saturating_add(1);
        *count >= self.hot_threshold
    }
}

/// Execute a compiled JIT entry and normalize the return value to bool.
fn run_entry(entry: &JitEntry, cpu: &mut CPU, memory: &Memory) -> bool {
    let result = unsafe { (entry.func)(cpu as *mut CPU, memory as *const Memory) };
    result != 0
}
