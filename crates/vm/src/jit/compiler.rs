use crate::cpu::CPU;
use crate::instruction::Instruction;
use crate::jit::helpers::register_helper_symbols;
use crate::jit::trace::{ends_trace, is_branch, Trace, TraceInst};
use crate::jit::{JitEntry, JitFn};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlags};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::mem::offset_of;

/// Function IDs for helper calls (register/memory/pc operations).
#[derive(Debug)]
struct HelperIds {
    load_u8_signed: FuncId,
    load_u8_unsigned: FuncId,
    load_u16_signed: FuncId,
    load_u16_unsigned: FuncId,
    load_u32: FuncId,
    store_u8: FuncId,
    store_u16: FuncId,
    store_u32: FuncId,
}

const CPU_PC_OFFSET: i32 = offset_of!(CPU, pc) as i32;
const CPU_REGS_OFFSET: i32 = offset_of!(CPU, regs) as i32;

/// Cranelift-backed compiler for traces.
pub struct JitCompiler {
    module: JITModule,
    builder_ctx: FunctionBuilderContext,
    helpers: HelperIds,
}

impl std::fmt::Debug for JitCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JitCompiler")
            .field("helpers", &self.helpers)
            .finish()
    }
}

impl JitCompiler {
    /// Create a compiler instance and register helper symbols.
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "false").expect("jit is_pic");
        flag_builder
            .set("use_colocated_libcalls", "false")
            .expect("jit colocated");
        let isa_builder = cranelift_native::builder().expect("jit isa builder");
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .expect("jit isa");
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        register_helper_symbols(&mut builder);

        let mut module = JITModule::new(builder);
        let helpers = declare_helpers(&mut module);

        Self {
            module,
            builder_ctx: FunctionBuilderContext::new(),
            helpers,
        }
    }

    /// Compile a trace into a callable JIT function and return its entry.
    pub fn compile_trace(&mut self, trace: &Trace) -> Option<JitEntry> {
        let ptr_ty = self.module.target_config().pointer_type();
        let mut ctx = self.module.make_context();
        ctx.func.signature.params.push(AbiParam::new(ptr_ty));
        ctx.func.signature.params.push(AbiParam::new(ptr_ty));
        ctx.func.signature.returns.push(AbiParam::new(types::I32));

        let func_name = format!("jit_trace_{:08x}", trace.start_pc);
        let func_id = self
            .module
            .declare_function(&func_name, Linkage::Local, &ctx.func.signature)
            .ok()?;

        let mut builder_ctx = std::mem::take(&mut self.builder_ctx);
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let cpu_ptr = builder.block_params(entry_block)[0];
        let mem_ptr = builder.block_params(entry_block)[1];
        let halt_block = builder.create_block();

        let mut needs_return = true;
        for inst in &trace.instrs {
            if !emit_instruction(
                &mut self.module,
                &self.helpers,
                &mut builder,
                cpu_ptr,
                mem_ptr,
                halt_block,
                inst,
            ) {
                break;
            }

            if ends_trace(&inst.instr) {
                if is_branch(&inst.instr) {
                    needs_return = false;
                }
                break;
            }
        }

        if needs_return {
            let one = builder.ins().iconst(types::I32, 1);
            builder.ins().return_(&[one]);
        }

        builder.switch_to_block(halt_block);
        builder.seal_block(halt_block);
        let zero = builder.ins().iconst(types::I32, 0);
        builder.ins().return_(&[zero]);

        builder.finalize();
        self.module.define_function(func_id, &mut ctx).ok()?;
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions();
        self.builder_ctx = builder_ctx;

        let code_ptr = self.module.get_finalized_function(func_id);
        let func = unsafe { std::mem::transmute::<*const u8, JitFn>(code_ptr) };
        Some(JitEntry { func })
    }
}

