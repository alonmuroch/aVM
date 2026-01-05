use crate::decoder::{decode_compressed, decode_full};
use crate::instruction::Instruction;
use crate::memory::{Memory, VirtualAddress};
use crate::metering::{MemoryAccessKind, MeterResult, Metering, NoopMeter};
use core::cell::RefCell;
use core::fmt::Write;
use std::collections::HashMap;
use std::rc::Rc;
#[path = "exe.rs"]
mod exec;

pub const CSR_SATP: u16 = 0x180;
pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_STVEC: u16 = 0x105;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_MEPC: u16 = 0x341;
pub const CSR_MTVEC: u16 = 0x305;
pub const CSR_MCAUSE: u16 = 0x342;
pub const CSR_MTVAL: u16 = 0x343;
const SCAUSE_ECALL_FROM_U: u32 = 8;
const SCAUSE_ECALL_FROM_S: u32 = 9;
const SCAUSE_ECALL_FROM_M: u32 = 11;
const SCAUSE_BREAKPOINT: u32 = 3;
const SSTATUS_SPP: u32 = 1 << 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivilegeMode {
    User,
    Supervisor,
    Machine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapMode {
    Supervisor,
    Machine,
}

/// Represents the Central Processing Unit (CPU) of our RISC-V virtual machine.
/// 
/// EDUCATIONAL PURPOSE: This struct models the core components of a real CPU:
/// - Program Counter (PC): Points to the next instruction to execute
/// - Registers: Fast storage locations for temporary data
/// - Verbose flag: For debugging and educational purposes
/// 
/// RISC-V ARCHITECTURE NOTES:
/// - RISC-V is a Reduced Instruction Set Computer (RISC) architecture
/// - It has 32 general-purpose registers (x0-x31)
/// - Register x0 is always hardwired to zero
/// - The PC is separate from the general registers
/// 
/// REAL CPU COMPARISON: This is a simplified model of a real CPU. In actual
/// hardware, a CPU has many more components:
/// - Arithmetic Logic Unit (ALU) for mathematical operations
/// - Control Unit for instruction decoding and control flow
/// - Cache memory for faster data access
/// - Pipeline stages for parallel instruction processing
/// - Memory Management Unit (MMU) for virtual memory
/// 
/// VIRTUAL MACHINE CONTEXT: In a VM, we simulate these components in software.
/// This allows us to run programs written for one architecture (RISC-V) on
/// different hardware (like x86 or ARM). The VM provides an abstraction layer
/// that makes the underlying hardware details transparent to the running program.
///
/// MEMORY MANAGEMENT: We use Rc-backed trait objects for shared memory, which
/// allows the CPU to read/write memory while maintaining Rust's safety guarantees.
///
/// PERFORMANCE CONSIDERATIONS: This is an interpretive VM, meaning each
/// instruction is decoded and executed one at a time. Real CPUs use techniques
/// like pipelining, out-of-order execution, and just-in-time compilation to
/// achieve much higher performance. However, this simple approach is perfect
/// for learning and understanding how CPUs work at a fundamental level.
pub struct CPU {
    /// Program Counter - points to the next instruction to execute
    /// EDUCATIONAL: In real CPUs, this is a special register that automatically
    /// increments after each instruction (unless the instruction changes it)
    pub pc: u32,
    
    /// General-purpose registers (x0-x31)
    /// EDUCATIONAL: Registers are the fastest storage in a CPU, much faster than
    /// main memory. RISC-V has 32 registers, each holding a 32-bit value.
    /// Register x0 is always zero, x1 is the return address, x2 is the stack pointer.
    pub regs: [u32; 32],

    /// Enable verbose logging for debugging and educational purposes
    /// EDUCATIONAL: This helps students understand what the CPU is doing
    /// by printing each instruction as it executes
    pub verbose: bool,
    /// Reservation address for LR/SC atomic operations
    /// EDUCATIONAL: This implements the Load-Reserved/Store-Conditional
    /// mechanism for atomic memory operations in RISC-V
    pub reservation_addr: Option<VirtualAddress>,
    
    /// Optional writer for verbose output
    /// If None, uses println! to console
    pub verbose_writer: Option<Rc<RefCell<dyn Write>>>,

    /// Pluggable metering implementation (gas, resource accounting, etc.)
    pub metering: Box<dyn Metering>,

    /// Minimal CSR storage for CSR instructions
    pub csrs: HashMap<u16, u32>,

    /// Current privilege mode (minimal U/S support).
    pub priv_mode: PrivilegeMode,
}

impl std::fmt::Debug for CPU {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CPU")
            .field("pc", &self.pc)
            .field("regs", &self.regs)
            .field("verbose", &self.verbose)
            .field("reservation_addr", &self.reservation_addr)
            .field(
                "verbose_writer",
                &self.verbose_writer.as_ref().map(|_| "Some(<writer>)"),
            )
            .field("metering", &"<dyn Metering>")
            .finish()
    }
}

