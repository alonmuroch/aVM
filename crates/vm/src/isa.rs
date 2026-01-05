/// EDUCATIONAL: RISC-V Instruction Set Architecture (ISA) Opcodes
///
/// This enum defines the primary opcodes used in RISC-V instruction encoding.
/// Each opcode represents a category of instructions that share the same basic format.
///
/// RISC-V INSTRUCTION ENCODING:
/// - All RISC-V instructions are 32 bits (or 16 bits for compressed instructions)
/// - The bottom 7 bits contain the opcode, which determines the instruction format
/// - Different opcodes use different instruction formats (R, I, S, B, U, J)
///
/// INSTRUCTION FORMATS:
/// - R-type: Register operations (ADD, SUB, AND, OR, etc.)
/// - I-type: Immediate operations (ADDI, LW, JALR, etc.)
/// - S-type: Store operations (SW, SH, SB)
/// - B-type: Branch operations (BEQ, BNE, BLT, etc.)
/// - U-type: Upper immediate operations (LUI, AUIPC)
/// - J-type: Jump operations (JAL)
///
/// REAL-WORLD CONTEXT: In actual RISC-V processors, these opcodes are encoded
/// as binary values in the instruction word. The decoder uses these opcodes
/// to determine how to interpret the rest of the instruction bits.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum Opcode {
    /// LOAD (0x03): Load instructions - LW, LH, LB, LHU, LBU
    /// EDUCATIONAL: Load instructions read data from memory into registers.
    /// They use I-type format with: rd, rs1 (base address), and immediate offset.
    /// Examples: LW (load word), LH (load halfword), LB (load byte)
    Load = 0x03,

    /// STORE (0x23): Store instructions - SW, SH, SB
    /// EDUCATIONAL: Store instructions write data from registers to memory.
    /// They use S-type format with: rs1 (base address), rs2 (data), and immediate offset.
    /// Examples: SW (store word), SH (store halfword), SB (store byte)
    Store = 0x23,

    /// BRANCH (0x63): Branch instructions - BEQ, BNE, BLT, BGE, etc.
    /// EDUCATIONAL: Branch instructions perform conditional jumps based on register comparisons.
    /// They use B-type format with: rs1, rs2 (comparison operands), and immediate offset.
    /// Examples: BEQ (branch if equal), BNE (branch if not equal), BLT (branch if less than)
    Branch = 0x63,

    /// JAL (0x6F): Jump and Link - unconditional jump with return address
    /// EDUCATIONAL: JAL performs an unconditional jump and saves the return address.
    /// It uses J-type format with: rd (return address register), and immediate offset.
    /// Used for function calls and long-distance jumps.
    Jal = 0x6f,

    /// JALR (0x67): Jump and Link Register - indirect function calls
    /// EDUCATIONAL: JALR performs an indirect jump and saves the return address.
    /// It uses I-type format with: rd (return address), rs1 (base address), and immediate offset.
    /// Used for indirect function calls and virtual function tables.
    Jalr = 0x67,

    /// OP-IMM (0x13): Immediate arithmetic and logical operations
    /// EDUCATIONAL: Immediate operations perform arithmetic/logical operations with a constant.
    /// They use I-type format with: rd, rs1, and immediate value.
    /// Examples: ADDI (add immediate), ANDI (and immediate), SLTI (set if less than immediate)
    OpImm = 0x13,

    /// OP (0x33): Register-register arithmetic and logical operations
    /// EDUCATIONAL: Register operations perform arithmetic/logical operations between registers.
    /// They use R-type format with: rd, rs1, and rs2.
    /// Examples: ADD, SUB, AND, OR, XOR, SLT (set if less than)
    Op = 0x33,

    /// LUI (0x37): Load Upper Immediate - load large constants
    /// EDUCATIONAL: LUI loads a 20-bit immediate into the upper bits of a register.
    /// It uses U-type format with: rd and immediate value (shifted left by 12).
    /// Used for loading large constants and addresses.
    Lui = 0x37,

    /// AUIPC (0x17): Add Upper Immediate to PC - PC-relative addressing
    /// EDUCATIONAL: AUIPC adds a 20-bit immediate (shifted left by 12) to the current PC.
    /// It uses U-type format with: rd and immediate value.
    /// Used for PC-relative addressing and position-independent code.
    Auipc = 0x17,

    /// SYSTEM (0x73): System instructions - ECALL, EBREAK, CSR operations
    /// EDUCATIONAL: System instructions provide privileged operations and OS interaction.
    /// They use I-type format and include: ECALL (environment call), EBREAK (breakpoint).
    /// Used for system calls, debugging, and privileged operations.
    System = 0x73,

    /// AMO (0x2F): Atomic Memory Operations - thread-safe memory operations
    /// EDUCATIONAL: AMO instructions perform atomic read-modify-write operations.
    /// They use R-type format and include: AMOSWAP, AMOADD, AMOAND, etc.
    /// Used for multi-threaded programming and synchronization primitives.
    Amo = 0x2f,
}

impl Opcode {
    /// EDUCATIONAL: Convert a raw byte value to an Opcode enum
    ///
    /// This function takes a byte value and attempts to match it to a valid RISC-V opcode.
    /// If the value doesn't correspond to a known opcode, it returns None.
    ///
    /// REAL-WORLD CONTEXT: In actual RISC-V processors, this conversion happens
    /// in the instruction decoder hardware, which extracts the opcode bits from
    /// the instruction word and routes the instruction to the appropriate execution unit.
    pub fn from_u8(value: u8) -> Option<Self> {
        use Opcode::*;
        Some(match value {
            0x03 => Load,     // Load instructions (LW, LH, LB, etc.)
            0x23 => Store,    // Store instructions (SW, SH, SB, etc.)
            0x63 => Branch,   // Branch instructions (BEQ, BNE, BLT, etc.)
            0x6f => Jal,      // Jump and Link (unconditional jump)
            0x67 => Jalr,     // Jump and Link Register (indirect jump)
            0x13 => OpImm,    // Immediate operations (ADDI, ANDI, etc.)
            0x33 => Op,       // Register operations (ADD, SUB, AND, etc.)
            0x37 => Lui,      // Load Upper Immediate
            0x17 => Auipc,    // Add Upper Immediate to PC
            0x73 => System,   // System instructions (ECALL, EBREAK)
            0x2f => Amo,      // Atomic Memory Operations
            _ => return None, // Unknown opcode
        })
    }
}
