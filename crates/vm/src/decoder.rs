use crate::instruction::{CsrOp, Instruction, MiscAluOp};
use crate::isa::Opcode;
use crate::isa_compressed::CompressedOpcode;

/// Unified decoder for either 16-bit compressed or 32-bit instruction.
///
/// EDUCATIONAL PURPOSE: This function demonstrates the first step of the
/// instruction cycle - instruction decoding. It takes raw bytes from memory
/// and converts them into structured instruction objects that the CPU can execute.
///
/// RISC-V INSTRUCTION FORMATS:
/// - 32-bit instructions: Standard RISC-V instructions
/// - 16-bit compressed instructions: Space-saving variants (RV32C extension)
///
/// COMPRESSED INSTRUCTION DETECTION:
/// RISC-V compressed instructions have bottom 2 bits != 0b11, while regular
/// instructions have bottom 2 bits = 0b11. This allows the decoder to quickly
/// determine which format to use.
///
/// BINARY FORMAT ANALYSIS:
/// The decoder examines the binary pattern of instruction bytes to determine:
/// 1. Whether it's a compressed (16-bit) or regular (32-bit) instruction
/// 2. What operation to perform (opcode)
/// 3. Which registers to use (rs1, rs2, rd)
/// 4. What immediate values to use (offsets, constants)
///
/// MEMORY EFFICIENCY: Compressed instructions save 50% of memory space
/// for common operations. This is especially important in embedded systems
/// where memory is limited and expensive.
///
/// DECODING STRATEGY: The function uses a two-step approach:
/// 1. Quick format detection using the bottom 2 bits
/// 2. Detailed decoding based on the detected format
///
/// ERROR HANDLING: Returns None for invalid or unrecognized instructions.
/// This allows the CPU to handle malformed code gracefully.
///
/// PARAMETERS:
/// - bytes: Raw instruction bytes from memory (at least 2 bytes)
///
/// RETURNS: Some((instruction, size)) if successful, None if invalid
/// - instruction: The decoded instruction object
/// - size: Number of bytes consumed (2 for compressed, 4 for regular)
pub fn decode(bytes: &[u8]) -> Option<(Instruction, u8)> {
    // EDUCATIONAL: Need at least 2 bytes to read the first 16 bits
    if bytes.len() < 2 {
        return None;
    }

    // EDUCATIONAL: Read the first 16 bits to check if it's compressed
    let hword = u16::from_le_bytes([bytes[0], bytes[1]]);

    // EDUCATIONAL: Check bottom 2 bits to determine instruction format
    // 0b11 = regular 32-bit instruction, anything else = compressed
    let is_compressed = (hword & 0b11) != 0b11;

    if is_compressed {
        // EDUCATIONAL: Decode as 16-bit compressed instruction
        decode_compressed(hword).map(|inst| (inst, 2))
    } else if bytes.len() >= 4 {
        // EDUCATIONAL: Decode as 32-bit regular instruction
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        decode_full(word).map(|inst| (inst, 4))
    } else {
        // EDUCATIONAL: Not enough bytes for a 32-bit instruction
        None
    }
}