/// Emit code for a single instruction. Returns false if unsupported.
fn emit_instruction(
    module: &mut JITModule,
    helpers: &HelperIds,
    builder: &mut FunctionBuilder,
    cpu_ptr: cranelift_codegen::ir::Value,
    mem_ptr: cranelift_codegen::ir::Value,
    halt_block: cranelift_codegen::ir::Block,
    inst: &TraceInst,
) -> bool {
    match &inst.instr {
        Instruction::Add { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().iadd(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sub { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().isub(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Addi { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let imm_val = builder.ins().iconst(types::I32, *imm as i64);
            let val = builder.ins().iadd(lhs, imm_val);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::And { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().band(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Or { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().bor(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Xor { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().bxor(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Andi { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let imm_val = builder.ins().iconst(types::I32, *imm as i64);
            let val = builder.ins().band(lhs, imm_val);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Ori { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let imm_val = builder.ins().iconst(types::I32, *imm as i64);
            let val = builder.ins().bor(lhs, imm_val);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Xori { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let imm_val = builder.ins().iconst(types::I32, *imm as i64);
            let val = builder.ins().bxor(lhs, imm_val);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Slt { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
            let val = select_bool(builder, cond);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sltu { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let cond = builder.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs);
            let val = select_bool(builder, cond);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Slti { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = builder.ins().iconst(types::I32, *imm as i64);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
            let val = select_bool(builder, cond);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sltiu { rd, rs1, imm } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = builder.ins().iconst(types::I32, *imm as i64);
            let cond = builder.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs);
            let val = select_bool(builder, cond);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sll { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let mask = builder.ins().iconst(types::I32, 31);
            let shamt = builder.ins().band(rhs, mask);
            let val = builder.ins().ishl(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Srl { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let mask = builder.ins().iconst(types::I32, 31);
            let shamt = builder.ins().band(rhs, mask);
            let val = builder.ins().ushr(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sra { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let mask = builder.ins().iconst(types::I32, 31);
            let shamt = builder.ins().band(rhs, mask);
            let val = builder.ins().sshr(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Slli { rd, rs1, shamt } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let shamt = builder.ins().iconst(types::I32, *shamt as i64);
            let val = builder.ins().ishl(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Srli { rd, rs1, shamt } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let shamt = builder.ins().iconst(types::I32, *shamt as i64);
            let val = builder.ins().ushr(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Srai { rd, rs1, shamt } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let shamt = builder.ins().iconst(types::I32, *shamt as i64);
            let val = builder.ins().sshr(lhs, shamt);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lui { rd, imm } => {
            let val = builder.ins().iconst(types::I32, (*imm << 12) as i64);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Auipc { rd, imm } => {
            let pc_val = builder.ins().iconst(types::I32, inst.pc as i64);
            let imm_val = builder.ins().iconst(types::I32, (*imm << 12) as i64);
            let val = builder.ins().iadd(pc_val, imm_val);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lw { rd, rs1, offset } | Instruction::Ld { rd, rs1, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            let val = emit_load(
                module,
                helpers,
                builder,
                helpers.load_u32,
                cpu_ptr,
                mem_ptr,
                addr,
                halt_block,
            );
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lb { rd, rs1, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            let val = emit_load(
                module,
                helpers,
                builder,
                helpers.load_u8_signed,
                cpu_ptr,
                mem_ptr,
                addr,
                halt_block,
            );
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lbu { rd, rs1, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            let val = emit_load(
                module,
                helpers,
                builder,
                helpers.load_u8_unsigned,
                cpu_ptr,
                mem_ptr,
                addr,
                halt_block,
            );
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lh { rd, rs1, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            let val = emit_load(
                module,
                helpers,
                builder,
                helpers.load_u16_signed,
                cpu_ptr,
                mem_ptr,
                addr,
                halt_block,
            );
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Lhu { rd, rs1, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            let val = emit_load(
                module,
                helpers,
                builder,
                helpers.load_u16_unsigned,
                cpu_ptr,
                mem_ptr,
                addr,
                halt_block,
            );
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sb { rs1, rs2, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let val = emit_read_reg(builder, cpu_ptr, *rs2);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            emit_store(
                module,
                helpers,
                builder,
                helpers.store_u8,
                cpu_ptr,
                mem_ptr,
                addr,
                val,
                halt_block,
            );
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sh { rs1, rs2, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let val = emit_read_reg(builder, cpu_ptr, *rs2);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            emit_store(
                module,
                helpers,
                builder,
                helpers.store_u16,
                cpu_ptr,
                mem_ptr,
                addr,
                val,
                halt_block,
            );
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Sw { rs1, rs2, offset } => {
            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let val = emit_read_reg(builder, cpu_ptr, *rs2);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let addr = builder.ins().iadd(base, off);
            emit_store(
                module,
                helpers,
                builder,
                helpers.store_u32,
                cpu_ptr,
                mem_ptr,
                addr,
                val,
                halt_block,
            );
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Beq { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::Equal,
            );
        }
        Instruction::Bne { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::NotEqual,
            );
        }
        Instruction::Blt { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::SignedLessThan,
            );
        }
        Instruction::Bge { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::SignedGreaterThanOrEqual,
            );
        }
        Instruction::Bltu { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::UnsignedLessThan,
            );
        }
        Instruction::Bgeu { rs1, rs2, offset } => {
            return emit_branch(
                builder,
                cpu_ptr,
                inst,
                *rs1,
                *rs2,
                *offset,
                IntCC::UnsignedGreaterThanOrEqual,
            );
        }
        Instruction::Jal { rd, offset, .. } => {
            let return_addr = inst.pc.wrapping_add(inst.size as u32);
            let return_val = builder.ins().iconst(types::I32, return_addr as i64);
            emit_write_reg(builder, cpu_ptr, *rd, return_val);
            let target = inst.pc.wrapping_add(*offset as u32);
            let target_val = builder.ins().iconst(types::I32, target as i64);
            emit_set_pc(builder, cpu_ptr, target_val);
            return true;
        }
        Instruction::Jalr {
            rd,
            rs1,
            offset,
            compressed,
        } => {
            let return_addr = if *compressed {
                inst.pc.wrapping_add(2)
            } else {
                inst.pc.wrapping_add(4)
            };
            let return_val = builder.ins().iconst(types::I32, return_addr as i64);
            emit_write_reg(builder, cpu_ptr, *rd, return_val);

            let base = emit_read_reg(builder, cpu_ptr, *rs1);
            let off = builder.ins().iconst(types::I32, *offset as i64);
            let mut target = builder.ins().iadd(base, off);
            let mask = builder.ins().iconst(types::I32, !1i32 as i64);
            target = builder.ins().band(target, mask);
            emit_set_pc(builder, cpu_ptr, target);
            return true;
        }
        Instruction::Mul { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let val = builder.ins().imul(lhs, rhs);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Mulh { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let lhs64 = builder.ins().sextend(types::I64, lhs);
            let rhs64 = builder.ins().sextend(types::I64, rhs);
            let prod = builder.ins().imul(lhs64, rhs64);
            let hi = builder.ins().sshr_imm(prod, 32);
            let val = builder.ins().ireduce(types::I32, hi);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Mulhu { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let lhs64 = builder.ins().uextend(types::I64, lhs);
            let rhs64 = builder.ins().uextend(types::I64, rhs);
            let prod = builder.ins().imul(lhs64, rhs64);
            let hi = builder.ins().ushr_imm(prod, 32);
            let val = builder.ins().ireduce(types::I32, hi);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Mulhsu { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let lhs64 = builder.ins().sextend(types::I64, lhs);
            let rhs64 = builder.ins().uextend(types::I64, rhs);
            let prod = builder.ins().imul(lhs64, rhs64);
            let hi = builder.ins().sshr_imm(prod, 32);
            let val = builder.ins().ireduce(types::I32, hi);
            emit_write_reg(builder, cpu_ptr, *rd, val);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Div { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let result = emit_div(builder, lhs, rhs, true);
            emit_write_reg(builder, cpu_ptr, *rd, result);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Divu { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let result = emit_div(builder, lhs, rhs, false);
            emit_write_reg(builder, cpu_ptr, *rd, result);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Rem { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let result = emit_rem(builder, lhs, rhs, true);
            emit_write_reg(builder, cpu_ptr, *rd, result);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Remu { rd, rs1, rs2 } => {
            let lhs = emit_read_reg(builder, cpu_ptr, *rs1);
            let rhs = emit_read_reg(builder, cpu_ptr, *rs2);
            let result = emit_rem(builder, lhs, rhs, false);
            emit_write_reg(builder, cpu_ptr, *rd, result);
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        Instruction::Fence | Instruction::Unimp => {
            emit_pc_add(builder, cpu_ptr, inst.size as u32);
        }
        _ => {
            return false;
        }
    }
    true
}

fn emit_read_reg(
    builder: &mut FunctionBuilder,
    cpu_ptr: cranelift_codegen::ir::Value,
    reg: usize,
) -> cranelift_codegen::ir::Value {
    let offset = CPU_REGS_OFFSET + (reg as i32 * 4);
    builder
        .ins()
        .load(types::I32, MemFlags::new(), cpu_ptr, offset)
}

fn emit_write_reg(
    builder: &mut FunctionBuilder,
    cpu_ptr: cranelift_codegen::ir::Value,
    reg: usize,
    value: cranelift_codegen::ir::Value,
) {
    if reg == 0 {
        return;
    }
    let offset = CPU_REGS_OFFSET + (reg as i32 * 4);
    builder
        .ins()
        .store(MemFlags::new(), value, cpu_ptr, offset);
}

fn emit_pc_add(builder: &mut FunctionBuilder, cpu_ptr: cranelift_codegen::ir::Value, delta: u32) {
    let pc_val = builder
        .ins()
        .load(types::I32, MemFlags::new(), cpu_ptr, CPU_PC_OFFSET);
    let delta_val = builder.ins().iconst(types::I32, delta as i64);
    let next = builder.ins().iadd(pc_val, delta_val);
    builder
        .ins()
        .store(MemFlags::new(), next, cpu_ptr, CPU_PC_OFFSET);
}

fn emit_set_pc(
    builder: &mut FunctionBuilder,
    cpu_ptr: cranelift_codegen::ir::Value,
    target: cranelift_codegen::ir::Value,
) {
    builder
        .ins()
        .store(MemFlags::new(), target, cpu_ptr, CPU_PC_OFFSET);
}

fn emit_load(
    module: &mut JITModule,
    helpers: &HelperIds,
    builder: &mut FunctionBuilder,
    helper: FuncId,
    cpu_ptr: cranelift_codegen::ir::Value,
    mem_ptr: cranelift_codegen::ir::Value,
    addr: cranelift_codegen::ir::Value,
    halt_block: cranelift_codegen::ir::Block,
) -> cranelift_codegen::ir::Value {
    let callee = declare_helper(module, builder, helper);
    let call = builder.ins().call(callee, &[cpu_ptr, mem_ptr, addr]);
    let packed = builder.inst_results(call)[0];
    let high64 = builder.ins().ushr_imm(packed, 32);
    let high = builder.ins().ireduce(types::I32, high64);
    let low = builder.ins().ireduce(types::I32, packed);
    branch_if_zero(builder, high, halt_block);
    low
}

fn emit_store(
    module: &mut JITModule,
    helpers: &HelperIds,
    builder: &mut FunctionBuilder,
    helper: FuncId,
    cpu_ptr: cranelift_codegen::ir::Value,
    mem_ptr: cranelift_codegen::ir::Value,
    addr: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    halt_block: cranelift_codegen::ir::Block,
) {
    let callee = declare_helper(module, builder, helper);
    let call = builder.ins().call(callee, &[cpu_ptr, mem_ptr, addr, value]);
    let ok = builder.inst_results(call)[0];
    branch_if_zero(builder, ok, halt_block);
}

fn emit_branch(
    builder: &mut FunctionBuilder,
    cpu_ptr: cranelift_codegen::ir::Value,
    inst: &TraceInst,
    rs1: usize,
    rs2: usize,
    offset: i32,
    cond: IntCC,
) -> bool {
    let lhs = emit_read_reg(builder, cpu_ptr, rs1);
    let rhs = emit_read_reg(builder, cpu_ptr, rs2);
    let cmp = builder.ins().icmp(cond, lhs, rhs);
    let taken_block = builder.create_block();
    let not_taken_block = builder.create_block();

    builder
        .ins()
        .brif(cmp, taken_block, &[], not_taken_block, &[]);

    builder.switch_to_block(taken_block);
    builder.seal_block(taken_block);
    let target = inst.pc.wrapping_add(offset as u32);
    let target_val = builder.ins().iconst(types::I32, target as i64);
    emit_set_pc(builder, cpu_ptr, target_val);
    let one = builder.ins().iconst(types::I32, 1);
    builder.ins().return_(&[one]);

    builder.switch_to_block(not_taken_block);
    builder.seal_block(not_taken_block);
    emit_pc_add(builder, cpu_ptr, inst.size as u32);
    let one = builder.ins().iconst(types::I32, 1);
    builder.ins().return_(&[one]);

    true
}

fn emit_div(
    builder: &mut FunctionBuilder,
    lhs: cranelift_codegen::ir::Value,
    rhs: cranelift_codegen::ir::Value,
    signed: bool,
) -> cranelift_codegen::ir::Value {
    let zero = builder.ins().iconst(types::I32, 0);
    let minus_one = builder.ins().iconst(types::I32, -1);
    let min_val = builder.ins().iconst(types::I32, i32::MIN as i64);

    let zero_block = builder.create_block();
    let calc_block = builder.create_block();
    let cont_block = builder.create_block();
    let phi = builder.append_block_param(cont_block, types::I32);

    let is_zero = builder.ins().icmp(IntCC::Equal, rhs, zero);
    builder
        .ins()
        .brif(is_zero, zero_block, &[], calc_block, &[]);

    builder.switch_to_block(zero_block);
    builder.seal_block(zero_block);
    builder.ins().jump(cont_block, &[minus_one]);

    builder.switch_to_block(calc_block);
    builder.seal_block(calc_block);

    if signed {
        let is_overflow_lhs = builder.ins().icmp(IntCC::Equal, lhs, min_val);
        let is_overflow_rhs = builder.ins().icmp(IntCC::Equal, rhs, minus_one);
        let is_overflow = builder.ins().band(is_overflow_lhs, is_overflow_rhs);
        let overflow_block = builder.create_block();
        let div_block = builder.create_block();

        builder
            .ins()
            .brif(is_overflow, overflow_block, &[], div_block, &[]);

        builder.switch_to_block(overflow_block);
        builder.seal_block(overflow_block);
        builder.ins().jump(cont_block, &[lhs]);

        builder.switch_to_block(div_block);
        builder.seal_block(div_block);
        let div_val = builder.ins().sdiv(lhs, rhs);
        builder.ins().jump(cont_block, &[div_val]);
    } else {
        let div_val = builder.ins().udiv(lhs, rhs);
        builder.ins().jump(cont_block, &[div_val]);
    }

    builder.switch_to_block(cont_block);
    builder.seal_block(cont_block);
    phi
}

fn emit_rem(
    builder: &mut FunctionBuilder,
    lhs: cranelift_codegen::ir::Value,
    rhs: cranelift_codegen::ir::Value,
    signed: bool,
) -> cranelift_codegen::ir::Value {
    let zero = builder.ins().iconst(types::I32, 0);
    let minus_one = builder.ins().iconst(types::I32, -1);
    let min_val = builder.ins().iconst(types::I32, i32::MIN as i64);

    let zero_block = builder.create_block();
    let calc_block = builder.create_block();
    let cont_block = builder.create_block();
    let phi = builder.append_block_param(cont_block, types::I32);

    let is_zero = builder.ins().icmp(IntCC::Equal, rhs, zero);
    builder
        .ins()
        .brif(is_zero, zero_block, &[], calc_block, &[]);

    builder.switch_to_block(zero_block);
    builder.seal_block(zero_block);
    builder.ins().jump(cont_block, &[lhs]);

    builder.switch_to_block(calc_block);
    builder.seal_block(calc_block);

    if signed {
        let is_overflow_lhs = builder.ins().icmp(IntCC::Equal, lhs, min_val);
        let is_overflow_rhs = builder.ins().icmp(IntCC::Equal, rhs, minus_one);
        let is_overflow = builder.ins().band(is_overflow_lhs, is_overflow_rhs);
        let overflow_block = builder.create_block();
        let rem_block = builder.create_block();

        builder
            .ins()
            .brif(is_overflow, overflow_block, &[], rem_block, &[]);

        builder.switch_to_block(overflow_block);
        builder.seal_block(overflow_block);
        builder.ins().jump(cont_block, &[zero]);

        builder.switch_to_block(rem_block);
        builder.seal_block(rem_block);
        let rem_val = builder.ins().srem(lhs, rhs);
        builder.ins().jump(cont_block, &[rem_val]);
    } else {
        let rem_val = builder.ins().urem(lhs, rhs);
        builder.ins().jump(cont_block, &[rem_val]);
    }

    builder.switch_to_block(cont_block);
    builder.seal_block(cont_block);
    phi
}

fn declare_helper(
    module: &mut JITModule,
    builder: &mut FunctionBuilder,
    func_id: FuncId,
) -> cranelift_codegen::ir::FuncRef {
    module.declare_func_in_func(func_id, builder.func)
}

fn select_bool(builder: &mut FunctionBuilder, cond: cranelift_codegen::ir::Value) -> cranelift_codegen::ir::Value {
    let one = builder.ins().iconst(types::I32, 1);
    let zero = builder.ins().iconst(types::I32, 0);
    builder.ins().select(cond, one, zero)
}

fn branch_if_zero(
    builder: &mut FunctionBuilder,
    value: cranelift_codegen::ir::Value,
    halt_block: cranelift_codegen::ir::Block,
) {
    let zero = builder.ins().iconst(types::I32, 0);
    let is_zero = builder.ins().icmp(IntCC::Equal, value, zero);
    let cont_block = builder.create_block();
    builder
        .ins()
        .brif(is_zero, halt_block, &[], cont_block, &[]);
    builder.switch_to_block(cont_block);
    builder.seal_block(cont_block);
}

/// Declare helper signatures so the JIT can call into Rust runtime functions.
fn declare_helpers(module: &mut JITModule) -> HelperIds {
    let ptr_ty = module.target_config().pointer_type();
    let mut sig_load = module.make_signature();
    sig_load.params.push(AbiParam::new(ptr_ty));
    sig_load.params.push(AbiParam::new(ptr_ty));
    sig_load.params.push(AbiParam::new(types::I32));
    sig_load.returns.push(AbiParam::new(types::I64));

    let mut sig_store = module.make_signature();
    sig_store.params.push(AbiParam::new(ptr_ty));
    sig_store.params.push(AbiParam::new(ptr_ty));
    sig_store.params.push(AbiParam::new(types::I32));
    sig_store.params.push(AbiParam::new(types::I32));
    sig_store.returns.push(AbiParam::new(types::I32));

    HelperIds {
        load_u8_signed: module
            .declare_function("jit_load_u8_signed", Linkage::Import, &sig_load)
            .unwrap(),
        load_u8_unsigned: module
            .declare_function("jit_load_u8_unsigned", Linkage::Import, &sig_load)
            .unwrap(),
        load_u16_signed: module
            .declare_function("jit_load_u16_signed", Linkage::Import, &sig_load)
            .unwrap(),
        load_u16_unsigned: module
            .declare_function("jit_load_u16_unsigned", Linkage::Import, &sig_load)
            .unwrap(),
        load_u32: module
            .declare_function("jit_load_u32", Linkage::Import, &sig_load)
            .unwrap(),
        store_u8: module
            .declare_function("jit_store_u8", Linkage::Import, &sig_store)
            .unwrap(),
        store_u16: module
            .declare_function("jit_store_u16", Linkage::Import, &sig_store)
            .unwrap(),
        store_u32: module
            .declare_function("jit_store_u32", Linkage::Import, &sig_store)
            .unwrap(),
    }
}