impl CPU {
    /// Creates a new CPU instance with default values.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates CPU initialization. In real systems,
    /// the CPU would be reset to a known state when powered on.
    /// 
    /// INITIALIZATION DETAILS:
    /// - PC starts at 0 (first instruction)
    /// - All registers start at 0 (except x0 which is always 0)
    /// - Verbose logging is disabled by default
    pub fn new() -> Self {
        Self::with_metering(Box::new(NoopMeter::default()))
    }

    /// Creates a new CPU instance with a custom metering implementation.
    pub fn with_metering(
        metering: Box<dyn Metering>,
    ) -> Self {
        Self {
            pc: 0,
            regs: [0; 32],
            verbose: false,
            reservation_addr: None,
            verbose_writer: None,
            metering,
            csrs: HashMap::new(),
            priv_mode: PrivilegeMode::Supervisor,
        }
    }
    
    /// Sets a writer for verbose output
    pub fn set_verbose_writer(&mut self, writer: Rc<RefCell<dyn Write>>) {
        self.verbose_writer = Some(writer);
    }

    /// Swap in a new metering implementation.
    pub fn set_metering(&mut self, metering: Box<dyn Metering>) {
        self.metering = metering;
    }
    
    /// Helper method to log output
    /// Only logs if verbose is true and self.verbose is enabled
    fn log(&self, message: &str, verbose: bool) {
        // Only log if this is not a verbose message, or if verbose logging is enabled
        if verbose && !self.verbose {
            return;
        }
        
        match &self.verbose_writer {
            Some(writer) => {
                let _ = write!(writer.borrow_mut(), "{}\n", message);
            }
            None => {
                println!("{}", message);
            }
        }
    }

    fn can_continue(result: MeterResult) -> bool {
        matches!(result, MeterResult::Continue)
    }

    fn read_csr(&mut self, csr: u16) -> Option<u32> {
        if !Self::can_continue(self.metering.on_pc_update(self.pc, self.pc)) {
            return None;
        }
        // Provide simple defaults for common CSRs; fall back to stored values or zero.
        Some(match csr {
            0xF14 => *self.csrs.get(&csr).unwrap_or(&0), // mhartid
            0xF11 | 0xF12 | 0xF13 => *self.csrs.get(&csr).unwrap_or(&0), // mvendorid/marchid/mimpid
            0x301 => *self.csrs.get(&csr).unwrap_or(&0), // misa
            0x300 => *self.csrs.get(&csr).unwrap_or(&0), // mstatus
            CSR_SSTATUS => *self.csrs.get(&csr).unwrap_or(&0),
            _ => *self.csrs.get(&csr).unwrap_or(&0),
        })
    }

    fn write_csr(&mut self, csr: u16, value: u32) -> bool {
        if !Self::can_continue(self.metering.on_pc_update(self.pc, self.pc)) {
            return false;
        }
        self.csrs.insert(csr, value);
        true
    }

    pub fn set_satp(&mut self, memory: &Memory, value: u32) -> bool {
        memory.set_satp(value);
        self.write_csr(CSR_SATP, value)
    }

