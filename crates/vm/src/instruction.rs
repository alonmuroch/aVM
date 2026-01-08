/// This enum defines the full instruction set for RV32IMAC
/// (Integer, Multiplication, Atomic, and Compressed extensions).
/// Each variant corresponds to a decoded instruction
/// from either a 32-bit or 16-bit RISC-V word.
///
/// EDUCATIONAL PURPOSE: This enum represents the complete instruction set
/// that our RISC-V virtual machine can execute. Understanding this structure
/// helps students learn about:
/// - CPU instruction formats and encoding
/// - Different types of operations (arithmetic, memory, control flow)
/// - How high-level programming constructs map to machine instructions
///
/// RISC-V INSTRUCTION FORMATS:
/// - R-type: Register operations (add, sub, and, or, etc.)
/// - I-type: Immediate operations (addi, lw, jalr, etc.)
/// - S-type: Store operations (sw, sh, sb)
/// - B-type: Branch operations (beq, bne, blt, etc.)
/// - U-type: Upper immediate operations (lui, auipc)
/// - J-type: Jump operations (jal)
///
/// INSTRUCTION CATEGORIES:
/// - ARITHMETIC: Basic math operations (add, sub, mul, div)
/// - LOGICAL: Bit manipulation (and, or, xor, shifts)
/// - MEMORY: Load and store operations
/// - CONTROL FLOW: Branches and jumps
/// - SYSTEM: Environment calls and breakpoints
/// - ATOMIC: Thread-safe memory operations
/// - COMPRESSED: 16-bit versions of common instructions
///
/// REAL-WORLD CONTEXT: In actual RISC-V processors, these instructions
/// are encoded as binary data. The decoder converts these binary patterns
/// into this enum structure, which makes them easier to work with in our VM.
///
/// PERFORMANCE IMPLICATIONS: Different instruction types have different
/// execution costs. Memory operations are typically slower than register
/// operations, and branches can cause pipeline stalls in real CPUs.
#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    // ===== RV32I =====
    /// ADD: rd = rs1 + rs2
    /// EDUCATIONAL: Basic arithmetic addition. Uses wrapping_add to handle overflow correctly.
    /// In real CPUs, overflow might set condition flags or trigger exceptions.
    /// This is an R-type instruction (register-register operation).
    Add { rd: usize, rs1: usize, rs2: usize },

    /// SUB: rd = rs1 - rs2  
    /// EDUCATIONAL: Basic arithmetic subtraction. Uses wrapping_sub to handle underflow.
    /// This is an R-type instruction (register-register operation).
    Sub { rd: usize, rs1: usize, rs2: usize },

    /// ADDI: rd = rs1 + imm
    /// EDUCATIONAL: Add immediate value to register. The immediate is sign-extended.
    /// This is an I-type instruction (register-immediate operation).
    /// Commonly used for adding constants and address calculations.
    Addi { rd: usize, rs1: usize, imm: i32 },

    /// AND: rd = rs1 & rs2
    /// EDUCATIONAL: Bitwise AND operation. Each bit in rd is 1 only if both corresponding
    /// bits in rs1 and rs2 are 1. Used for masking and clearing bits.
    /// This is an R-type instruction (register-register operation).
    And { rd: usize, rs1: usize, rs2: usize },

    /// OR: rd = rs1 | rs2
    /// EDUCATIONAL: Bitwise OR operation. Each bit in rd is 1 if either corresponding
    /// bit in rs1 or rs2 is 1. Used for setting bits and combining flags.
    /// This is an R-type instruction (register-register operation).
    Or { rd: usize, rs1: usize, rs2: usize },

    /// XOR: rd = rs1 ^ rs2
    /// EDUCATIONAL: Bitwise XOR (exclusive OR) operation. Each bit in rd is 1 if the
    /// corresponding bits in rs1 and rs2 are different. Used for toggling bits and
    /// detecting differences. This is an R-type instruction.
    Xor { rd: usize, rs1: usize, rs2: usize },

    /// ANDI: rd = rs1 & imm
    /// EDUCATIONAL: Bitwise AND with immediate. The immediate is sign-extended to 32 bits.
    /// Used for masking operations with constants. This is an I-type instruction.
    Andi { rd: usize, rs1: usize, imm: i32 },

    /// ORI: rd = rs1 | imm
    /// EDUCATIONAL: Bitwise OR with immediate. The immediate is sign-extended to 32 bits.
    /// Used for setting specific bits with constants. This is an I-type instruction.
    Ori { rd: usize, rs1: usize, imm: i32 },

    /// XORI: rd = rs1 ^ imm
    /// EDUCATIONAL: Bitwise XOR with immediate. The immediate is sign-extended to 32 bits.
    /// Used for toggling specific bits with constants. This is an I-type instruction.
    Xori { rd: usize, rs1: usize, imm: i32 },

    /// SLT: rd = (rs1 < rs2) ? 1 : 0
    /// EDUCATIONAL: Set if less than (signed comparison). Sets rd to 1 if rs1 < rs2,
    /// 0 otherwise. Used for conditional logic and comparisons.
    /// This is an R-type instruction (register-register operation).
    Slt { rd: usize, rs1: usize, rs2: usize },

    /// SLTU: rd = (rs1 < rs2) ? 1 : 0 (unsigned)
    /// EDUCATIONAL: Set if less than (unsigned comparison). Treats values as unsigned.
    /// Used for address comparisons and unsigned arithmetic checks.
    /// This is an R-type instruction (register-register operation).
    Sltu { rd: usize, rs1: usize, rs2: usize },

    /// SLTI: rd = (rs1 < imm) ? 1 : 0
    /// EDUCATIONAL: Set if less than immediate (signed comparison). The immediate is
    /// sign-extended. Used for comparing registers with constants.
    /// This is an I-type instruction (register-immediate operation).
    Slti { rd: usize, rs1: usize, imm: i32 },

    /// SLTIU: rd = (rs1 < imm) ? 1 : 0 (unsigned)
    /// EDUCATIONAL: Set if less than immediate (unsigned comparison). The immediate is
    /// sign-extended but comparison is unsigned. Used for unsigned comparisons with constants.
    /// This is an I-type instruction (register-immediate operation).
    Sltiu { rd: usize, rs1: usize, imm: i32 },

    /// SLL: rd = rs1 << rs2
    /// EDUCATIONAL: Shift left logical. Shifts rs1 left by rs2 bits, filling with zeros.
    /// Equivalent to multiplying by 2^rs2. Only the bottom 5 bits of rs2 are used.
    /// This is an R-type instruction (register-register operation).
    Sll { rd: usize, rs1: usize, rs2: usize },

    /// SRL: rd = rs1 >> rs2 (logical)
    /// EDUCATIONAL: Shift right logical. Shifts rs1 right by rs2 bits, filling with zeros.
    /// Equivalent to unsigned division by 2^rs2. Only the bottom 5 bits of rs2 are used.
    /// This is an R-type instruction (register-register operation).
    Srl { rd: usize, rs1: usize, rs2: usize },

    /// SRA: rd = rs1 >> rs2 (arithmetic)
    /// EDUCATIONAL: Shift right arithmetic. Shifts rs1 right by rs2 bits, preserving the sign bit.
    /// Equivalent to signed division by 2^rs2. Only the bottom 5 bits of rs2 are used.
    /// This is an R-type instruction (register-register operation).
    Sra { rd: usize, rs1: usize, rs2: usize },

    /// SLLI: rd = rs1 << shamt
    /// EDUCATIONAL: Shift left logical immediate. Shifts rs1 left by shamt bits, filling with zeros.
    /// The shift amount is a 5-bit immediate value (0-31). Used for fast multiplication by powers of 2.
    /// This is an I-type instruction (register-immediate operation).
    Slli { rd: usize, rs1: usize, shamt: u8 },

    /// SRLI: rd = rs1 >> shamt (logical)
    /// EDUCATIONAL: Shift right logical immediate. Shifts rs1 right by shamt bits, filling with zeros.
    /// The shift amount is a 5-bit immediate value (0-31). Used for fast unsigned division by powers of 2.
    /// This is an I-type instruction (register-immediate operation).
    Srli { rd: usize, rs1: usize, shamt: u8 },

    /// SRAI: rd = rs1 >> shamt (arithmetic)
    /// EDUCATIONAL: Shift right arithmetic immediate. Shifts rs1 right by shamt bits, preserving the sign bit.
    /// The shift amount is a 5-bit immediate value (0-31). Used for fast signed division by powers of 2.
    /// This is an I-type instruction (register-immediate operation).
    Srai { rd: usize, rs1: usize, shamt: u8 },

    /// LW: rd = *(rs1 + offset)
    /// EDUCATIONAL: Load word (32-bit) from memory. Address = rs1 + offset (sign-extended).
    /// Loads a full 32-bit word from memory into register rd. This is an I-type instruction.
    /// Used for loading variables, array elements, and pointer dereferencing.
    Lw { rd: usize, rs1: usize, offset: i32 },

    /// LD: rd = *(rs1 + offset) (64-bit, truncated to 32-bit)
    /// EDUCATIONAL: Load doubleword (64-bit) from memory, but truncated to 32-bit in this VM.
    /// In a real 64-bit RISC-V system, this would load 64 bits. Address = rs1 + offset.
    /// This is an I-type instruction.
    Ld { rd: usize, rs1: usize, offset: i32 },

    /// LB: rd = sign_extend(*(rs1 + offset))
    /// EDUCATIONAL: Load byte (8-bit) from memory with sign extension. Address = rs1 + offset.
    /// Loads a single byte and sign-extends it to 32 bits. Used for loading signed char values.
    /// This is an I-type instruction.
    Lb { rd: usize, rs1: usize, offset: i32 },

    /// LBU: rd = zero_extend(*(rs1 + offset))
    /// EDUCATIONAL: Load byte unsigned (8-bit) from memory with zero extension. Address = rs1 + offset.
    /// Loads a single byte and zero-extends it to 32 bits. Used for loading unsigned char values.
    /// This is an I-type instruction.
    Lbu { rd: usize, rs1: usize, offset: i32 },

    /// LH: rd = sign_extend(*(rs1 + offset))
    /// EDUCATIONAL: Load halfword (16-bit) from memory with sign extension. Address = rs1 + offset.
    /// Loads a 16-bit value and sign-extends it to 32 bits. Used for loading signed short values.
    /// This is an I-type instruction.
    Lh { rd: usize, rs1: usize, offset: i32 },

    /// LHU: rd = zero_extend(*(rs1 + offset))
    /// EDUCATIONAL: Load halfword unsigned (16-bit) from memory with zero extension. Address = rs1 + offset.
    /// Loads a 16-bit value and zero-extends it to 32 bits. Used for loading unsigned short values.
    /// This is an I-type instruction.
    Lhu { rd: usize, rs1: usize, offset: i32 },
    /// SH: *(rs1 + offset) = rs2 & 0xFFFF
    /// EDUCATIONAL: Store halfword (16-bit) to memory. Address = rs1 + offset (sign-extended).
    /// Stores the lower 16 bits of rs2 to memory. Used for storing short values.
    /// This is an S-type instruction (store operation).
    Sh { rs1: usize, rs2: usize, offset: i32 },

    /// SW: *(rs1 + offset) = rs2
    /// EDUCATIONAL: Store word (32-bit) to memory. Address = rs1 + offset (sign-extended).
    /// Stores the full 32-bit value from rs2 to memory. Used for storing variables and pointers.
    /// This is an S-type instruction (store operation).
    Sw { rs1: usize, rs2: usize, offset: i32 },

    /// SB: *(rs1 + offset) = rs2 & 0xFF
    /// EDUCATIONAL: Store byte (8-bit) to memory. Address = rs1 + offset (sign-extended).
    /// Stores the lower 8 bits of rs2 to memory. Used for storing char values.
    /// This is an S-type instruction (store operation).
    Sb { rs1: usize, rs2: usize, offset: i32 },

    /// BEQ: if (rs1 == rs2) pc += offset
    /// EDUCATIONAL: Branch if equal. Conditionally jumps if rs1 equals rs2.
    /// The offset is sign-extended and added to the current PC. Used for if/else statements.
    /// This is a B-type instruction (branch operation).
    Beq { rs1: usize, rs2: usize, offset: i32 },

    /// BNE: if (rs1 != rs2) pc += offset
    /// EDUCATIONAL: Branch if not equal. Conditionally jumps if rs1 is not equal to rs2.
    /// The offset is sign-extended and added to the current PC. Used for loop conditions.
    /// This is a B-type instruction (branch operation).
    Bne { rs1: usize, rs2: usize, offset: i32 },

    /// BLT: if (rs1 < rs2) pc += offset (signed)
    /// EDUCATIONAL: Branch if less than (signed comparison). Conditionally jumps if rs1 < rs2.
    /// Treats values as signed integers. Used for signed comparisons in loops and conditionals.
    /// This is a B-type instruction (branch operation).
    Blt { rs1: usize, rs2: usize, offset: i32 },

    /// BGE: if (rs1 >= rs2) pc += offset (signed)
    /// EDUCATIONAL: Branch if greater than or equal (signed comparison). Conditionally jumps if rs1 >= rs2.
    /// Treats values as signed integers. Used for signed comparisons in loops and conditionals.
    /// This is a B-type instruction (branch operation).
    Bge { rs1: usize, rs2: usize, offset: i32 },

    /// BLTU: if (rs1 < rs2) pc += offset (unsigned)
    /// EDUCATIONAL: Branch if less than (unsigned comparison). Conditionally jumps if rs1 < rs2.
    /// Treats values as unsigned integers. Used for address comparisons and unsigned arithmetic.
    /// This is a B-type instruction (branch operation).
    Bltu { rs1: usize, rs2: usize, offset: i32 },

    /// BGEU: if (rs1 >= rs2) pc += offset (unsigned)
    /// EDUCATIONAL: Branch if greater than or equal (unsigned comparison). Conditionally jumps if rs1 >= rs2.
    /// Treats values as unsigned integers. Used for address comparisons and unsigned arithmetic.
    /// This is a B-type instruction (branch operation).
    Bgeu { rs1: usize, rs2: usize, offset: i32 },

    /// JAL: rd = pc + 4; pc += offset
    /// EDUCATIONAL: Jump and link (unconditional jump with return address). Saves the return address
    /// (PC + 4) in rd, then jumps to PC + offset. Used for function calls and long-distance jumps.
    /// This is a J-type instruction (jump operation).
    Jal {
        rd: usize,
        offset: i32,
        compressed: bool,
    },

    /// JALR: rd = pc + 4; pc = rs1 + offset
    /// EDUCATIONAL: Jump and link register (indirect function call). Saves the return address
    /// (PC + 4) in rd, then jumps to rs1 + offset (with bottom bit cleared for alignment).
    /// Used for indirect function calls and virtual function tables.
    /// This is an I-type instruction (register-immediate operation).
    Jalr {
        rd: usize,
        rs1: usize,
        offset: i32,
        compressed: bool,
    },

    /// LUI: rd = imm << 12
    /// EDUCATIONAL: Load upper immediate. Loads a 20-bit immediate into bits 31-12 of rd,
    /// with bits 11-0 set to zero. Used for loading large constants and addresses.
    /// This is a U-type instruction (upper immediate operation).
    Lui { rd: usize, imm: i32 },

    /// AUIPC: rd = pc + (imm << 12)
    /// EDUCATIONAL: Add upper immediate to PC. Adds a 20-bit immediate (shifted left by 12)
    /// to the current PC and stores the result in rd. Used for PC-relative addressing
    /// and position-independent code. This is a U-type instruction.
    Auipc { rd: usize, imm: i32 },

    /// ECALL: Environment call
    /// EDUCATIONAL: Environment call instruction. Used to request services from the operating system
    /// or virtual machine. In this VM, it triggers the syscall handler to process system calls.
    /// Common uses include: file I/O, memory allocation, process control, and program termination.
    /// This is a system instruction.
    Ecall,

    // ===== RV32M (Multiplication and Division Extension) =====
    // EDUCATIONAL: The M extension adds hardware support for multiplication and division operations.
    // These operations are more complex than basic arithmetic and require multiple clock cycles
    // in real hardware. They're essential for efficient mathematical computations.
    /// MUL: rd = rs1 * rs2 (lower 32 bits)
    /// EDUCATIONAL: Multiply two registers and store the lower 32 bits of the result.
    /// Uses wrapping multiplication to handle overflow. This is an R-type instruction.
    /// Used for basic multiplication operations in arithmetic expressions.
    Mul { rd: usize, rs1: usize, rs2: usize },

    /// MULH: rd = (rs1 * rs2)[63:32] (signed)
    /// EDUCATIONAL: Multiply two signed registers and store the upper 32 bits of the 64-bit result.
    /// Used for high-precision arithmetic and when you need the full 64-bit product.
    /// This is an R-type instruction.
    Mulh { rd: usize, rs1: usize, rs2: usize },

    /// MULHU: rd = (rs1 * rs2)[63:32] (unsigned)
    /// EDUCATIONAL: Multiply two unsigned registers and store the upper 32 bits of the 64-bit result.
    /// Used for unsigned high-precision arithmetic. This is an R-type instruction.
    Mulhu { rd: usize, rs1: usize, rs2: usize },

    /// MULHSU: rd = (rs1 * rs2)[63:32] (signed * unsigned)
    /// EDUCATIONAL: Multiply a signed register by an unsigned register and store the upper 32 bits.
    /// Used for mixed signed/unsigned arithmetic. This is an R-type instruction.
    Mulhsu { rd: usize, rs1: usize, rs2: usize },

    /// DIV: rd = rs1 / rs2 (signed)
    /// EDUCATIONAL: Signed division. Divides rs1 by rs2 and stores the quotient in rd.
    /// Division by zero returns -1, and overflow (dividend = -2^31, divisor = -1) returns the dividend.
    /// This is an R-type instruction. Used for signed integer division.
    Div { rd: usize, rs1: usize, rs2: usize },

    /// DIVU: rd = rs1 / rs2 (unsigned)
    /// EDUCATIONAL: Unsigned division. Divides rs1 by rs2 and stores the quotient in rd.
    /// Division by zero returns 2^32 - 1. This is an R-type instruction.
    /// Used for unsigned integer division.
    Divu { rd: usize, rs1: usize, rs2: usize },

    /// REM: rd = rs1 % rs2 (signed)
    /// EDUCATIONAL: Signed remainder (modulo). Computes the remainder of rs1 divided by rs2.
    /// Division by zero returns the dividend. This is an R-type instruction.
    /// Used for signed modulo operations.
    Rem { rd: usize, rs1: usize, rs2: usize },

    /// REMU: rd = rs1 % rs2 (unsigned)
    /// EDUCATIONAL: Unsigned remainder (modulo). Computes the remainder of rs1 divided by rs2.
    /// Division by zero returns the dividend. This is an R-type instruction.
    /// Used for unsigned modulo operations.
    Remu { rd: usize, rs1: usize, rs2: usize },

    // ===== RV32A (Atomic Memory Operations Extension) =====
    // EDUCATIONAL: The A extension provides atomic memory operations for multi-threaded programming.
    // These instructions perform read-modify-write operations atomically, ensuring thread safety
    // without requiring explicit locks. They're essential for implementing synchronization primitives.
    /// AMOSWAP.W: rd = memory[rs1]; memory[rs1] = rs2
    /// EDUCATIONAL: Atomic swap. Atomically exchanges the value in memory with the value in rs2.
    /// Returns the original value from memory in rd. Used for implementing locks and mutexes.
    /// This is an atomic memory operation (AMO).
    AmoswapW { rd: usize, rs1: usize, rs2: usize },

    /// AMOADD.W: rd = memory[rs1]; memory[rs1] += rs2
    /// EDUCATIONAL: Atomic add. Atomically adds rs2 to the value in memory.
    /// Returns the original value from memory in rd. Used for atomic counters and accumulators.
    /// This is an atomic memory operation (AMO).
    AmoaddW { rd: usize, rs1: usize, rs2: usize },

    /// AMOAND.W: rd = memory[rs1]; memory[rs1] &= rs2
    /// EDUCATIONAL: Atomic bitwise AND. Atomically performs AND between memory value and rs2.
    /// Returns the original value from memory in rd. Used for atomic bit clearing operations.
    /// This is an atomic memory operation (AMO).
    AmoandW { rd: usize, rs1: usize, rs2: usize },

    /// AMOOR.W: rd = memory[rs1]; memory[rs1] |= rs2
    /// EDUCATIONAL: Atomic bitwise OR. Atomically performs OR between memory value and rs2.
    /// Returns the original value from memory in rd. Used for atomic bit setting operations.
    /// This is an atomic memory operation (AMO).
    AmoorW { rd: usize, rs1: usize, rs2: usize },

    /// AMOXOR.W: rd = memory[rs1]; memory[rs1] ^= rs2
    /// EDUCATIONAL: Atomic bitwise XOR. Atomically performs XOR between memory value and rs2.
    /// Returns the original value from memory in rd. Used for atomic bit toggling operations.
    /// This is an atomic memory operation (AMO).
    AmoxorW { rd: usize, rs1: usize, rs2: usize },

    /// AMOMAX.W: rd = memory[rs1]; memory[rs1] = max(memory[rs1], rs2) (signed)
    /// EDUCATIONAL: Atomic maximum (signed). Atomically sets memory to the maximum of its current
    /// value and rs2 (signed comparison). Returns the original value from memory in rd.
    /// This is an atomic memory operation (AMO).
    AmomaxW { rd: usize, rs1: usize, rs2: usize },

    /// AMOMIN.W: rd = memory[rs1]; memory[rs1] = min(memory[rs1], rs2) (signed)
    /// EDUCATIONAL: Atomic minimum (signed). Atomically sets memory to the minimum of its current
    /// value and rs2 (signed comparison). Returns the original value from memory in rd.
    /// This is an atomic memory operation (AMO).
    AmominW { rd: usize, rs1: usize, rs2: usize },

    /// AMOMAXU.W: rd = memory[rs1]; memory[rs1] = max(memory[rs1], rs2) (unsigned)
    /// EDUCATIONAL: Atomic maximum (unsigned). Atomically sets memory to the maximum of its current
    /// value and rs2 (unsigned comparison). Returns the original value from memory in rd.
    /// This is an atomic memory operation (AMO).
    AmomaxuW { rd: usize, rs1: usize, rs2: usize },

    /// AMOMINU.W: rd = memory[rs1]; memory[rs1] = min(memory[rs1], rs2) (unsigned)
    /// EDUCATIONAL: Atomic minimum (unsigned). Atomically sets memory to the minimum of its current
    /// value and rs2 (unsigned comparison). Returns the original value from memory in rd.
    /// This is an atomic memory operation (AMO).
    AmominuW { rd: usize, rs1: usize, rs2: usize },

    /// LR.W: rd = memory[rs1]; set reservation
    /// EDUCATIONAL: Load Reserved (Load with reservation). Loads a value from memory and sets
    /// a reservation on that memory address. Used in conjunction with SC for atomic operations.
    /// This is a load-reserved operation.
    LrW { rd: usize, rs1: usize },

    /// SC.W: rd = success ? 0 : 1; if (success) memory[rs1] = rs2
    /// EDUCATIONAL: Store Conditional. Stores rs2 to memory only if the reservation is still valid.
    /// Returns 0 on success, 1 on failure. Used with LR for implementing atomic operations.
    /// This is a store-conditional operation.
    ScW { rd: usize, rs1: usize, rs2: usize },

    // ===== RV32C (Compressed Instructions Extension) =====
    // EDUCATIONAL: The C extension provides 16-bit versions of common 32-bit instructions.
    // These instructions save code space and improve instruction cache efficiency.
    // They're automatically used by compilers when possible to reduce program size.
    /// C.JR: pc = rs1 (compressed jump register)
    /// EDUCATIONAL: Compressed jump register. Jumps to the address in rs1.
    /// This is a 16-bit version of JALR with rd=x0 (no return address saved).
    /// Used for indirect jumps and function calls where return address isn't needed.
    Jr { rs1: usize },

    /// C.RET: pc = x1 (compressed return)
    /// EDUCATIONAL: Compressed return instruction. Equivalent to JR x1 (jump to return address).
    /// This is a 16-bit version of JALR with rs1=x1, rd=x0. Used for function returns.
    Ret,

    /// C.MV: rd = rs2 (compressed move)
    /// EDUCATIONAL: Compressed register move. Copies the value from rs2 to rd.
    /// This is a 16-bit version of ADD with rs1=x0. Used for register-to-register copies.
    Mv { rd: usize, rs2: usize },

    /// C.ADDI16SP: x2 += imm (compressed add immediate to stack pointer)
    /// EDUCATIONAL: Compressed add immediate to stack pointer. Adds a 6-bit immediate to x2 (SP).
    /// Used for stack frame adjustments in function prologues and epilogues.
    /// This is a 16-bit version of ADDI with rd=rs1=x2.
    Addi16sp { imm: i32 },

    /// C.ADDI4SPN: rd = x2 + imm (compressed add immediate to SP + register)
    /// EDUCATIONAL: Compressed add immediate to stack pointer and store in register.
    /// Adds a 10-bit immediate to x2 (SP) and stores the result in rd.
    /// Used for accessing stack variables and local storage.
    Addi4spn { rd: usize, imm: u32 },

    /// C.NOP: no operation (compressed nop)
    /// EDUCATIONAL: Compressed no-operation instruction. Does nothing, used for alignment
    /// and timing in real systems. This is a 16-bit version of ADDI with rd=rs1=x0, imm=0.
    Nop,

    /// C.BEQZ: if (rs1 == 0) pc += offset (compressed branch if equal to zero)
    /// EDUCATIONAL: Compressed branch if equal to zero. Conditionally jumps if rs1 equals zero.
    /// This is a 16-bit version of BEQ with rs2=x0. Used for null pointer checks and loop conditions.
    Beqz { rs1: usize, offset: i32 },

    /// C.BNEZ: if (rs1 != 0) pc += offset (compressed branch if not equal to zero)
    /// EDUCATIONAL: Compressed branch if not equal to zero. Conditionally jumps if rs1 is not zero.
    /// This is a 16-bit version of BNE with rs2=x0. Used for loop conditions and existence checks.
    Bnez { rs1: usize, offset: i32 },

    /// C.EBREAK: environment breakpoint (compressed)
    /// EDUCATIONAL: Compressed environment breakpoint. Used for debugging and program termination.
    /// This is a 16-bit version of EBREAK. Triggers a debugger breakpoint in real systems.
    Ebreak,

    /// MRET: Machine-mode return (treated as a halt in this VM)
    Mret,
    /// SRET: Supervisor-mode return
    Sret,

    /// C.MISC-ALU: compressed miscellaneous ALU operations
    /// EDUCATIONAL: Compressed miscellaneous ALU operations including C.SUB, C.XOR, C.OR, C.AND.
    /// These are 16-bit versions of common logical and arithmetic operations.
    /// Used to save code space for frequently used operations.
    MiscAlu {
        rd: usize,
        rs2: usize,
        op: MiscAluOp,
    },

    /// FENCE: Memory barrier
    /// EDUCATIONAL: Memory barrier instruction. Ensures memory operations complete in order.
    /// In real hardware, this prevents instruction reordering across the fence.
    /// In this VM, it's implemented as a no-op since we don't have instruction reordering.
    Fence,

    /// CSR: Control and Status Register operations (CSRRW/CSRRS/CSRRC and immediate variants)
    /// EDUCATIONAL: Allows reading/writing CSRs like mhartid, misa, etc.
    Csr {
        rd: usize,
        rs1: usize,
        csr: u16,
        op: CsrOp,
        imm: bool,
    },

    /// UNIMP: Unimplemented instruction
    /// EDUCATIONAL: Unimplemented instruction marker. Used for instructions that aren't
    /// supported by this VM but may be present in compiled code.
    /// In this VM, it's treated as a no-op for compatibility.
    Unimp,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CsrOp {
    Csrrw,
    Csrrs,
    Csrrc,
}