/// Decodes a 32-bit RISC-V instruction into an Instruction object.
///
/// EDUCATIONAL PURPOSE: This function demonstrates RISC-V instruction encoding.
/// RISC-V uses a fixed 32-bit instruction format with specific bit fields
/// for different instruction components.
///
/// RISC-V INSTRUCTION FORMAT:
/// ```text
/// 31:25  funct7  (7 bits) - function code for register-register ops
/// 24:20  rs2     (5 bits) - second source register
/// 19:15  rs1     (5 bits) - first source register  
/// 14:12  funct3  (3 bits) - function code for immediate ops
/// 11:7   rd      (5 bits) - destination register
/// 6:0    opcode  (7 bits) - operation code
/// ```
///
/// INSTRUCTION TYPE ANALYSIS:
/// The opcode field determines the instruction type and how to interpret
/// the remaining fields:
/// - R-type: Register operations (add, sub, and, or, etc.)
/// - I-type: Immediate operations (addi, lw, jalr, etc.)
/// - S-type: Store operations (sw, sh, sb)
/// - B-type: Branch operations (beq, bne, blt, etc.)
/// - U-type: Upper immediate operations (lui, auipc)
/// - J-type: Jump operations (jal)
///
/// IMMEDIATE EXTRACTION: Different instruction types pack immediate values
/// in different bit positions. This function extracts them correctly for each type.
/// - I-type: 12-bit immediate in bits 31:20
/// - S-type: 12-bit immediate split across bits 31:25 and 11:7
/// - B-type: 13-bit immediate split across bits 31:25 and 11:8
/// - U-type: 20-bit immediate in bits 31:12
/// - J-type: 21-bit immediate split across bits 31:12 and 20:12
///
/// BIT MANIPULATION TECHNIQUES:
/// The function uses bit shifting and masking to extract fields:
/// - (word >> n) & mask: Extract n bits starting at position n
/// - (word as i32) >> 20: Sign-extend 12-bit immediate
/// - ((a << n) | b) << m >> m: Combine split fields and sign-extend
///
/// FUNCTION CODES: The funct3 and funct7 fields provide additional
/// information to distinguish between similar operations:
/// - funct3: Used for immediate operations and memory operations
/// - funct7: Used for register-register operations
///
/// PARAMETERS:
/// - word: 32-bit instruction word from memory
///
/// RETURNS: Some(instruction) if valid, None if unrecognized
pub fn decode_full(word: u32) -> Option<Instruction> {
    // Null bytes (padding) - treat as no-op
    if word == 0x00000000 {
        return Some(Instruction::Unimp);
    }
    // FENCE: 0x0ff0000f
    if word == 0x0ff0000f {
        return Some(Instruction::Fence);
    }
    // FENCE.I: 0x100f
    if word == 0x100f {
        return Some(Instruction::Fence);
    }
    // UNIMP: 0x0000000f (common convention)
    if word == 0x0000000f {
        return Some(Instruction::Unimp);
    }

    // EDUCATIONAL: Extract instruction fields
    let opcode_raw = word & 0x7f; // bits 0-6
    let rd = ((word >> 7) & 0x1f) as usize; // bits 7-11
    let funct3 = ((word >> 12) & 0x7) as u8; // bits 12-14
    let rs1 = ((word >> 15) & 0x1f) as usize; // bits 15-19
    let rs2 = ((word >> 20) & 0x1f) as usize; // bits 20-24
    let funct7 = ((word >> 25) & 0x7f) as u8; // bits 25-31

    // EDUCATIONAL: Extract opcode from bottom 7 bits
    let opcode = Opcode::from_u8(opcode_raw as u8)?;

    match opcode {
        // EDUCATIONAL: Register-register operations (R-type)
        // These instructions use rs1, rs2, and rd registers
        Opcode::Op => match (funct3, funct7) {
            // EDUCATIONAL: Basic arithmetic operations
            (0x0, 0x00) => Some(Instruction::Add { rd, rs1, rs2 }),
            (0x0, 0x20) => Some(Instruction::Sub { rd, rs1, rs2 }),

            // EDUCATIONAL: Logical operations
            (0x1, 0x00) => Some(Instruction::Sll { rd, rs1, rs2 }),
            (0x4, 0x00) => Some(Instruction::Xor { rd, rs1, rs2 }),
            (0x5, 0x00) => Some(Instruction::Srl { rd, rs1, rs2 }),
            (0x5, 0x20) => Some(Instruction::Sra { rd, rs1, rs2 }),
            (0x6, 0x00) => Some(Instruction::Or { rd, rs1, rs2 }),
            (0x7, 0x00) => Some(Instruction::And { rd, rs1, rs2 }),

            // EDUCATIONAL: Comparison operations
            (0x2, 0x00) => Some(Instruction::Slt { rd, rs1, rs2 }),
            (0x3, 0x00) => Some(Instruction::Sltu { rd, rs1, rs2 }),

            // EDUCATIONAL: Extended arithmetic (M extension)
            (0x0, 0x01) => Some(Instruction::Mul { rd, rs1, rs2 }),
            (0x1, 0x01) => Some(Instruction::Mulh { rd, rs1, rs2 }),
            (0x2, 0x01) => Some(Instruction::Mulhsu { rd, rs1, rs2 }),
            (0x3, 0x01) => Some(Instruction::Mulhu { rd, rs1, rs2 }),
            (0x4, 0x01) => Some(Instruction::Div { rd, rs1, rs2 }),
            (0x5, 0x01) => Some(Instruction::Divu { rd, rs1, rs2 }),
            (0x6, 0x01) => Some(Instruction::Rem { rd, rs1, rs2 }),
            (0x7, 0x01) => Some(Instruction::Remu { rd, rs1, rs2 }),
            _ => None,
        },

        // EDUCATIONAL: Atomic Memory Operations (A extension)
        // These instructions use rs1, rs2, and rd registers
        Opcode::Amo => {
            if funct3 == 0x2 {
                match funct7 {
                    // LR.W: funct7=0x02, rs2=0
                    0x02 => Some(Instruction::LrW { rd, rs1 }),
                    // SC.W: funct7=0x03, rs2≠0
                    0x03 => Some(Instruction::ScW { rd, rs1, rs2 }),
                    // AMO instructions
                    0x01 => Some(Instruction::AmoswapW { rd, rs1, rs2 }),
                    0x00 => Some(Instruction::AmoaddW { rd, rs1, rs2 }),
                    0x04 => Some(Instruction::AmoswapW { rd, rs1, rs2 }),
                    0x08 => Some(Instruction::AmoorW { rd, rs1, rs2 }),
                    0x0C => Some(Instruction::ScW { rd, rs1, rs2 }), // Alternative SC encoding
                    0x10 => Some(Instruction::AmoxorW { rd, rs1, rs2 }),
                    0x14 => Some(Instruction::AmomaxW { rd, rs1, rs2 }),
                    0x18 => Some(Instruction::AmominuW { rd, rs1, rs2 }),
                    0x1C => Some(Instruction::AmomaxuW { rd, rs1, rs2 }),
                    0x20 => Some(Instruction::AmoorW { rd, rs1, rs2 }),
                    0x30 => Some(Instruction::AmoandW { rd, rs1, rs2 }),
                    0x40 => Some(Instruction::AmominW { rd, rs1, rs2 }),
                    0x50 => Some(Instruction::AmomaxW { rd, rs1, rs2 }),
                    0x60 => Some(Instruction::AmominuW { rd, rs1, rs2 }),
                    0x70 => Some(Instruction::AmomaxuW { rd, rs1, rs2 }),
                    _ => None,
                }
            } else {
                None
            }
        }

        // EDUCATIONAL: Immediate operations (I-type)
        // These instructions use rs1, immediate, and rd
        Opcode::OpImm => {
            // EDUCATIONAL: Sign-extend 12-bit immediate from bits 31:20
            let imm = (word as i32) >> 20;
            match funct3 {
                // EDUCATIONAL: Basic immediate arithmetic
                0x0 => Some(Instruction::Addi { rd, rs1, imm }),

                // EDUCATIONAL: Immediate comparisons
                0x2 => Some(Instruction::Slti { rd, rs1, imm }),
                0x3 => Some(Instruction::Sltiu { rd, rs1, imm }),

                // EDUCATIONAL: Immediate logical operations
                0x4 => Some(Instruction::Xori { rd, rs1, imm }),
                0x6 => Some(Instruction::Ori { rd, rs1, imm }),
                0x7 => Some(Instruction::Andi { rd, rs1, imm }),

                // EDUCATIONAL: Immediate shifts (use only bottom 5 bits of immediate)
                0x1 => Some(Instruction::Slli {
                    rd,
                    rs1,
                    shamt: (imm & 0x1f) as u8, // Only bottom 5 bits for shift amount
                }),
                0x5 => match funct7 {
                    // EDUCATIONAL: Logical vs arithmetic right shift
                    0x00 => Some(Instruction::Srli {
                        rd,
                        rs1,
                        shamt: (imm & 0x1f) as u8,
                    }),
                    0x20 => Some(Instruction::Srai {
                        rd,
                        rs1,
                        shamt: (imm & 0x1f) as u8,
                    }),
                    _ => None,
                },
                _ => None,
            }
        }

        // EDUCATIONAL: Load operations (I-type)
        // Load data from memory into register
        Opcode::Load => {
            // EDUCATIONAL: Sign-extend 12-bit immediate for address offset
            let imm = (word as i32) >> 20;
            match funct3 {
                0x0 => Some(Instruction::Lb {
                    // Load byte (8-bit, sign-extended)
                    rd,
                    rs1,
                    offset: imm,
                }),
                0x1 => Some(Instruction::Lh {
                    // Load halfword (16-bit)
                    rd,
                    rs1,
                    offset: imm,
                }),
                0x2 => Some(Instruction::Lw {
                    // Load word (32-bit)
                    rd,
                    rs1,
                    offset: imm,
                }),
                0x3 => Some(Instruction::Ld {
                    // Load doubleword (64-bit)
                    rd,
                    rs1,
                    offset: imm,
                }),
                0x4 => Some(Instruction::Lbu {
                    rd,
                    rs1,
                    offset: imm,
                }), // Load byte unsigned
                0x5 => Some(Instruction::Lhu {
                    rd,
                    rs1,
                    offset: imm,
                }), // Load halfword unsigned
                _ => None,
            }
        }

        // EDUCATIONAL: Store operations (S-type)
        // Store data from register to memory
        Opcode::Store => {
            // EDUCATIONAL: S-type immediates are split across the instruction
            // imm[11:5] is in bits 31:25, imm[4:0] is in bits 11:7
            let imm11_5 = ((word >> 25) & 0x7f) << 5;
            let imm4_0 = (word >> 7) & 0x1f;
            let imm = ((imm11_5 | imm4_0) as i32) << 20 >> 20; // sign-extend 12-bit

            match funct3 {
                0x0 => Some(Instruction::Sb {
                    // Store byte (8-bit)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x1 => Some(Instruction::Sh {
                    // Store halfword (16-bit)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x2 => Some(Instruction::Sw {
                    // Store word (32-bit)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                _ => None,
            }
        }

        // EDUCATIONAL: Branch operations (B-type)
        // Conditional jumps based on register comparison
        Opcode::Branch => {
            // EDUCATIONAL: B-type immediates are also split and sign-extended
            let imm = extract_branch_offset(word);
            match funct3 {
                0x0 => Some(Instruction::Beq {
                    // Branch if equal
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x1 => Some(Instruction::Bne {
                    // Branch if not equal
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x4 => Some(Instruction::Blt {
                    // Branch if less than (signed)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x5 => Some(Instruction::Bge {
                    // Branch if greater/equal (signed)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x6 => Some(Instruction::Bltu {
                    // Branch if less than (unsigned)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                0x7 => Some(Instruction::Bgeu {
                    // Branch if greater/equal (unsigned)
                    rs1,
                    rs2,
                    offset: imm,
                }),
                _ => None,
            }
        }

        // EDUCATIONAL: Jump and Link (J-type)
        // Unconditional jump with return address saved
        Opcode::Jal => {
            // EDUCATIONAL: J-type immediates are 20-bit signed values
            let imm = extract_jal_offset(word);
            Some(Instruction::Jal {
                rd,
                offset: imm,
                compressed: false,
            })
        }

        // EDUCATIONAL: Jump and Link Register (I-type)
        // Indirect jump with return address saved
        Opcode::Jalr => {
            // EDUCATIONAL: 12-bit signed immediate for address offset
            let imm = (word as i32) >> 20;
            Some(Instruction::Jalr {
                rd,
                rs1,
                offset: imm,
                compressed: false,
            })
        }

        // EDUCATIONAL: Load Upper Immediate (U-type)
        // Load 20-bit immediate into upper bits of register
        Opcode::Lui => {
            // EDUCATIONAL: 20-bit immediate goes into bits 31:12
            let imm = ((word >> 12) & 0xFFFFF) as i32;
            Some(Instruction::Lui {
                rd,
                imm: imm as i32,
            })
        }

        // EDUCATIONAL: Add Upper Immediate to PC (U-type)
        // PC-relative addressing for position-independent code
        Opcode::Auipc => {
            // EDUCATIONAL: 20-bit immediate added to PC (bits 31:12)
            let imm = ((word >> 12) & 0xfffff) as i32;
            Some(Instruction::Auipc { rd, imm })
        }

        // EDUCATIONAL: System instructions (ecall/ebreak and CSRs)
        Opcode::System => {
            let funct3 = (word >> 12) & 0x7;
            let csr = (word >> 20) & 0xfff;
            let rs1 = ((word >> 15) & 0x1f) as usize;

            match funct3 {
                0 => {
                    let funct12 = word >> 20;
                    match funct12 {
                        0 => Some(Instruction::Ecall),
                        1 => Some(Instruction::Ebreak),
                        0x302 => Some(Instruction::Mret),
                        0x102 => Some(Instruction::Sret),
                        _ => None,
                    }
                }
                1 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrw,
                    imm: false,
                }),
                2 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrs,
                    imm: false,
                }),
                3 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrc,
                    imm: false,
                }),
                5 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrw,
                    imm: true,
                }),
                6 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrs,
                    imm: true,
                }),
                7 => Some(Instruction::Csr {
                    rd,
                    rs1,
                    csr: csr as u16,
                    op: CsrOp::Csrrc,
                    imm: true,
                }),
                _ => None,
            }
        }
    }
}