    fn set_sstatus_spp(&mut self, prev: PrivilegeMode) {
        let mut sstatus = self.read_csr(CSR_SSTATUS).unwrap_or(0);
        match prev {
            PrivilegeMode::User => sstatus &= !SSTATUS_SPP,
            PrivilegeMode::Supervisor => sstatus |= SSTATUS_SPP,
            PrivilegeMode::Machine => sstatus |= SSTATUS_SPP,
        }
        let _ = self.write_csr(CSR_SSTATUS, sstatus);
    }

    fn take_sstatus_spp(&mut self) -> PrivilegeMode {
        let mut sstatus = self.read_csr(CSR_SSTATUS).unwrap_or(0);
        let prev = if sstatus & SSTATUS_SPP != 0 {
            PrivilegeMode::Supervisor
        } else {
            PrivilegeMode::User
        };
        sstatus &= !SSTATUS_SPP;
        let _ = self.write_csr(CSR_SSTATUS, sstatus);
        prev
    }

    fn ecall_cause(&self) -> u32 {
        match self.priv_mode {
            PrivilegeMode::User => SCAUSE_ECALL_FROM_U,
            PrivilegeMode::Supervisor => SCAUSE_ECALL_FROM_S,
            PrivilegeMode::Machine => SCAUSE_ECALL_FROM_M,
        }
    }

    fn trap_to_vector(
        &mut self,
        mode: TrapMode,
        cause: u32,
        trap_value: u32,
        _syscall_id: Option<u32>,
    ) -> bool {
        match mode {
            TrapMode::Machine => {
                if !self.write_csr(CSR_MEPC, self.pc) {
                    panic!("trap_to_vector: failed to write mepc");
                }
                if !self.write_csr(CSR_MCAUSE, cause) {
                    panic!("trap_to_vector: failed to write mcause");
                }
                if !self.write_csr(CSR_MTVAL, trap_value) {
                    panic!("trap_to_vector: failed to write mtval");
                }
                let mtvec = match self.read_csr(CSR_MTVEC) {
                    Some(val) => val & !0x3,
                    None => return false,
                };
                self.priv_mode = PrivilegeMode::Machine;
                self.set_pc(mtvec)
            }
            TrapMode::Supervisor => {
                if !self.write_csr(CSR_SEPC, self.pc) {
                    panic!("trap_to_vector: failed to write sepc");
                }
                if !self.write_csr(CSR_SCAUSE, cause) {
                    panic!("trap_to_vector: failed to write scause");
                }
                if !self.write_csr(CSR_STVAL, trap_value) {
                    panic!("trap_to_vector: failed to write stval");
                }
                let stvec = match self.read_csr(CSR_STVEC) {
                    Some(val) => val & !0x3,
                    None => return false,
                };
                self.set_sstatus_spp(self.priv_mode);
                self.priv_mode = PrivilegeMode::Supervisor;
                self.set_pc(stvec)
            }
        }
    }

    fn has_trap_vector(&self) -> Option<TrapMode> {
        match self.priv_mode {
            PrivilegeMode::Machine => {
                if self.csrs.contains_key(&CSR_MTVEC) {
                    Some(TrapMode::Machine)
                } else if self.csrs.contains_key(&CSR_STVEC) {
                    Some(TrapMode::Supervisor)
                } else {
                    None
                }
            }
            PrivilegeMode::Supervisor | PrivilegeMode::User => {
                if self.csrs.contains_key(&CSR_STVEC) {
                    Some(TrapMode::Supervisor)
                } else if self.csrs.contains_key(&CSR_MTVEC) {
                    Some(TrapMode::Machine)
                } else {
                    None
                }
            }
        }
    }

