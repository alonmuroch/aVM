use std::cell::{Cell, RefCell};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::mem;
use std::rc::Rc;

use compiler::elf::parse_elf_from_bytes;
use goblin::elf::Elf;
use types::SV32_DIRECT_MAP_BASE;
use types::boot::BootInfo;
use types::kernel_result::KERNEL_RESULT_ADDR;
use vm::memory::{API, HEAP_PTR_OFFSET, MMU, PAGE_SIZE, Perms, Sv32Memory, VirtualAddress};
use vm::metering::{MeterResult, Metering};
use vm::registers::Register;
use vm::vm::VM;
use vm::instruction::Instruction;

use crate::arch::{ArchRunner, RunError, RunResult};
use crate::types::{ElfTarget, RunOptions};

pub struct AvmRunner;

impl AvmRunner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AvmRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct InstructionCounter {
    count: Rc<Cell<u64>>,
}

impl Metering for InstructionCounter {
    fn on_instruction(&mut self, _pc: u32, _instr: &Instruction, _size: u8) -> MeterResult {
        self.count.set(self.count.get().saturating_add(1));
        MeterResult::Continue
    }
}

impl ArchRunner for AvmRunner {
    fn name(&self) -> &str {
        "avm"
    }

    fn run(&self, elf: &ElfTarget, options: &RunOptions) -> Result<RunResult, RunError> {
        let elf_bytes = fs::read(&elf.path).map_err(|e| RunError {
            message: format!("failed to read elf {}: {e}", elf.path.display()),
        })?;

        let total_size = options.vm_memory_size.unwrap_or(16 * 1024 * 1024);
        let memory = Rc::new(Sv32Memory::new(total_size, PAGE_SIZE));
        let heap_ptr = Rc::new(Cell::new(0u32));
        let entry_point = load_kernel(&elf_bytes, &memory, heap_ptr.as_ref())?;

        if options.input.len() > 3usize {
            return Err(RunError {
                message: format!("too many inputs ({}); max is 3", options.input.len()),
            });
        }
        let mut input_ptrs = [0u32; 3];
        let mut input_lens = [0u32; 3];
        for idx in 0..options.input.len() {
            let bytes = options
                .input
                .get(idx)
                .map(|input| input.as_slice())
                .unwrap_or(&[]);
            let ptr = alloc_on_heap(memory.as_ref(), heap_ptr.as_ref(), bytes);
            input_ptrs[idx] = ptr;
            input_lens[idx] = bytes.len() as u32;
        }
        let boot_info_ptr = place_boot_info(memory.as_ref(), heap_ptr.as_ref(), total_size)?;

        let mut vm = VM::new(memory.clone());
        vm.set_reg_u32(Register::Sp, KERNEL_STACK_TOP);
        vm.cpu.verbose = options.verbose;
        let instruction_count = Rc::new(Cell::new(0u64));
        vm.set_metering(Box::new(InstructionCounter {
            count: Rc::clone(&instruction_count),
        }));

        let writer = Rc::new(RefCell::new(StringWriter::default()));
        vm.cpu.set_verbose_writer(writer.clone());
        vm.cpu.pc = entry_point;

        // set input regs
        const ARG_REGS: [Register; 8] = [
            Register::A0,
            Register::A1,
            Register::A2,
            Register::A3,
            Register::A4,
            Register::A5,
            Register::A6,
            Register::A7,
        ];
        for (idx, ptr) in input_ptrs.iter().enumerate() {
            let reg_idx = idx * 2;
            vm.set_reg_u32(ARG_REGS[reg_idx], *ptr);
            vm.set_reg_u32(ARG_REGS[reg_idx + 1], input_lens[idx]);
        }
        let boot_reg_idx = options.input.len() * 2;
        if boot_reg_idx >= ARG_REGS.len() {
            return Err(RunError {
                message: "no argument register available for boot info".to_string(),
            });
        }
        vm.set_reg_u32(ARG_REGS[boot_reg_idx], boot_info_ptr);
        if boot_reg_idx + 1 < ARG_REGS.len() {
            vm.set_reg_u32(ARG_REGS[boot_reg_idx + 1], 0);
        }

        vm.raw_run();

        let stdout = writer.borrow().buffer.clone();
        let output = read_kernel_blob(memory.as_ref()).unwrap_or_default();
        let exit_code = 0;
        let stderr = String::new();
        let instruction_count = instruction_count.get();

        Ok(RunResult {
            exit_code,
            stdout,
            stderr,
            output,
            instruction_count,
        })
    }
}

const KERNEL_WINDOW_BYTES: usize = 4 * 1024 * 1024;
const KERNEL_STACK_TOP: u32 = KERNEL_WINDOW_BYTES as u32;
const KERNEL_RESULT_DUMP_BYTES: u32 = 1024 * 1024;