/// Decode a 16-bit RISC-V compressed instruction into a full Instruction.
///
/// EDUCATIONAL PURPOSE: This demonstrates RISC-V compressed instruction decoding.
/// Compressed instructions save memory by using shorter encodings for common
/// operations, but they're more complex to decode.
///
/// COMPRESSED INSTRUCTION FORMATS:
/// - CI: Compressed Immediate (addi, li, etc.)
/// - CL: Compressed Load (lw, etc.)
/// - CS: Compressed Store (sw, etc.)
/// - CJ: Compressed Jump (jal, etc.)
/// - CB: Compressed Branch (beqz, etc.)
///
/// REGISTER MAPPING: Compressed instructions use a subset of registers
/// (x8-x15) to save bits, and have special encodings for common registers
/// like x0 (zero), x1 (ra), x2 (sp).
///
/// This function currently supports a minimal set:
/// - C.ADDI, C.LI, C.LW, C.SW, C.JAL, C.JR, C.RET
pub fn decode_compressed(hword: u16) -> Option<Instruction> {
    let funct3 = (hword >> 13) & 0b111;
    let opcode = hword & 0b11;
    let rd = ((hword >> 7) & 0x1f) as usize;
    let rs1 = rd;
    let rs2 = ((hword >> 2) & 0x1f) as usize;

    let op = CompressedOpcode::from_bits(funct3, opcode)?;

    match op {
        CompressedOpcode::Addi => {
            let imm = (((hword >> 2) & 0b11111) | (((hword >> 12) & 0x1) << 5)) as i32;
            let imm = (imm << 26) >> 26; // sign-extend
            Some(Instruction::Addi { rd, rs1, imm })
        }

        CompressedOpcode::Li => {
            let imm = (((hword >> 2) & 0b11111) | (((hword >> 12) & 0x1) << 5)) as i32;
            let imm = (imm << 26) >> 26;
            Some(Instruction::Addi { rd, rs1: 0, imm })
        }

        CompressedOpcode::LuiOrAddi16sp => {
            if rd == 2 {
                // C.ADDI16SP
                let imm2 = (((hword >> 12) & 0x1) << 9
                    | ((hword >> 6) & 0x1) << 4
                    | ((hword >> 5) & 0x1) << 6
                    | ((hword >> 4) & 0x1) << 8
                    | ((hword >> 3) & 0x1) << 7
                    | ((hword >> 2) & 0x1) << 5) as i32;

                // Sign-extend 10-bit immediate
                let imm = (imm2 << 22) >> 22;

                Some(Instruction::Addi16sp { imm })
            } else if rd != 0 {
                // C.LUI
                // According to spec:
                // - imm[5] (bit 12) = nzimm[17] (sign bit)
                // - imm[4:0] (bits 6:2) = nzimm[16:12]
                let nzimm_17 = (hword >> 12) & 0x1; // nzimm[17] from bit 12
                let nzimm_16_12 = (hword >> 2) & 0x1F; // nzimm[16:12] from bits 6:2
                let nzimm = (nzimm_17 << 5) | nzimm_16_12; // Combine into 6-bit nzimm[17:12]

                // C.LUI expands into lui rd, nzimm[17:12]
                // The 6-bit nzimm is used as imm[17:12] of the 20-bit LUI immediate
                // Sign-extend bit 17 (nzimm[17]) into all higher bits [31:18]
                let signed_nzimm = ((nzimm as i32) << 26) >> 26;

                Some(Instruction::Lui {
                    rd,
                    imm: signed_nzimm,
                })
            } else {
                None
            }
        }

        CompressedOpcode::Addi4spn => {
            let _rd = 8 + ((hword >> 2) & 0b111) as i32; // rd' field

            if _rd == 0 {
                return None; // reserved
            }

            let imm = ((hword >> 12) & 0b1) << 5  | // imm[5]  from bit 12
                ((hword >> 11) & 0b1) << 4  | // imm[4]  from bit 11
                ((hword >> 10) & 0b1) << 9  | // imm[9]  from bit 10
                ((hword >>  9) & 0b1) << 8  | // imm[8]  from bit 9
                ((hword >>  8) & 0b1) << 7  | // imm[7]  from bit 8
                ((hword >>  7) & 0b1) << 6  | // imm[6]  from bit 7
                ((hword >>  6) & 0b1) << 2  | // imm[2]  from bit 6
                ((hword >>  5) & 0b1) << 3; // imm[3]  from bit 5

            if imm == 0 {
                return None; // nzuimm must be non-zero
            }

            Some(Instruction::Addi4spn {
                rd: _rd as usize,
                imm: imm as u32,
            })
        }

        CompressedOpcode::Jal => {
            let imm = decode_cj_imm(hword); // implement CJ immediate decoder
            Some(Instruction::Jal {
                rd: 1,
                offset: imm,
                compressed: true,
            }) // rd = x1
        }

        CompressedOpcode::J => {
            let imm = decode_cj_imm(hword);
            Some(Instruction::Jal {
                rd: 0,
                offset: imm,
                compressed: true,
            }) // jump without link
        }

        CompressedOpcode::RegOrJump => {
            let bit12 = (hword >> 12) & 0b1;

            // === EBREAK: rs1 = 0 and rs2 = 0 (regardless of bit12) ===
            if rs1 == 0 && rs2 == 0 {
                return Some(Instruction::Ebreak);
            }

            if bit12 == 0 {
                // === Bit 12 = 0: JR, RET, MV ===
                match (rs1, rs2) {
                    (1, 0) => Some(Instruction::Ret),               // C.RET
                    (rs1, 0) => Some(Instruction::Jr { rs1 }),      // C.JR
                    (rd, rs2) => Some(Instruction::Mv { rd, rs2 }), // C.MV
                }
            } else {
                // === Bit 12 = 1: JALR, ADD ===
                match (rs1, rs2) {
                    (_, 0) => Some(Instruction::Jalr {
                        rd: 1,
                        rs1: rs1,
                        offset: 0,
                        compressed: true,
                    }), // C.JALR (implicit rd = x1)
                    (rd, rs2) => Some(Instruction::Add { rd, rs1: rd, rs2 }), // C.ADD
                }
            }
        }

        CompressedOpcode::Slli => {
            let shamt = ((hword >> 2) & 0b11111) as u8;
            Some(Instruction::Slli {
                rd,
                rs1,
                shamt: shamt,
            }) // emulate as ADDI with left shift beforehand
        }

        CompressedOpcode::Lwsp => {
            let rd = ((hword >> 7) & 0x1F) as usize;
            if rd == 0 {
                return None; // rd must not be x0
            }

            // Reconstruct immediate as: [7:6][5][4:2]
            let imm = ((hword >> 2) & 0b11) << 6   // bits 3:2 -> imm[7:6]
            | ((hword >> 12) & 0x1) << 5   // bit 12   -> imm[5]
            | ((hword >> 4) & 0b111) << 2; // bits 6:4 -> imm[4:2]

            let imm = imm as i32;

            Some(Instruction::Lw {
                rd,
                rs1: 2,
                offset: imm,
            }) // x2 is sp
        }

        CompressedOpcode::Beqz | CompressedOpcode::Bnez => {
            let rs1 = 8 + ((hword >> 7) & 0b111) as usize; // rs1' field

            // Decode CB-format immediate for compressed branches
            let imm = (
                ((hword >> 12) & 0x1) << 8 |  // imm[8]
                ((hword >> 10) & 0x3) << 3 |  // imm[4:3]
                ((hword >> 5) & 0x3) << 6 |   // imm[7:6]
                ((hword >> 3) & 0x3) << 1 |   // imm[2:1]
                ((hword >> 2) & 0x1) << 5
                // imm[5]
            ) as i32;

            // Sign extend the 9-bit immediate
            let imm = (imm << 23) >> 23;

            match op {
                CompressedOpcode::Beqz => Some(Instruction::Beqz { rs1, offset: imm }),
                CompressedOpcode::Bnez => Some(Instruction::Bnez { rs1, offset: imm }),
                _ => unreachable!(),
            }
        }

        CompressedOpcode::Sw => {
            let rd_ = ((hword >> 2) & 0b111) + 8; // rs2'
            let rs1_ = ((hword >> 7) & 0b111) + 8; // rs1'

            // Fix: Correct immediate extraction for C.SW according to RISC-V spec
            // imm[6] from bit 5, imm[5:3] from bits 12-10, imm[2] from bit 6
            let imm = ((hword >> 5) & 0b1) << 6      // imm[6] from bit 5
                    | ((hword >> 10) & 0b111) << 3    // imm[5:3] from bits 12-10
                    | ((hword >> 6) & 0b1) << 2; // imm[2] from bit 6
            let offset = imm as i32; // no sign-extension needed

            Some(Instruction::Sw {
                rs1: rs1_ as usize,
                rs2: rd_ as usize,
                offset,
            })
        }

        CompressedOpcode::Lw => {
            let rd_ = ((hword >> 2) & 0b111) + 8; // rd'
            let rs1_ = ((hword >> 7) & 0b111) + 8; // rs1'

            // imm[5|4:3|2] → bits: [5] bit 5 = bit 5, [4:3] bits 11:10, [2] bit 6
            let imm = ((hword >> 6) & 0b1) << 2      // imm[2]
                    | ((hword >> 10) & 0b11) << 3    // imm[4:3]
                    | ((hword >> 5) & 0b1) << 6; // imm[5]

            let offset = imm as i32;

            Some(Instruction::Lw {
                rd: rd_ as usize,
                rs1: rs1_ as usize,
                offset,
            })
        }

        // ...other compressed cases...
        CompressedOpcode::MiscAlu => {
            let funct2 = (hword >> 10) & 0b11; // bits 11–10
            let bit12 = (hword >> 12) & 0x1;
            let rd = 8 + ((hword >> 7) & 0b111) as usize;
            let rs2 = 8 + ((hword >> 2) & 0b111) as usize;

            match funct2 {
                0b00 => {
                    // C.SRLI
                    let shamt = ((bit12 << 5) | ((hword >> 2) & 0x1f)) as u8;
                    Some(Instruction::Srli { rd, rs1: rd, shamt })
                }
                0b01 => {
                    // C.SRAI
                    let shamt = ((bit12 << 5) | ((hword >> 2) & 0x1f)) as u8;
                    Some(Instruction::Srai { rd, rs1: rd, shamt })
                }
                0b10 => {
                    // ✅ C.ANDI (bit12 is imm[5], NOT format discriminator)
                    let imm_raw = ((bit12 << 5) | ((hword >> 2) & 0x1f)) as i32;
                    let imm = (imm_raw << 26) >> 26; // sign-extend 6-bit
                    Some(Instruction::Andi { rd, rs1: rd, imm })
                }
                0b11 => {
                    // CA-format — check bits [6:5] (rs2′)
                    let op = match (hword >> 5) & 0b11 {
                        0b00 => MiscAluOp::Sub,
                        0b01 => MiscAluOp::Xor,
                        0b10 => MiscAluOp::Or,
                        0b11 => MiscAluOp::And,
                        _ => return None,
                    };
                    Some(Instruction::MiscAlu { rd, rs2, op })
                }
                _ => None,
            }
        }

        CompressedOpcode::Swsp => {
            let rs2 = ((hword >> 2) & 0x1F) as usize;

            let imm = (((hword >> 12) & 0x1) << 5) | // bit 12 → imm[5]
                (((hword >> 11) & 0x1) << 4) | // bit 11 → imm[4]
                (((hword >> 10) & 0x1) << 3) | // bit 10 → imm[3]
                (((hword >>  9) & 0x1) << 2) | // bit 9  → imm[2]
                (((hword >>  8) & 0x1) << 7) | // bit 8  → imm[7]
                (((hword >>  7) & 0x1) << 6); // bit 7  → imm[6]

            let offset = imm as i32;

            Some(Instruction::Sw {
                rs1: 2, // sp
                rs2,
                offset,
            })
        }
    }
}

