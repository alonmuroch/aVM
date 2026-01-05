use core::fmt::Write as FmtWrite;
use core::{mem, slice};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::vec::Vec;

use compiler::elf::parse_elf_from_bytes;
use goblin::elf::Elf;
use types::{SV32_DIRECT_MAP_BASE, boot::BootInfo, transaction::TransactionBundle};

use state::State;
use vm::memory::{
    API, HEAP_PTR_OFFSET, Memory as MmuRef, PAGE_SIZE, Perms, Sv32Memory, VirtualAddress,
};
use vm::registers::Register;
use vm::vm::VM;

const KERNEL_WINDOW_BYTES: usize = 4 * 1024 * 1024;
const KERNEL_STACK_TOP: u32 = KERNEL_WINDOW_BYTES as u32;

/// Boot configuration options consumed by the loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootConfig {
    pub debug_console: bool,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            debug_console: true,
        }
    }
}

/// Bootloader skeleton that loads a kernel image into fresh memory and
/// hands control to the kernel.
#[derive(Debug)]
pub struct Bootloader {
    pub config: BootConfig,
    memory: Rc<Sv32Memory>,
    heap_ptr: Rc<Cell<u32>>,
}

impl Bootloader {
    pub fn new(total_size_bytes: usize) -> Self {
        Self {
            config: BootConfig::default(),
            memory: Rc::new(Sv32Memory::new(total_size_bytes, PAGE_SIZE)),
            heap_ptr: Rc::new(Cell::new(0)),
        }
    }

    /// Load an ELF kernel image into a fresh page and return its entry point + backing memory.
    pub fn load_kernel(&mut self, elf_bytes: &[u8]) -> (u32, MmuRef) {
        let elf = parse_elf_from_bytes(elf_bytes).expect("failed to parse kernel ELF");
        let entry_point = Elf::parse(elf_bytes)
            .expect("failed to parse entry point")
            .entry as u32;

        let (code, code_base) = elf.get_flat_code().expect("kernel ELF missing .text");
        let (rodata, ro_base) = elf.get_flat_rodata().unwrap_or((Vec::new(), code_base));
        let (bss, bss_base) = elf.get_flat_bss().unwrap_or((Vec::new(), code_base));
        println!(
            "kernel elf: text_base=0x{:08x} text_len=0x{:x} ro_base=0x{:08x} ro_len=0x{:x} bss_base=0x{:08x} bss_len=0x{:x}",
            code_base as u32,
            code.len(),
            ro_base as u32,
            rodata.len(),
            bss_base as u32,
            bss.len()
        );

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
                .expect("bss end overflow") as usize;
            image_end = core::cmp::max(image_end, bss_end);
        }
        let image_size = image_end.checked_sub(min_base).expect("invalid image size");

        assert!(
            image_end <= self.memory.size(),
            "ELF image does not fit in mapped memory (need {}, have {})",
            image_end,
            self.memory.size()
        );
        assert!(
            KERNEL_WINDOW_BYTES <= self.memory.size(),
            "kernel window exceeds physical memory (need {}, have {})",
            KERNEL_WINDOW_BYTES,
            self.memory.size()
        );

        // Flatten code + rodata into a single buffer and write once to set heap pointer properly.
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