/// EDUCATIONAL: Miscellaneous ALU operations for compressed instructions.
/// These represent the different operations that can be performed by the C.MISC-ALU instruction.
/// Each operation is a 16-bit compressed version of a corresponding 32-bit instruction.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MiscAluOp {
    /// C.SUB: rd = rd - rs2 (compressed subtract)
    /// EDUCATIONAL: Compressed subtract operation. Subtracts rs2 from rd and stores result in rd.
    /// This is a 16-bit version of SUB instruction.
    Sub,

    /// C.XOR: rd = rd ^ rs2 (compressed XOR)
    /// EDUCATIONAL: Compressed XOR operation. Performs bitwise XOR between rd and rs2.
    /// This is a 16-bit version of XOR instruction.
    Xor,

    /// C.OR: rd = rd | rs2 (compressed OR)
    /// EDUCATIONAL: Compressed OR operation. Performs bitwise OR between rd and rs2.
    /// This is a 16-bit version of OR instruction.
    Or,

    /// C.AND: rd = rd & rs2 (compressed AND)
    /// EDUCATIONAL: Compressed AND operation. Performs bitwise AND between rd and rs2.
    /// This is a 16-bit version of AND instruction.
    And,
}

impl Instruction {
    pub fn pretty_print(&self) -> String {
        fn reg(r: usize) -> String {
            format!("x{r}") // or use register aliases like a0, t1, etc. if desired
        }

        match self {
            Instruction::Add { rd, rs1, rs2 } => {
                format!("add  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Sub { rd, rs1, rs2 } => {
                format!("sub  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Addi { rd, rs1, imm } => {
                format!("addi {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }

            Instruction::And { rd, rs1, rs2 } => {
                format!("and  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Or { rd, rs1, rs2 } => {
                format!("or   {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Xor { rd, rs1, rs2 } => {
                format!("xor  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }

            Instruction::Andi { rd, rs1, imm } => {
                format!("andi {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }
            Instruction::Ori { rd, rs1, imm } => {
                format!("ori  {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }
            Instruction::Xori { rd, rs1, imm } => {
                format!("xori {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }

            Instruction::Slt { rd, rs1, rs2 } => {
                format!("slt  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Sltu { rd, rs1, rs2 } => {
                format!("sltu {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Slti { rd, rs1, imm } => {
                format!("slti {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }
            Instruction::Sltiu { rd, rs1, imm } => {
                format!("sltiu {}, {}, {}", reg(*rd), reg(*rs1), imm)
            }

            Instruction::Sll { rd, rs1, rs2 } => {
                format!("sll  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Srl { rd, rs1, rs2 } => {
                format!("srl  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Sra { rd, rs1, rs2 } => {
                format!("sra  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Slli { rd, rs1, shamt } => {
                format!("slli {}, {}, {}", reg(*rd), reg(*rs1), shamt)
            }
            Instruction::Srli { rd, rs1, shamt } => {
                format!("srli {}, {}, {}", reg(*rd), reg(*rs1), shamt)
            }
            Instruction::Srai { rd, rs1, shamt } => {
                format!("srai {}, {}, {}", reg(*rd), reg(*rs1), shamt)
            }

            Instruction::Lw { rd, rs1, offset } => {
                format!("lw   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Ld { rd, rs1, offset } => {
                format!("ld   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Lb { rd, rs1, offset } => {
                format!("lb   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Lbu { rd, rs1, offset } => {
                format!("lbu   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Lh { rd, rs1, offset } => {
                format!("lh   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Lhu { rd, rs1, offset } => {
                format!("lhu   {}, {}({})", reg(*rd), offset, reg(*rs1))
            }
            Instruction::Sh { rs1, rs2, offset } => {
                format!("sh   {}, {}({})", reg(*rs2), offset, reg(*rs1))
            }
            Instruction::Sw { rs1, rs2, offset } => {
                format!("sw   {}, {}({})", reg(*rs2), offset, reg(*rs1))
            }
            Instruction::Sb { rs1, rs2, offset } => {
                format!("sb   {}, {}({})", reg(*rs2), offset, reg(*rs1))
            }

            Instruction::Beq { rs1, rs2, offset } => {
                format!("beq  {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }
            Instruction::Bne { rs1, rs2, offset } => {
                format!("bne  {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }
            Instruction::Blt { rs1, rs2, offset } => {
                format!("blt  {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }
            Instruction::Bge { rs1, rs2, offset } => {
                format!("bge  {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }
            Instruction::Bltu { rs1, rs2, offset } => {
                format!("bltu {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }
            Instruction::Bgeu { rs1, rs2, offset } => {
                format!("bgeu {}, {}, pc+{}", reg(*rs1), reg(*rs2), offset)
            }

            Instruction::Jal {
                rd,
                offset,
                compressed: _,
            } => format!("jal  {}, pc+{}", reg(*rd), offset),
            Instruction::Jalr {
                rd,
                rs1,
                offset,
                compressed: _,
            } => format!("jalr {}, {}({})", reg(*rd), offset, reg(*rs1)),

            Instruction::Lui { rd, imm } => format!("lui  {}, {}", reg(*rd), imm),
            Instruction::Auipc { rd, imm } => format!("auipc {}, {}", reg(*rd), imm),

            Instruction::Ecall => "ecall".to_string(),
            Instruction::Fence => "fence".to_string(),
            Instruction::Unimp => "unimp".to_string(),

            Instruction::Mul { rd, rs1, rs2 } => {
                format!("mul  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Mulh { rd, rs1, rs2 } => {
                format!("mulh {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Mulhsu { rd, rs1, rs2 } => {
                format!("mulhsu {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Mulhu { rd, rs1, rs2 } => {
                format!("mulhu {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }

            Instruction::Div { rd, rs1, rs2 } => {
                format!("div  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Divu { rd, rs1, rs2 } => {
                format!("divu {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Rem { rd, rs1, rs2 } => {
                format!("rem  {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::Remu { rd, rs1, rs2 } => {
                format!("remu {}, {}, {}", reg(*rd), reg(*rs1), reg(*rs2))
            }

            Instruction::AmoswapW { rd, rs1, rs2 } => {
                format!("amoswap.w {}, ({}) <- {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmoaddW { rd, rs1, rs2 } => {
                format!("amoadd.w  {}, ({}) + {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmoandW { rd, rs1, rs2 } => {
                format!("amoand.w  {}, ({}) & {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmoorW { rd, rs1, rs2 } => {
                format!("amoor.w   {}, ({}) | {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmoxorW { rd, rs1, rs2 } => {
                format!("amoxor.w  {}, ({}) ^ {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmomaxW { rd, rs1, rs2 } => {
                format!("amomax.w  {}, ({}) max {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmominW { rd, rs1, rs2 } => {
                format!("amomin.w  {}, ({}) min {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmomaxuW { rd, rs1, rs2 } => {
                format!("amomaxu.w {}, ({}) maxu {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::AmominuW { rd, rs1, rs2 } => {
                format!("amominu.w {}, ({}) minu {}", reg(*rd), reg(*rs1), reg(*rs2))
            }
            Instruction::LrW { rd, rs1 } => format!("lr.w   {}, ({})", reg(*rd), reg(*rs1)),
            Instruction::ScW { rd, rs1, rs2 } => {
                format!("sc.w   {}, ({}) <- {}", reg(*rd), reg(*rs1), reg(*rs2))
            }

            Instruction::Jr { rs1 } => format!("jr   {}", reg(*rs1)),
            Instruction::Ret => "ret".to_string(),
            Instruction::Mv { rd, rs2 } => format!("mv   {}, {}", reg(*rd), reg(*rs2)),
            Instruction::Addi16sp { imm } => format!("addi16sp sp, {imm}"),
            Instruction::Addi4spn { rd, imm } => format!("c.addi4spn {}, {}", reg(*rd), imm),

            Instruction::Nop => "nop".to_string(),
            Instruction::Beqz { rs1, offset } => format!("beqz {}, pc+{}", reg(*rs1), offset),
            Instruction::Bnez { rs1, offset } => format!("bnez {}, pc+{}", reg(*rs1), offset),
            Instruction::Ebreak => "ebreak".to_string(),
            Instruction::Mret => "mret".to_string(),
            Instruction::Sret => "sret".to_string(),
            Instruction::Csr {
                rd,
                rs1,
                csr,
                op,
                imm,
            } => {
                let op_str = match op {
                    CsrOp::Csrrw => {
                        if *imm {
                            "csrrwi"
                        } else {
                            "csrrw"
                        }
                    }
                    CsrOp::Csrrs => {
                        if *imm {
                            "csrrsi"
                        } else {
                            "csrrs"
                        }
                    }
                    CsrOp::Csrrc => {
                        if *imm {
                            "csrrci"
                        } else {
                            "csrrc"
                        }
                    }
                };
                let src = if *imm { format!("{rs1}") } else { reg(*rs1) };
                format!("{} {}, {}, 0x{:03x}", op_str, reg(*rd), src, csr)
            }

            Instruction::MiscAlu { rd, rs2, op } => {
                let op_str = match op {
                    MiscAluOp::Sub => "c.sub",
                    MiscAluOp::Xor => "c.xor",
                    MiscAluOp::Or => "c.or",
                    MiscAluOp::And => "c.and",
                };
                format!("{} {}, {}", op_str, reg(*rd), reg(*rs2))
            }
        }
    }
}