fn load_kernel(
    elf_bytes: &[u8],
    memory: &Rc<Sv32Memory>,
    heap_ptr: &Cell<u32>,
) -> Result<u32, RunError> {
    let elf = parse_elf_from_bytes(elf_bytes).map_err(|e| RunError {
        message: format!("failed to parse kernel elf: {e}"),
    })?;
    let entry_point = Elf::parse(elf_bytes)
        .map_err(|e| RunError {
            message: format!("failed to parse entry point: {e}"),
        })?
        .entry as u32;

    let (code, code_base) = elf.get_flat_code().ok_or_else(|| RunError {
        message: "kernel elf missing .text".to_string(),
    })?;
    let (rodata, ro_base) = elf.get_flat_rodata().unwrap_or((Vec::new(), code_base));
    let (bss, bss_base) = elf.get_flat_bss().unwrap_or((Vec::new(), code_base));

    let mut min_base = core::cmp::min(code_base, ro_base) as usize;
    if !bss.is_empty() {
        min_base = core::cmp::min(min_base, bss_base as usize);
    }
    let code_end = (code_base + code.len() as u64) as usize;
    let ro_end = (ro_base + rodata.len() as u64) as usize;
    let mut image_end = core::cmp::max(code_end, ro_end);
    if !bss.is_empty() {
        let bss_end = bss_base
            .checked_add(bss.len() as u64)
            .ok_or_else(|| RunError {
                message: "bss end overflow".to_string(),
            })? as usize;
        image_end = core::cmp::max(image_end, bss_end);
    }
    let image_size = image_end.checked_sub(min_base).ok_or_else(|| RunError {
        message: "invalid image size".to_string(),
    })?;

    if image_end > memory.size() {
        return Err(RunError {
            message: format!(
                "elf image does not fit in mapped memory (need {}, have {})",
                image_end,
                memory.size()
            ),
        });
    }
    if KERNEL_WINDOW_BYTES > memory.size() {
        return Err(RunError {
            message: format!(
                "kernel window exceeds physical memory (need {}, have {})",
                KERNEL_WINDOW_BYTES,
                memory.size()
            ),
        });
    }

    let mut image = vec![0u8; image_size];
    let code_off = (code_base as usize).saturating_sub(min_base);
    image[code_off..code_off + code.len()].copy_from_slice(&code);
    if !rodata.is_empty() {
        let ro_off = (ro_base as usize).saturating_sub(min_base);
        image[ro_off..ro_off + rodata.len()].copy_from_slice(&rodata);
    }
    if !bss.is_empty() {
        let bss_off = (bss_base as usize).saturating_sub(min_base);
        image[bss_off..bss_off + bss.len()].copy_from_slice(&bss);
    }

    memory.map_range(VirtualAddress(0), KERNEL_WINDOW_BYTES, Perms::rwx_kernel());
    memory.write_bytes(VirtualAddress(min_base as u32), &image);

    let heap_start = ((image_end + HEAP_PTR_OFFSET as usize + 7) & !7) as u32;
    heap_ptr.set(heap_start);

    let mapped = memory.map_physical_range(
        VirtualAddress(SV32_DIRECT_MAP_BASE),
        0,
        memory.size(),
        Perms::rw_kernel(),
    );
    if !mapped {
        return Err(RunError {
            message: "failed to map kernel direct physical window".to_string(),
        });
    }

    Ok(entry_point)
}

fn read_kernel_blob(memory: &Sv32Memory) -> Option<Vec<u8>> {
    let start = VirtualAddress(KERNEL_RESULT_ADDR);
    let end = start.checked_add(KERNEL_RESULT_DUMP_BYTES)?;
    let slice = memory.mem_slice(start, end)?;
    Some(slice.as_ref().to_vec())
}

fn place_boot_info(
    memory: &Sv32Memory,
    heap_ptr: &Cell<u32>,
    memory_size: usize,
) -> Result<u32, RunError> {
    let heap_start = ensure_heap_ptr(heap_ptr);
    let aligned_heap = (heap_start + 7) & !7;
    let boot_info_size = mem::size_of::<BootInfo>() as u32;
    let next_heap = aligned_heap
        .checked_add(boot_info_size)
        .and_then(|v| v.checked_add(HEAP_PTR_OFFSET))
        .ok_or_else(|| RunError {
            message: "boot info heap pointer overflow".to_string(),
        })?;
    let boot_info = BootInfo::new(
        memory.current_root() as u32,
        KERNEL_STACK_TOP,
        next_heap,
        memory_size as u32,
        memory.next_free_ppn() as u32,
        0,
        KERNEL_WINDOW_BYTES as u32,
    );
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &boot_info as *const BootInfo as *const u8,
            mem::size_of::<BootInfo>(),
        )
    };
    let addr = alloc_on_heap(memory, heap_ptr, bytes);
    Ok(addr)
}

fn alloc_on_heap(memory: &Sv32Memory, heap_ptr: &Cell<u32>, data: &[u8]) -> u32 {
    let addr = ensure_heap_ptr(heap_ptr);
    memory.write_bytes(VirtualAddress(addr), data);
    let next = (addr + data.len() as u32 + HEAP_PTR_OFFSET + 7) & !7;
    heap_ptr.set(next);
    addr
}

fn ensure_heap_ptr(heap_ptr: &Cell<u32>) -> u32 {
    let current = heap_ptr.get();
    if current == 0 {
        heap_ptr.set(HEAP_PTR_OFFSET);
        HEAP_PTR_OFFSET
    } else {
        current
    }
}

#[derive(Default)]
struct StringWriter {
    buffer: String,
}

impl FmtWrite for StringWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.buffer.push_str(s);
        Ok(())
    }
}
