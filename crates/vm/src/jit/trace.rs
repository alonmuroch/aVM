use crate::decoder::{decode_compressed, decode_full};
use crate::instruction::Instruction;
use crate::memory::{Memory, VirtualAddress};

const TRACE_LIMIT: usize = 64;

/// A decoded instruction plus the metadata needed for codegen.
#[derive(Debug)]
pub struct TraceInst {
    pub pc: u32,
    pub size: u8,
    pub instr: Instruction,
}

/// A linear sequence of supported instructions starting at a hot PC.
/// Traces stop at control-flow boundaries or when an unsupported op appears.
#[derive(Debug)]
pub struct Trace {
    pub start_pc: u32,
    pub instrs: Vec<TraceInst>,
}

impl Trace {
    /// Build a trace by decoding from `start_pc` until a stop condition is hit.
    /// If the first instruction is unsupported or decoding fails, returns None.
    pub fn build(start_pc: u32, memory: &Memory) -> Option<Self> {
        let mut instrs = Vec::new();
        let mut pc = start_pc;

        for _ in 0..TRACE_LIMIT {
            let (instr, size) = decode_at(memory, pc)?;
            if !is_supported(&instr) {
                break;
            }
            instrs.push(TraceInst { pc, size, instr });
            pc = pc.wrapping_add(size as u32);

            if ends_trace(instrs.last().map(|inst| &inst.instr).unwrap()) {
                break;
            }
        }

        if instrs.is_empty() {
            return None;
        }

        Some(Self { start_pc, instrs })
    }
}

/// True if the instruction should terminate a trace.
pub fn ends_trace(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Beq { .. }
            | Instruction::Bne { .. }
            | Instruction::Blt { .. }
            | Instruction::Bge { .. }
            | Instruction::Bltu { .. }
            | Instruction::Bgeu { .. }
            | Instruction::Jal { .. }
            | Instruction::Jalr { .. }
    )
}

/// True if the instruction is a conditional branch (used to tailor codegen).
pub fn is_branch(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Beq { .. }
            | Instruction::Bne { .. }
            | Instruction::Blt { .. }
            | Instruction::Bge { .. }
            | Instruction::Bltu { .. }
            | Instruction::Bgeu { .. }
    )
}

/// Fetch and decode a single instruction at `pc`.
/// Handles compressed (16-bit) and full-width (32-bit) forms.
fn decode_at(memory: &Memory, pc: u32) -> Option<(Instruction, u8)> {
    let pc_va = VirtualAddress(pc);
    let end_va = VirtualAddress(pc.wrapping_add(4));
    let bytes = memory.mem_slice(pc_va, end_va)?;
    if bytes.len() < 2 {
        return None;
    }
    let hword = u16::from_le_bytes([bytes[0], bytes[1]]);
    let is_compressed = (hword & 0b11) != 0b11;
    if is_compressed {
        decode_compressed(hword).map(|inst| (inst, 2))
    } else if bytes.len() >= 4 {
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        decode_full(word).map(|inst| (inst, 4))
    } else {
        None
    }
}

/// Only a subset of RV32IMAC is currently JIT compiled; everything else
/// falls back to the interpreter.
fn is_supported(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Add { .. }
            | Instruction::Sub { .. }
            | Instruction::Addi { .. }
            | Instruction::And { .. }
            | Instruction::Or { .. }
            | Instruction::Xor { .. }
            | Instruction::Andi { .. }
            | Instruction::Ori { .. }
            | Instruction::Xori { .. }
            | Instruction::Slt { .. }
            | Instruction::Sltu { .. }
            | Instruction::Slti { .. }
            | Instruction::Sltiu { .. }
            | Instruction::Sll { .. }
            | Instruction::Srl { .. }
            | Instruction::Sra { .. }
            | Instruction::Slli { .. }
            | Instruction::Srli { .. }
            | Instruction::Srai { .. }
            | Instruction::Lw { .. }
            | Instruction::Ld { .. }
            | Instruction::Lb { .. }
            | Instruction::Lbu { .. }
            | Instruction::Lh { .. }
            | Instruction::Lhu { .. }
            | Instruction::Sh { .. }
            | Instruction::Sw { .. }
            | Instruction::Sb { .. }
            | Instruction::Lui { .. }
            | Instruction::Auipc { .. }
            | Instruction::Mul { .. }
            | Instruction::Mulh { .. }
            | Instruction::Mulhu { .. }
            | Instruction::Mulhsu { .. }
            | Instruction::Div { .. }
            | Instruction::Divu { .. }
            | Instruction::Rem { .. }
            | Instruction::Remu { .. }
            | Instruction::Beq { .. }
            | Instruction::Bne { .. }
            | Instruction::Blt { .. }
            | Instruction::Bge { .. }
            | Instruction::Bltu { .. }
            | Instruction::Bgeu { .. }
            | Instruction::Jal { .. }
            | Instruction::Jalr { .. }
            | Instruction::Fence
            | Instruction::Unimp
    )
}