        self.memory
            .map_range(VirtualAddress(0), KERNEL_WINDOW_BYTES, Perms::rwx_kernel());
        self.memory
            .write_bytes(VirtualAddress(min_base as u32), &image);
        // Start the heap after the loaded image to avoid overwriting kernel text/rodata.
        let heap_start = ((image_end + HEAP_PTR_OFFSET as usize + 7) & !7) as u32;
        self.set_next_heap(heap_start);
        // Map a direct window over all physical memory so the kernel can touch
        // page tables after paging is enabled.
        let mapped = self.memory.map_physical_range(
            VirtualAddress(SV32_DIRECT_MAP_BASE),
            0,
            self.memory.size(),
            Perms::rw_kernel(),
        );
        assert!(mapped, "failed to map kernel direct physical window");
        let memory: MmuRef = self.memory.clone();
        (entry_point, memory)
    }

    /// Execute a transaction bundle by delegating to the kernel. This mirrors the
    /// AVM entry point where the kernel is responsible for invoking programs.
    pub fn execute_bundle(
        &mut self,
        kernel_elf: &[u8],
        bundle: &TransactionBundle,
        state: Rc<RefCell<State>>,
        verbose: bool,
        verbose_writer: Option<Rc<RefCell<dyn FmtWrite>>>,
    ) -> Option<crate::result::KernelRunResult> {
        let (entry_point, memory) = self.load_kernel(kernel_elf);
        let mut vm = VM::new(memory.clone());
        vm.set_reg_u32(Register::Sp, KERNEL_STACK_TOP);
        vm.cpu.verbose = verbose;
        if let Some(writer) = verbose_writer {
            vm.cpu.set_verbose_writer(writer);
        }
        vm.cpu.pc = entry_point;

        self.place_bundle(&mut vm, bundle);
        let encoded_state = state.borrow().encode();
        self.place_state(&mut vm, &encoded_state);
        self.place_boot_info(&mut vm);
        vm.raw_run();
        crate::result::read_kernel_result(&memory)
    }

    fn place_bundle(&mut self, vm: &mut VM, bundle: &TransactionBundle) {
        let encoded = bundle.encode();
        let addr = self.place_data(vm, Register::A0, &encoded);
        // Register a length hint so the kernel can bounds-check the payload.
        vm.set_reg_u32(Register::A1, encoded.len() as u32);
        // Keep heap aligned after our write.
        self.set_next_heap((addr as usize + encoded.len() + HEAP_PTR_OFFSET as usize) as u32);
    }

    fn place_state(&mut self, vm: &mut VM, state: &[u8]) {
        let addr = self.place_data(vm, Register::A2, state);
        vm.set_reg_u32(Register::A3, state.len() as u32);
        self.set_next_heap((addr as usize + state.len() + HEAP_PTR_OFFSET as usize) as u32);
    }

    fn place_boot_info(&mut self, vm: &mut VM) {
        // For now the bootloader owns the page tables, so `root_ppn` is a placeholder (0).
        let heap_start = self.ensure_heap_ptr();
        let aligned_heap = (heap_start + 7) & !7;
        let boot_info_size = mem::size_of::<BootInfo>() as u32;
        let next_heap = aligned_heap
            .checked_add(boot_info_size)
            .and_then(|v| v.checked_add(HEAP_PTR_OFFSET))
            .expect("boot info heap pointer overflow");
        let boot_info = BootInfo::new(
            self.memory.current_root() as u32,
            KERNEL_STACK_TOP,
            next_heap,
            self.memory.size() as u32,
            self.memory.next_free_ppn() as u32,
            0,
            KERNEL_WINDOW_BYTES as u32,
        );
        let bytes = unsafe {
            slice::from_raw_parts(
                &boot_info as *const BootInfo as *const u8,
                mem::size_of::<BootInfo>(),
            )
        };
        let _addr = self.place_data(vm, Register::A4, bytes);
        self.set_next_heap(next_heap);
    }

    fn place_data(&self, vm: &mut VM, reg: Register, data: &[u8]) -> u32 {
        let addr = self.alloc_on_heap(data).as_u32();
        vm.cpu.regs[reg as usize] = addr;
        addr
    }

    fn ensure_heap_ptr(&self) -> u32 {
        let current = self.heap_ptr.get();
        if current == 0 {
            self.heap_ptr.set(HEAP_PTR_OFFSET);
            HEAP_PTR_OFFSET
        } else {
            current
        }
    }

    fn set_next_heap(&self, next: u32) {
        self.heap_ptr.set(next);
    }

    fn alloc_on_heap(&self, data: &[u8]) -> VirtualAddress {
        let mut addr = self.ensure_heap_ptr();
        let align = 8u32;
        addr = (addr + (align - 1)) & !(align - 1);
        let end = addr
            .checked_add(data.len() as u32)
            .expect("heap allocation overflow");
        let start = VirtualAddress(addr);
        self.memory.map_range(start, data.len(), Perms::rw_kernel());
        self.memory.write_bytes(start, data);
        self.heap_ptr.set(end);
        start
    }
}
