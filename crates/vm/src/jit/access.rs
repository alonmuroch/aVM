use crate::cpu::CPU;

/// Narrow adapter trait that exposes CPU internals to JIT helpers.
/// Keeping this in the JIT module avoids making CPU methods public.
pub(crate) trait JitAccess {
    fn jit_read_reg(&mut self, reg: u32) -> Option<u32>;
    fn jit_write_reg(&mut self, reg: u32, value: u32) -> bool;
    fn jit_pc_add(&mut self, delta: u32) -> bool;
    fn jit_set_pc(&mut self, target: u32) -> bool;
}

impl JitAccess for CPU {
    fn jit_read_reg(&mut self, reg: u32) -> Option<u32> {
        self.read_reg(reg as usize)
    }

    fn jit_write_reg(&mut self, reg: u32, value: u32) -> bool {
        self.write_reg(reg as usize, value)
    }

    fn jit_pc_add(&mut self, delta: u32) -> bool {
        self.pc_add(delta)
    }

    fn jit_set_pc(&mut self, target: u32) -> bool {
        self.set_pc(target)
    }
}
