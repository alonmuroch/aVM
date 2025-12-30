pub struct Config;

impl Config {
    pub const MAX_INPUT_LEN: usize = 1024;
    pub const CODE_SIZE_LIMIT: usize = 0x30000; // 192KB headroom for non-compressed RV32IM binaries
    pub const RO_DATA_SIZE_LIMIT: usize = 0x2000; // 8KB for read-only data
    pub const HEAP_START_ADDR: usize = Self::CODE_SIZE_LIMIT + Self::RO_DATA_SIZE_LIMIT + 0x100;
    pub const MAX_RESULT_SIZE: usize = types::result::RESULT_SIZE;

    pub const PROGRAM_START_ADDR: u32 = 0x400;
    pub const RESULT_ADDR: u32 = 0x100;
    /// Kernel handoff header address for serialized receipts (kernel VA).
    pub const KERNEL_RESULT_ADDR: u32 = 0x100;
}
