use crate::cpu::CPU;
use crate::memory::{API, Memory};
use crate::metering::Metering;
use crate::registers::Register;
use std::rc::Rc;

/// Represents a complete RISC-V virtual machine.
///
/// EDUCATIONAL PURPOSE: This struct encapsulates all the components needed
/// to run a virtual machine: CPU and memory. It provides
/// a high-level interface for VM operations while hiding the complexity of
/// the underlying components.
///
/// VM ARCHITECTURE OVERVIEW:
/// - CPU: Executes RISC-V instructions
/// - Memory: Provides RAM for the running program
///
/// MEMORY MANAGEMENT: Uses Rc to share a trait-backed memory implementation,
/// allowing the VM to manage resources efficiently while maintaining Rust's
/// safety guarantees.
#[derive(Debug)]
pub struct VM {
    /// The CPU that executes RISC-V instructions
    pub cpu: CPU,

    /// Shared reference to the VM's memory (RAM)
    pub memory: Memory,

}

impl VM {
    /// Creates a new virtual machine with the specified memory.
    pub fn new(memory: Memory) -> Self {
        let mut cpu = CPU::new();
        cpu.regs[Register::Sp as usize] = memory.stack_top().as_u32();
        let satp = memory.satp();
        cpu.set_satp(&memory, satp);
        Self { cpu, memory }
    }

    /// Installs a metering implementation on the underlying CPU.
    pub fn set_metering(&mut self, metering: Box<dyn Metering>) {
        self.cpu.set_metering(metering);
    }
    pub fn set_reg_u32(&mut self, reg: Register, data: u32) {
        self.cpu.regs[reg as usize] = data;
    }

    pub fn memory_api(&self) -> Rc<dyn API> {
        self.memory.clone() as Rc<dyn API>
    }

    /// Dumps the entire memory contents for debugging.
    ///
    /// EDUCATIONAL PURPOSE: This demonstrates memory inspection tools that
    /// are essential for debugging VM programs. It shows both hex and ASCII
    /// representations of memory contents.
    ///
    /// DEBUGGING: Memory dumps are crucial for understanding what's happening
    /// when programs don't work as expected. They show the actual data in memory.
    pub fn dump_all_memory(&self) {
        self.dump_memory(0, self.memory.mem().len());
    }

    /// Dumps a specific range of memory for debugging.
    ///
    /// EDUCATIONAL PURPOSE: This function provides a detailed view of memory
    /// contents, showing both hexadecimal and ASCII representations. This is
    /// similar to tools like 'hexdump' or 'xxd' in Unix systems.
    ///
    /// OUTPUT FORMAT:
    /// - Address in hexadecimal
    /// - 16 bytes of data in hex format
    /// - ASCII representation (printable characters only)
    /// - Heap pointer location
    ///
    /// MEMORY LAYOUT: The output shows how memory is organized, including
    /// where the heap pointer is and what data is stored where.
    pub fn dump_memory(&self, start: usize, end: usize) {
        let borrowed_memory = self.memory.as_ref();

        // EDUCATIONAL: Validate memory range to prevent errors
        assert!(start < end, "invalid memory range");
        assert!(end <= borrowed_memory.mem().len(), "range out of bounds");

        println!("--- Memory Dump ---");

        // EDUCATIONAL: Display memory in 16-byte lines
        for addr in (start..end).step_by(16) {
            let line = &borrowed_memory.mem()[addr..end.min(addr + 16)];

            // EDUCATIONAL: Convert bytes to hex strings
            let hex: Vec<String> = line.iter().map(|b| format!("{:02x}", b)).collect();
            let hex_str = hex.join(" ");

            // EDUCATIONAL: Convert bytes to ASCII (printable characters only)
            let ascii: String = line
                .iter()
                .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
                .collect();

            println!("{:08x}  {:<47}  |{}|", addr, hex_str, ascii);
        }
        println!("-------------------");
    }

    /// Dumps the current state of all CPU registers for debugging.
    ///
    /// EDUCATIONAL PURPOSE: This demonstrates register inspection, which is
    /// essential for understanding program state and debugging issues.
    ///
    /// RISC-V REGISTER CONVENTIONS: The output shows both register numbers
    /// and their ABI (Application Binary Interface) names, which helps
    /// understand how registers are used in RISC-V programs.
    ///
    /// REGISTER USAGE:
    /// - x0 (zero): Always zero
    /// - x1 (ra): Return address
    /// - x2 (sp): Stack pointer
    /// - x10-x17 (a0-a7): Function arguments
    /// - x5-x7, x28-x31 (t0-t6): Temporary registers
    /// - x8, x9, x18-x27 (s0-s11): Saved registers
    pub fn dump_registers(&self) {
        println!("--- Register Dump ---");

        // EDUCATIONAL: RISC-V ABI register names for easier understanding
        const ABI_NAMES: [&str; 32] = [
            "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3",
            "a4", "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
            "t3", "t4", "t5", "t6",
        ];

        // EDUCATIONAL: Display each register with its name and value
        for i in 0..32 {
            let name = ABI_NAMES[i];
            let val = self.cpu.regs[i];
            println!("x{:02} ({:<4}) = 0x{:08x} ({})", i, name, val, val);
        }

        // EDUCATIONAL: Show program counter separately
        println!("pc           = 0x{:08x}", self.cpu.pc);
        println!("------------------------");
    }

    /// Starts program execution without initializing registers or setting up state.
    ///
    /// EDUCATIONAL PURPOSE: This is the main execution loop of the VM. It
    /// continuously fetches, decodes, and executes instructions until the
    /// program halts or encounters an error.
    ///
    /// EXECUTION LOOP: This implements the classic fetch-decode-execute cycle
    /// that all CPUs follow. The loop continues until the CPU returns false,
    /// indicating that execution should stop.
    ///
    /// ASSUMPTIONS: This function assumes the VM is already properly configured
    /// with code loaded and registers set up. For a complete VM, you'd typically
    /// call this after setting up the initial state.
    pub fn raw_run(&mut self) {
        // EDUCATIONAL: Main execution loop - fetch, decode, execute
        while self.cpu.step(Rc::clone(&self.memory)) {}
    }
}
