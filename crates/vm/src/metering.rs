use crate::instruction::Instruction;

/// Outcome returned by metering hooks to indicate whether execution should continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterResult {
    Continue,
    Halt,
}

/// Identifies the type of memory access being charged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessKind {
    Load,
    Store,
    Atomic,
    ReservationLoad,
    ReservationStore,
}

/// Pluggable metering interface. Implementors can account for gas or other resource
/// usage without changing the VM core. All methods default to no-op/continue.
pub trait Metering: std::fmt::Debug {
    /// Called on instruction execution.
    fn on_instruction(&mut self, _pc: u32, _instr: &Instruction, _size: u8) -> MeterResult {
        MeterResult::Continue
    }

    /// Called for each memory access with its width.
    fn on_memory_access(
        &mut self,
        _kind: MemoryAccessKind,
        _addr: usize,
        _bytes: usize,
    ) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when a syscall is dispatched (before handler-specific work).
    fn on_syscall(&mut self, _call_id: u32, _args: &[u32; 6]) -> MeterResult {
        MeterResult::Continue
    }

    /// Called for syscall-specific data-dependent charges (payload copies, etc.).
    fn on_syscall_data(&mut self, _call_id: u32, _bytes: usize) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when a general-purpose register is read.
    fn on_register_read(&mut self, _reg: usize) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when a general-purpose register is written.
    fn on_register_write(&mut self, _reg: usize) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when the program counter is updated.
    fn on_pc_update(&mut self, _old_pc: u32, _new_pc: u32) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when guest requests heap allocation.
    fn on_alloc(&mut self, _bytes: usize) -> MeterResult {
        MeterResult::Continue
    }

    /// Called when guest requests a program call; input_bytes covers calldata size.
    fn on_call(&mut self, _input_bytes: usize) -> MeterResult {
        MeterResult::Continue
    }
}

/// Default metering that performs no accounting.
#[derive(Debug, Default)]
pub struct NoopMeter;

impl Metering for NoopMeter {}