    /// Executes a single instruction cycle (fetch, decode, execute).
    /// 
    /// EDUCATIONAL PURPOSE: This is the heart of the CPU - the instruction cycle.
    /// Every CPU follows this basic pattern:
    /// 1. Fetch: Read the next instruction from memory
    /// 2. Decode: Figure out what the instruction does
    /// 3. Execute: Perform the operation
    /// 
    /// INSTRUCTION CYCLE DETAILS:
    /// - FETCH: The CPU reads the instruction from memory at the address
    ///   pointed to by the Program Counter (PC)
    /// - DECODE: The instruction is analyzed to determine what operation
    ///   to perform and what operands (registers, memory addresses) to use
    /// - EXECUTE: The actual operation is performed (arithmetic, memory access,
    ///   control flow, etc.)
    /// 
    /// ERROR HANDLING: If an invalid instruction is encountered, the CPU
    /// handles it gracefully by calling unknown_instruction() which provides
    /// debugging information and halts execution safely.
    /// 
    /// RETURN VALUE: Returns true if execution should continue, false to halt
    /// 
    /// MEMORY ACCESS: Uses shared references to memory to allow
    /// the CPU to read/write while maintaining Rust's safety guarantees.
    /// 
    /// REAL-WORLD ANALOGY: This is like a factory assembly line where each
    /// worker (instruction) performs a specific task. The conveyor belt (PC)
    /// moves to the next task automatically, unless a task specifically
    /// redirects the flow (like a branch or jump instruction).
    pub fn step(&mut self, memory: Memory) -> bool {
        // EDUCATIONAL: Step 1 - Fetch and decode the next instruction
        let instr = self.next_instruction(Rc::clone(&memory));
        
        // EDUCATIONAL: Step 2 - Execute the instruction or handle errors
        match instr {
            Some((instr, size)) => {
                // Valid instruction found - execute it
                self.run_instruction(instr, size, Rc::clone(&memory))
            }
            None => {
                // No valid instruction found - handle the error
                self.unknown_instruction(Rc::clone(&memory))
            }
        }
    }

    /// Executes a single instruction and updates the program counter.
    /// 
    /// EDUCATIONAL PURPOSE: This function demonstrates instruction execution
    /// and program counter management. Some instructions (like branches and jumps)
    /// modify the PC directly, while others just increment it.
    /// 
    /// PC MANAGEMENT: The PC is only incremented if the instruction didn't
    /// change it. This handles branches, jumps, and calls correctly.
    /// 
    /// PARAMETERS:
    /// - instr: The decoded instruction to execute
    /// - size: Size of the instruction in bytes (2 for compressed, 4 for full)
    /// - memory: Shared reference to memory for load/store operations
    fn run_instruction(
        &mut self,
        instr: Instruction,
        size: u8,
        memory: Memory,
    ) -> bool {
        // EDUCATIONAL: Debug output to help understand what's happening
        // Get the actual instruction bytes for debugging
        let pc_va = VirtualAddress(self.pc);
        let end_va = VirtualAddress(self.pc.wrapping_add(size as u32));
        if let Some(bytes) = memory.mem_slice(pc_va, end_va) {
            let hex_bytes = bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            self.log(
                &format!(
                    "PC = 0x{:08x}, Bytes = [{}], Instr = {}",
                    self.pc,
                    hex_bytes,
                    instr.pretty_print()
                ),
                true,
            );
        } else {
            self.log(
                &format!("PC = 0x{:08x}, Instr = {}", self.pc, instr.pretty_print()),
                true,
            );
        }

        if !Self::can_continue(self.metering.on_instruction(self.pc, &instr, size)) {
            return false;
        }

        // EDUCATIONAL: Remember the old PC to detect if the instruction changed it
        let old_pc = self.pc;

        // EDUCATIONAL: Execute the instruction
        let result = self.execute(instr.clone(), memory);
        if !result {
            self.log(
                &format!(
                    "Execution halted at PC=0x{:08x} on instr={}",
                    self.pc,
                    instr.pretty_print()
                ),
                false,
            );
        }

        // EDUCATIONAL: Only increment PC if the instruction didn't change it
        // This handles branches, jumps, and calls correctly
        if self.pc == old_pc {
            if !self.pc_add(size as u32) {
                return false;
            }
        }
        result
    }

