use super::{Instruction, MemoryAccessKind, Memory, CPU, CSR_MEPC, CSR_SATP, CSR_SEPC, SCAUSE_BREAKPOINT};
use crate::console::{console_write, CONSOLE_WRITE_ID};
use crate::memory::VirtualAddress;
use crate::instruction::CsrOp;
use crate::registers::Register;

impl CPU {
    /// Executes a decoded instruction.
    ///
    /// EDUCATIONAL PURPOSE: This is the execute phase of the instruction cycle.
    /// It contains the implementation of all RISC-V instructions supported by
    /// our VM. This is where the actual computation happens.
    ///
    /// INSTRUCTION CATEGORIES:
    /// - Arithmetic: ADD, SUB, MUL, DIV, etc.
    /// - Logical: AND, OR, XOR, shifts
    /// - Memory: Load and store operations
    /// - Control: Branches and jumps
    /// - System: System calls and special operations
    ///
    /// REGISTER CONVENTIONS:
    /// - rd: Destination register (where result goes)
    /// - rs1, rs2: Source registers (operands)
    /// - imm: Immediate value (constant)
    ///
    /// RETURN VALUE: Returns true to continue execution, false to halt
    pub fn execute(
        &mut self,
        instr: Instruction,
        memory: Memory,
    ) -> bool {
        match instr {
            // EDUCATIONAL: Arithmetic instructions - perform mathematical operations
            Instruction::Add { rd, rs1, rs2 } => {
                // EDUCATIONAL: Use wrapping_add to handle overflow correctly
                // In real CPUs, overflow might set flags or cause exceptions
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs.wrapping_add(rhs)) {
                    return false;
                }
            }
            Instruction::Sub { rd, rs1, rs2 } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs.wrapping_sub(rhs)) {
                    return false;
                }
            }
            Instruction::Addi { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs.wrapping_add(imm as u32)) {
                    return false;
                }
            }

            // EDUCATIONAL: Logical instructions - perform bitwise operations
            Instruction::And { rd, rs1, rs2 } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs & rhs) {
                    return false;
                }
            }
            Instruction::Or { rd, rs1, rs2 } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs | rhs) {
                    return false;
                }
            }
            Instruction::Xor { rd, rs1, rs2 } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs ^ rhs) {
                    return false;
                }
            }
            Instruction::Andi { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs & (imm as u32)) {
                    return false;
                }
            }
            Instruction::Ori { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs | (imm as u32)) {
                    return false;
                }
            }
            Instruction::Xori { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs ^ (imm as u32)) {
                    return false;
                }
            }

            // EDUCATIONAL: Comparison instructions - set result to 0 or 1
            Instruction::Slt { rd, rs1, rs2 } => {
                // EDUCATIONAL: Set if less than (signed comparison)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, (lhs as i32).lt(&(rhs as i32)) as u32) {
                    return false;
                }
            }
            Instruction::Sltu { rd, rs1, rs2 } => {
                // EDUCATIONAL: Set if less than (unsigned comparison)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, (lhs < rhs) as u32) {
                    return false;
                }
            }
            Instruction::Slti { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, (lhs as i32).lt(&imm) as u32) {
                    return false;
                }
            }
            Instruction::Sltiu { rd, rs1, imm } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = imm as u32;
                if !self.write_reg(rd, if lhs < rhs { 1 } else { 0 }) {
                    return false;
                }
            }

            // EDUCATIONAL: Shift instructions - move bits left or right
            Instruction::Sll { rd, rs1, rs2 } => {
                // EDUCATIONAL: Logical left shift - multiply by 2^shift_amount
                // The & 0x1F ensures shift amount is 0-31 (5 bits)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs << (rhs & 0x1F)) {
                    return false;
                }
            }
            Instruction::Srl { rd, rs1, rs2 } => {
                // EDUCATIONAL: Logical right shift - divide by 2^shift_amount
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs >> (rhs & 0x1F)) {
                    return false;
                }
            }
            Instruction::Sra { rd, rs1, rs2 } => {
                // EDUCATIONAL: Arithmetic right shift - preserves sign bit
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, ((lhs as i32) >> (rhs & 0x1F)) as u32) {
                    return false;
                }
            }
            Instruction::Slli { rd, rs1, shamt } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs << shamt) {
                    return false;
                }
            }
            Instruction::Srli { rd, rs1, shamt } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs >> shamt) {
                    return false;
                }
            }
            Instruction::Srai { rd, rs1, shamt } => {
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, ((lhs as i32) >> shamt) as u32) {
                    return false;
                }
            }

            // EDUCATIONAL: Load instructions - read data from memory into registers
            Instruction::Lw { rd, rs1, offset } => {
                // EDUCATIONAL: Load word (32-bit) from memory
                // Address = base register + offset
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let val =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Load) {
                        Some(v) => v,
                        None => return false,
                    };
                if !self.write_reg(rd, val) {
                    return false;
                }
            }
            Instruction::Ld { rd, rs1, offset } => {
                // EDUCATIONAL: Load doubleword (64-bit) from memory, truncated to 32-bit
                // Since this is a 32-bit VM, we only load the lower 32 bits
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let val =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Load) {
                        Some(v) => v,
                        None => return false,
                    };
                if !self.write_reg(rd, val) {
                    return false;
                }
            }
            Instruction::Lb { rd, rs1, offset } => {
                // EDUCATIONAL: Load byte (8-bit, sign-extended)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let byte =
                    match memory.load_byte(addr, self.metering.as_mut(), MemoryAccessKind::Load) {
                        Some(v) => v,
                        None => return false,
                    };
                let value = (byte as i8) as i32 as u32; // sign-extend to 32-bit
                if !self.write_reg(rd, value) {
                    return false;
                }
            }
            Instruction::Lbu { rd, rs1, offset } => {
                // EDUCATIONAL: Load byte unsigned (8-bit, zero-extended)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let byte =
                    match memory.load_byte(addr, self.metering.as_mut(), MemoryAccessKind::Load) {
                        Some(v) => v,
                        None => return false,
                    };
                if !self.write_reg(rd, byte as u32) {
                    return false;
                }
            }
            Instruction::Lh { rd, rs1, offset } => {
                // EDUCATIONAL: Load halfword (16-bit, sign-extended)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let halfword = match memory.load_halfword(
                    addr,
                    self.metering.as_mut(),
                    MemoryAccessKind::Load,
                ) {
                    Some(v) => v,
                    None => return false,
                };
                let value = (halfword as i16) as i32 as u32; // sign-extend to 32-bit
                if !self.write_reg(rd, value) {
                    return false;
                }
            }
            Instruction::Lhu { rd, rs1, offset } => {
                // EDUCATIONAL: Load halfword unsigned (16-bit, zero-extended)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let halfword = match memory.load_halfword(
                    addr,
                    self.metering.as_mut(),
                    MemoryAccessKind::Load,
                ) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, halfword as u32) {
                    return false;
                } // zero-extend to 32-bit
            }

            // EDUCATIONAL: Store instructions - write data from registers to memory
            Instruction::Sh { rs1, rs2, offset } => {
                // EDUCATIONAL: Store halfword (16-bit)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !memory.store_u16(
                    addr,
                    (src & 0xFFFF) as u16,
                    self.metering.as_mut(),
                    MemoryAccessKind::Store,
                ) {
                    return false;
                }
            }
            Instruction::Sw { rs1, rs2, offset } => {
                // EDUCATIONAL: Store word (32-bit)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !memory.store_u32(addr, src, self.metering.as_mut(), MemoryAccessKind::Store) {
                    return false;
                }
            }
            Instruction::Sb { rs1, rs2, offset } => {
                // EDUCATIONAL: Store byte (8-bit)
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base.wrapping_add(offset as u32));
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !memory.store_u8(
                    addr,
                    (src & 0xFF) as u8,
                    self.metering.as_mut(),
                    MemoryAccessKind::Store,
                ) {
                    return false;
                }
            }

            // EDUCATIONAL: Branch instructions - conditionally change the PC
            // These implement if/else and loop constructs
            Instruction::Beq { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if equal - jump if two registers are equal
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if lhs == rhs {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            Instruction::Bne { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if not equal
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if lhs != rhs {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            Instruction::Blt { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if less than (signed comparison)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if (lhs as i32) < (rhs as i32) {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            Instruction::Bge { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if greater than or equal (signed)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if (lhs as i32) >= (rhs as i32) {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            Instruction::Bltu { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if less than (unsigned comparison)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if lhs < rhs {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }

            Instruction::Bgeu { rs1, rs2, offset } => {
                // EDUCATIONAL: Branch if greater than or equal (unsigned)
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if lhs >= rhs {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            // EDUCATIONAL: Jump and Link instructions - for function calls
            Instruction::Jal {
                rd,
                offset,
                compressed,
            } => {
                // EDUCATIONAL: JAL (Jump and Link) - unconditional jump with return address
                // Used for function calls and long-distance jumps
                // The return address is stored in rd (usually x1/ra)
                let return_address = if compressed { self.pc + 2 } else { self.pc + 4 };
                if !self.write_reg(rd, return_address) {
                    return false;
                }
                if !self.pc_add(offset as u32) {
                    return false;
                }
                return true;
            }
            Instruction::Jalr {
                rd,
                rs1,
                offset,
                compressed,
            } => {
                // EDUCATIONAL: JALR (Jump and Link Register) - indirect function calls
                // Target address = base register + offset, with bottom bit cleared
                // This ensures proper alignment and is required by RISC-V spec
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let target = base.wrapping_add(offset as u32) & !1;

                // For compressed instructions (c.jalr), return address should be pc + 2
                // For regular instructions (jalr), return address should be pc + 4
                let return_address = if compressed { self.pc + 2 } else { self.pc + 4 };

                if !self.write_reg(rd, return_address) {
                    return false;
                }

                if !self.set_pc(target) {
                    return false;
                }
                return true;
            }

            // EDUCATIONAL: Load Upper Immediate - loads immediate into upper bits
            Instruction::Lui { rd, imm } => {
                // EDUCATIONAL: LUI loads a 20-bit immediate into bits 31-12 of rd
                // This is used to load large constants (like addresses) into registers
                if !self.write_reg(rd, (imm << 12) as u32) {
                    return false;
                }
            }
            Instruction::Auipc { rd, imm } => {
                // EDUCATIONAL: AUIPC (Add Upper Immediate to PC) - PC-relative addressing
                // Used for position-independent code and loading addresses relative to PC
                if !self.write_reg(rd, self.pc.wrapping_add((imm << 12) as u32)) {
                    return false;
                }
            }

            // EDUCATIONAL: Multiplication instructions - extended arithmetic
            Instruction::Mul { rd, rs1, rs2 } => {
                // EDUCATIONAL: MUL - multiply two registers, store lower 32 bits
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, lhs.wrapping_mul(rhs)) {
                    return false;
                }
            }
            Instruction::Mulh { rd, rs1, rs2 } => {
                // EDUCATIONAL: MULH - multiply signed, store upper 32 bits
                // Properly sign-extend 32-bit values to 64-bit for signed multiplication
                let val1 = (match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                } as i32) as i64;
                let val2 = (match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                } as i32) as i64;
                let result = val1 * val2;
                if !self.write_reg(rd, (result >> 32) as u32) {
                    return false;
                }
            }
            Instruction::Mulhu { rd, rs1, rs2 } => {
                // EDUCATIONAL: MULHU - multiply unsigned, store upper 32 bits
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v as u64,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v as u64,
                    None => return false,
                };
                if !self.write_reg(rd, ((lhs * rhs) >> 32) as u32) {
                    return false;
                }
            }
            Instruction::Mulhsu { rd, rs1, rs2 } => {
                // EDUCATIONAL: MULHSU - multiply signed by unsigned, store upper 32 bits
                // Properly sign-extend first operand to signed 64-bit, keep second as unsigned 64-bit
                let val1 = (match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                } as i32) as i64;
                let val2 = match self.read_reg(rs2) {
                    Some(v) => v as u64,
                    None => return false,
                };
                let result = val1 * (val2 as i64);
                if !self.write_reg(rd, (result >> 32) as u32) {
                    return false;
                }
            }
            // EDUCATIONAL: Division and remainder instructions
            Instruction::Div { rd, rs1, rs2 } => {
                // EDUCATIONAL: DIV - signed division
                // RISC-V spec: division by zero returns -1, overflow returns dividend
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if rhs == 0 {
                    if !self.write_reg(rd, 0xFFFFFFFF) {
                        return false;
                    } // -1 in two's complement
                } else {
                    let dividend = lhs as i32;
                    let divisor = rhs as i32;

                    // Check for overflow: -2^31 / -1 = 2^31 (overflow)
                    if dividend == i32::MIN && divisor == -1 {
                        if !self.write_reg(rd, lhs) {
                            return false;
                        } // Return dividend on overflow
                    } else {
                        if !self.write_reg(rd, (dividend / divisor) as u32) {
                            return false;
                        }
                    }
                }
            }
            Instruction::Divu { rd, rs1, rs2 } => {
                // EDUCATIONAL: DIVU - unsigned division
                // RISC-V spec: division by zero returns 2^XLEN - 1
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if rhs == 0 {
                    if !self.write_reg(rd, 0xFFFFFFFF) {
                        return false;
                    } // 2^32 - 1
                } else {
                    if !self.write_reg(rd, lhs / rhs) {
                        return false;
                    }
                }
            }
            Instruction::Rem { rd, rs1, rs2 } => {
                // EDUCATIONAL: REM - signed remainder
                // RISC-V spec: remainder by zero returns dividend, overflow returns dividend
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if rhs == 0 {
                    if !self.write_reg(rd, lhs) {
                        return false;
                    }
                } else {
                    let dividend = lhs as i32;
                    let divisor = rhs as i32;

                    // Check for overflow: -2^31 % -1 = 0 (no overflow, but -2^31 % -1 = 0)
                    if dividend == i32::MIN && divisor == -1 {
                        if !self.write_reg(rd, 0) {
                            return false;
                        } // Remainder of -2^31 % -1 is 0
                    } else {
                        if !self.write_reg(rd, (dividend % divisor) as u32) {
                            return false;
                        }
                    }
                }
            }
            Instruction::Remu { rd, rs1, rs2 } => {
                // EDUCATIONAL: REMU - unsigned remainder
                // RISC-V spec: remainder by zero returns dividend
                let lhs = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let rhs = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if rhs == 0 {
                    if !self.write_reg(rd, lhs) {
                        return false;
                    }
                } else {
                    if !self.write_reg(rd, lhs % rhs) {
                        return false;
                    }
                }
            }

            // EDUCATIONAL: System instructions - for OS interaction and debugging
            Instruction::Ecall => {
                // Prepare syscall args from registers
                let args = [
                    match self.read_reg(Register::A1 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                    match self.read_reg(Register::A2 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                    match self.read_reg(Register::A3 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                    match self.read_reg(Register::A4 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                    match self.read_reg(Register::A5 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                    match self.read_reg(Register::A6 as usize) {
                        Some(v) => v,
                        None => return false,
                    },
                ];
                let call_id = match self.read_reg(Register::A7 as usize) {
                    Some(v) => v,
                    None => return false,
                };
                if call_id == CONSOLE_WRITE_ID {
                    let result = console_write(
                        args,
                        self.priv_mode,
                        memory,
                        self.metering.as_mut(),
                        &self.verbose_writer,
                    );
                    if !self.write_reg(Register::A0 as usize, result) {
                        return false;
                    }
                    return true;
                }
                if let Some(trap_mode) = self.has_trap_vector() {
                    if !self.trap_to_vector(trap_mode, self.ecall_cause(), 0, Some(call_id)) {
                        panic!(
                            "trap_to_vector returned false for ecall id={} pc=0x{:08x}",
                            call_id, self.pc
                        );
                    }
                    return true;
                }
                panic!(
                    "ecall without trap vector for id={} pc=0x{:08x}",
                    call_id, self.pc
                );
            }
            Instruction::Csr {
                rd,
                rs1,
                csr,
                op,
                imm,
            } => {
                let src = if imm {
                    rs1 as u32
                } else {
                    match self.read_reg(rs1) {
                        Some(v) => v,
                        None => return false,
                    }
                };
                let old = match self.read_csr(csr) {
                    Some(v) => v,
                    None => return false,
                };

                // Apply CSR op semantics
                let mut new_val = old;
                match op {
                    CsrOp::Csrrw => {
                        if !(imm == false && rs1 == 0) {
                            new_val = src;
                        }
                    }
                    CsrOp::Csrrs => {
                        if src != 0 {
                            new_val = old | src;
                        }
                    }
                    CsrOp::Csrrc => {
                        if src != 0 {
                            new_val = old & !src;
                        }
                    }
                }

                let will_write = src != 0 || matches!(op, CsrOp::Csrrw);
                if will_write {
                    if csr == CSR_SATP {
                        if !self.set_satp(&memory, new_val) {
                            panic!("failed to update satp");
                        }
                    } else if !self.write_csr(csr, new_val) {
                        panic!("failed to write csr 0x{:03x}", csr);
                    }
                }

                if rd != 0 && !self.write_reg(rd, old) {
                    return false;
                }
            }
            Instruction::Ebreak => {
                // EDUCATIONAL: EBREAK - Environment Break - for debugging
                // In real systems, this would trigger a debugger breakpoint
                if self.priv_mode == super::PrivilegeMode::User {
                    if let Some(trap_mode) = self.has_trap_vector() {
                        if !self.trap_to_vector(trap_mode, SCAUSE_BREAKPOINT, 0, None) {
                            panic!(
                                "trap_to_vector returned false for ebreak pc=0x{:08x}",
                                self.pc
                            );
                        }
                        return true;
                    }
                }
                return false;
            }
            Instruction::Mret => {
                let target = match self.read_csr(CSR_MEPC).or_else(|| self.read_csr(CSR_SEPC)) {
                    Some(v) => v,
                    None => return false,
                };
                let prev = self.take_sstatus_spp();
                if !self.set_pc(target) {
                    return false;
                }
                self.priv_mode = prev;
                return true;
            }
            Instruction::Sret => {
                let target = match self.read_csr(CSR_SEPC) {
                    Some(v) => v,
                    None => return false,
                };
                let prev = self.take_sstatus_spp();
                if !self.set_pc(target) {
                    return false;
                }
                self.priv_mode = prev;
                return true;
            }

            // EDUCATIONAL: Compressed instruction set (RV32C) - space-saving instructions
            Instruction::Jr { rs1 } => {
                // EDUCATIONAL: JR (Jump Register) - compressed jump to register
                let target = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.set_pc(target) {
                    return false;
                }
                return true;
            }
            Instruction::Ret => {
                // EDUCATIONAL: RET - compressed return instruction
                // Equivalent to JR x1 (jump to return address register)
                let target = match self.read_reg(1) {
                    Some(v) => v,
                    None => return false,
                }; // x1 = ra (return address)
                if target == 0 || target == 0xFFFF_FFFF {
                    return false; // halt if ret target is 0 or invalid
                }

                if !self.set_pc(target) {
                    return false;
                }
                return true;
            }
            Instruction::Mv { rd, rs2 } => {
                // EDUCATIONAL: MV (Move) - compressed register copy
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, src) {
                    return false;
                }
            }
            Instruction::Addi16sp { imm } => {
                // EDUCATIONAL: ADDI16SP - add immediate to stack pointer
                // x2 is the stack pointer (SP)
                if !self.sp_add(imm as u32) {
                    return false;
                }
            }
            Instruction::Addi4spn { rd, imm } => {
                // EDUCATIONAL: ADDI4SPN - add immediate to SP, store in rd
                // Used for stack frame setup in function prologues
                let sp = match self.read_reg(2) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, sp.wrapping_add(imm)) {
                    return false;
                }
            }
            Instruction::Nop => {
                // EDUCATIONAL: NOP - No Operation - does nothing
                // Used for alignment and timing in real systems
            }
            Instruction::Beqz { rs1, offset } => {
                // EDUCATIONAL: BEQZ - Branch if Equal to Zero (compressed)
                let val = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if val == 0 {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }
            Instruction::Bnez { rs1, offset } => {
                // EDUCATIONAL: BNEZ - Branch if Not Equal to Zero (compressed)
                let val = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                if val != 0 {
                    if !self.pc_add(offset as u32) {
                        return false;
                    }
                    return true;
                }
            }

            // EDUCATIONAL: Miscellaneous ALU operations (compressed)
            Instruction::MiscAlu { rd, rs2, op } => {
                match op {
                    crate::instruction::MiscAluOp::Sub => {
                        // EDUCATIONAL: C.SUB - compressed subtract
                        let lhs = match self.read_reg(rd) {
                            Some(v) => v,
                            None => return false,
                        };
                        let rhs = match self.read_reg(rs2) {
                            Some(v) => v,
                            None => return false,
                        };
                        if !self.write_reg(rd, lhs.wrapping_sub(rhs)) {
                            return false;
                        }
                    }
                    crate::instruction::MiscAluOp::Xor => {
                        // EDUCATIONAL: C.XOR - compressed XOR
                        let lhs = match self.read_reg(rd) {
                            Some(v) => v,
                            None => return false,
                        };
                        let rhs = match self.read_reg(rs2) {
                            Some(v) => v,
                            None => return false,
                        };
                        if !self.write_reg(rd, lhs ^ rhs) {
                            return false;
                        }
                    }
                    crate::instruction::MiscAluOp::Or => {
                        // EDUCATIONAL: C.OR - compressed OR
                        let lhs = match self.read_reg(rd) {
                            Some(v) => v,
                            None => return false,
                        };
                        let rhs = match self.read_reg(rs2) {
                            Some(v) => v,
                            None => return false,
                        };
                        if !self.write_reg(rd, lhs | rhs) {
                            return false;
                        }
                    }
                    crate::instruction::MiscAluOp::And => {
                        // EDUCATIONAL: C.AND - compressed AND
                        let lhs = match self.read_reg(rd) {
                            Some(v) => v,
                            None => return false,
                        };
                        let rhs = match self.read_reg(rs2) {
                            Some(v) => v,
                            None => return false,
                        };
                        if !self.write_reg(rd, lhs & rhs) {
                            return false;
                        }
                    }
                }
            }
            Instruction::Fence => {
                // FENCE is a memory barrier in hardware, but is a no-op in this VM
            }
            Instruction::Unimp => {
                // UNIMP is an unimplemented instruction, treat as a no-op for compatibility
            }
            // ===== RV32A (Atomics) =====
            Instruction::AmoswapW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                if !memory.store_u32(addr, src, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmoaddW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = orig.wrapping_add(src);
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmoandW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = orig & src;
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmoorW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = orig | src;
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmoxorW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = orig ^ src;
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmomaxW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = if (orig as i32) > (src as i32) {
                    orig
                } else {
                    src
                };
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmominW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = if (orig as i32) < (src as i32) {
                    orig
                } else {
                    src
                };
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmomaxuW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = if orig > src { orig } else { src };
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            Instruction::AmominuW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let src = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let orig =
                    match memory.load_u32(addr, self.metering.as_mut(), MemoryAccessKind::Atomic) {
                        Some(v) => v,
                        None => return false,
                    };
                let new_val = if orig < src { orig } else { src };
                if !memory.store_u32(
                    addr,
                    new_val,
                    self.metering.as_mut(),
                    MemoryAccessKind::Atomic,
                ) {
                    return false;
                }
                if !self.write_reg(rd, orig) {
                    return false;
                }
            }
            // ===== RV32A (LR/SC) =====
            Instruction::LrW { rd, rs1 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let value = match memory.load_u32(
                    addr,
                    self.metering.as_mut(),
                    MemoryAccessKind::ReservationLoad,
                ) {
                    Some(v) => v,
                    None => return false,
                };
                if !self.write_reg(rd, value) {
                    return false;
                }
                // Set reservation for this address
                self.reservation_addr = Some(addr);
            }
            Instruction::ScW { rd, rs1, rs2 } => {
                let base = match self.read_reg(rs1) {
                    Some(v) => v,
                    None => return false,
                };
                let addr = VirtualAddress(base);
                let value_to_store = match self.read_reg(rs2) {
                    Some(v) => v,
                    None => return false,
                };

                // Check if we have a valid reservation for this address
                if self.reservation_addr == Some(addr) {
                    // Reservation is valid, perform the store
                    if !memory.store_u32(
                        addr,
                        value_to_store,
                        self.metering.as_mut(),
                        MemoryAccessKind::ReservationStore,
                    ) {
                        return false;
                    }
                    if !self.write_reg(rd, 0) {
                        return false;
                    } // 0 = success
                      // Clear the reservation (it's consumed)
                    self.reservation_addr = None;
                } else {
                    // No valid reservation, fail
                    if !self.write_reg(rd, 1) {
                        return false;
                    } // 1 = failure
                }
            }
        }
        true
    }
}