fn decode_cj_imm(hword: u16) -> i32 {
    let imm = (((hword >> 12) & 0b1) << 11
        | ((hword >> 11) & 0b1) << 4
        | ((hword >> 9) & 0b11) << 8
        | ((hword >> 8) & 0b1) << 10
        | ((hword >> 7) & 0b1) << 6
        | ((hword >> 6) & 0b1) << 7
        | ((hword >> 3) & 0b111) << 1
        | ((hword >> 2) & 0b1) << 5) as i32;
    (imm << 20) >> 20 // sign-extend 12-bit
}

fn extract_branch_offset(word: u32) -> i32 {
    let imm12 = ((word >> 31) & 0x1) << 12;
    let imm10_5 = ((word >> 25) & 0x3f) << 5;
    let imm4_1 = ((word >> 8) & 0xf) << 1;
    let imm11 = ((word >> 7) & 0x1) << 11;
    let imm = (imm12 | imm11 | imm10_5 | imm4_1) as i32;
    (imm << 19) >> 19
}

fn extract_jal_offset(word: u32) -> i32 {
    let imm20 = ((word >> 31) & 0x1) << 20;
    let imm10_1 = ((word >> 21) & 0x3ff) << 1;
    let imm11 = ((word >> 20) & 0x1) << 11;
    let imm19_12 = ((word >> 12) & 0xff) << 12;
    let imm = (imm20 | imm19_12 | imm11 | imm10_1) as i32;
    (imm << 11) >> 11
}