    /// Handles unknown or invalid instructions.
    ///
    /// EDUCATIONAL PURPOSE: This demonstrates error handling in CPU design.
    /// When a CPU encounters an invalid instruction, it needs to handle it
    /// gracefully rather than crashing.
    ///
    /// DEBUGGING: This function provides detailed information about what
    /// went wrong, including the hex dump of the invalid bytes.
    ///
    /// RETURN VALUE: This method always panics and never returns.
    fn unknown_instruction(&mut self, memory: Memory) -> ! {
        // EDUCATIONAL: Try to read the invalid instruction bytes for debugging
        if let Some(slice_ref) = memory.mem_slice(VirtualAddress(self.pc), VirtualAddress(self.pc.wrapping_add(4)))
        {
            // EDUCATIONAL: Convert bytes to hex for human-readable debugging
            let hex_dump = slice_ref
                .iter()
                .map(|b| format!("{:02x}", b)) // still needs deref
                .collect::<Vec<_>>()
                .join(" ");

            panic!(
                "🚨 Unknown or invalid instruction at PC = 0x{:08x} (bytes: [{}])",
                self.pc, hex_dump
            );
        } else {
            panic!(
                "🚨 Unknown or invalid instruction at PC = 0x{:08x} (could not read memory)",
                self.pc
            );
        }
    }

    /// Fetches and decodes the next instruction from memory.
    ///
    /// EDUCATIONAL PURPOSE: This demonstrates the fetch and decode phases
    /// of the instruction cycle. It handles both regular (32-bit) and
    /// compressed (16-bit) RISC-V instructions.
    ///
    /// RISC-V COMPRESSED INSTRUCTIONS: RISC-V supports 16-bit compressed
    /// instructions to reduce code size. The bottom 2 bits determine if
    /// an instruction is compressed (not 0b11) or regular (0b11).
    ///
    /// RETURN VALUE: Returns Some((instruction, size)) if successful, None if invalid
    pub fn next_instruction(&mut self, memory: Memory) -> Option<(Instruction, u8)> {
        let pc = VirtualAddress(self.pc);

        // EDUCATIONAL: Read 4 bytes from memory (enough for any instruction)
        let bytes = memory.mem_slice(pc, VirtualAddress(self.pc.wrapping_add(4)))?;

        // EDUCATIONAL: Need at least 2 bytes for any instruction
        if bytes.len() < 2 {
            return None;
        }

        // EDUCATIONAL: Check if this is a compressed instruction
        // RISC-V compressed instructions have bottom 2 bits != 0b11
        let hword = u16::from_le_bytes([bytes[0], bytes[1]]);
        let is_compressed = (hword & 0b11) != 0b11;

        if is_compressed {
            // EDUCATIONAL: Decode 16-bit compressed instruction
            decode_compressed(hword).map(|inst| (inst, 2))
        } else if bytes.len() >= 4 {
            // EDUCATIONAL: Decode 32-bit regular instruction
            let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            decode_full(word).map(|inst| (inst, 4))
        } else {
            None
        }
    }

    /// Safely read a register with metering.
    fn read_reg(&mut self, reg: usize) -> Option<u32> {
        if !Self::can_continue(self.metering.on_register_read(reg)) {
            return None;
        }
        Some(self.regs[reg])
    }

    /// Safely write to a register, ignoring writes to x0 (which should always be 0).
    /// Returns false if metering halts execution.
    fn write_reg(&mut self, rd: usize, value: u32) -> bool {
        if rd != 0 {
            if !Self::can_continue(self.metering.on_register_write(rd)) {
                return false;
            }
            self.regs[rd] = value;
        }
        true
    }

    /// Add to the program counter with wrapping semantics and metering.
    fn pc_add(&mut self, delta: u32) -> bool {
        let old = self.pc;
        let new_pc = self.pc.wrapping_add(delta);
        if !Self::can_continue(self.metering.on_pc_update(old, new_pc)) {
            return false;
        }
        self.pc = new_pc;
        true
    }

    /// Add to the stack pointer (x2) with metering.
    fn sp_add(&mut self, delta: u32) -> bool {
        let sp = match self.read_reg(2) {
            Some(v) => v,
            None => return false,
        };
        self.write_reg(2, sp.wrapping_add(delta))
    }

    /// Set the program counter and meter the update.
    fn set_pc(&mut self, target: u32) -> bool {
        let old = self.pc;
        if !Self::can_continue(self.metering.on_pc_update(old, target)) {
            return false;
        }
        self.pc = target;
        true
    }
}
