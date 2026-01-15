use crate::cpu::CPU;
use crate::jit::JitAccess;
use crate::memory::{Memory, VirtualAddress};
use crate::metering::MemoryAccessKind;
use cranelift_jit::JITBuilder;

/// Register all helper symbols with the JIT so generated code can call into Rust.
pub fn register_helper_symbols(builder: &mut JITBuilder) {
    builder.symbol("jit_read_reg", jit_read_reg as *const u8);
    builder.symbol("jit_write_reg", jit_write_reg as *const u8);
    builder.symbol("jit_pc_add", jit_pc_add as *const u8);
    builder.symbol("jit_set_pc", jit_set_pc as *const u8);
    builder.symbol("jit_load_u8_signed", jit_load_u8_signed as *const u8);
    builder.symbol("jit_load_u8_unsigned", jit_load_u8_unsigned as *const u8);
    builder.symbol("jit_load_u16_signed", jit_load_u16_signed as *const u8);
    builder.symbol("jit_load_u16_unsigned", jit_load_u16_unsigned as *const u8);
    builder.symbol("jit_load_u32", jit_load_u32 as *const u8);
    builder.symbol("jit_store_u8", jit_store_u8 as *const u8);
    builder.symbol("jit_store_u16", jit_store_u16 as *const u8);
    builder.symbol("jit_store_u32", jit_store_u32 as *const u8);
}

/// Pack a value with a success bit in the upper 32 bits.
fn pack_ok(value: u32) -> u64 {
    ((1u64) << 32) | value as u64
}

/// Pack a failure indicator (upper 32 bits are 0).
fn pack_err() -> u64 {
    0
}

/// Read a register with metering. Returns packed (value, ok) for JIT use.
pub(crate) unsafe extern "C" fn jit_read_reg(cpu: *mut CPU, reg: u32) -> u64 {
    let cpu = &mut *cpu;
    match cpu.jit_read_reg(reg) {
        Some(val) => pack_ok(val),
        None => pack_err(),
    }
}

/// Write a register with metering. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_write_reg(cpu: *mut CPU, reg: u32, value: u32) -> u32 {
    let cpu = &mut *cpu;
    if cpu.jit_write_reg(reg, value) {
        1
    } else {
        0
    }
}

/// Add to PC with metering. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_pc_add(cpu: *mut CPU, delta: u32) -> u32 {
    let cpu = &mut *cpu;
    if cpu.jit_pc_add(delta) {
        1
    } else {
        0
    }
}

/// Set PC with metering. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_set_pc(cpu: *mut CPU, target: u32) -> u32 {
    let cpu = &mut *cpu;
    if cpu.jit_set_pc(target) {
        1
    } else {
        0
    }
}

/// Load a byte and sign-extend to 32-bit. Returns packed (value, ok).
pub(crate) unsafe extern "C" fn jit_load_u8_signed(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
) -> u64 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    let byte = match memory.load_byte(addr, cpu.metering.as_mut(), MemoryAccessKind::Load) {
        Some(val) => val,
        None => return pack_err(),
    };
    let value = (byte as i8) as i32 as u32;
    pack_ok(value)
}

/// Load a byte and zero-extend to 32-bit. Returns packed (value, ok).
pub(crate) unsafe extern "C" fn jit_load_u8_unsigned(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
) -> u64 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    let byte = match memory.load_byte(addr, cpu.metering.as_mut(), MemoryAccessKind::Load) {
        Some(val) => val,
        None => return pack_err(),
    };
    pack_ok(byte as u32)
}

/// Load a halfword and sign-extend to 32-bit. Returns packed (value, ok).
pub(crate) unsafe extern "C" fn jit_load_u16_signed(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
) -> u64 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    let halfword = match memory.load_halfword(addr, cpu.metering.as_mut(), MemoryAccessKind::Load) {
        Some(val) => val,
        None => return pack_err(),
    };
    let value = (halfword as i16) as i32 as u32;
    pack_ok(value)
}

/// Load a halfword and zero-extend to 32-bit. Returns packed (value, ok).
pub(crate) unsafe extern "C" fn jit_load_u16_unsigned(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
) -> u64 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    let halfword = match memory.load_halfword(addr, cpu.metering.as_mut(), MemoryAccessKind::Load) {
        Some(val) => val,
        None => return pack_err(),
    };
    pack_ok(halfword as u32)
}

/// Load a word. Returns packed (value, ok).
pub(crate) unsafe extern "C" fn jit_load_u32(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
) -> u64 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    let val = match memory.load_u32(addr, cpu.metering.as_mut(), MemoryAccessKind::Load) {
        Some(v) => v,
        None => return pack_err(),
    };
    pack_ok(val)
}

/// Store a byte. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_store_u8(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
    value: u32,
) -> u32 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    if memory.store_u8(
        addr,
        value as u8,
        cpu.metering.as_mut(),
        MemoryAccessKind::Store,
    ) {
        1
    } else {
        0
    }
}

/// Store a halfword. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_store_u16(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
    value: u32,
) -> u32 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    if memory.store_u16(
        addr,
        value as u16,
        cpu.metering.as_mut(),
        MemoryAccessKind::Store,
    ) {
        1
    } else {
        0
    }
}

/// Store a word. Returns 1 on success.
pub(crate) unsafe extern "C" fn jit_store_u32(
    cpu: *mut CPU,
    memory: *const Memory,
    addr: u32,
    value: u32,
) -> u32 {
    let cpu = &mut *cpu;
    let memory = &*memory;
    let addr = VirtualAddress(addr);
    if memory.store_u32(addr, value, cpu.metering.as_mut(), MemoryAccessKind::Store) {
        1
    } else {
        0
    }
}
