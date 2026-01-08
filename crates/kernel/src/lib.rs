#![no_std]
#![feature(alloc_error_handler)]

pub use types::boot::BootInfo;

pub mod global;
pub mod task;
pub use task::{AddressSpace, Task, TrapFrame};
pub use task::{
    PROGRAM_VA_BASE, PROGRAM_WINDOW_BYTES, kernel_run_task, prep_program_task, run_task,
};
pub mod memory;
pub mod syscall;
pub mod trap;
pub mod user_program;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let mut buf = [0u8; 256];
    let len = {
        let mut writer = clibc::BufferWriter::new(&mut buf);
        if write!(&mut writer, "{}", info).is_ok() {
            writer.len()
        } else {
            0
        }
    };
    if len == 0 {
        clibc::log!("kernel panic");
    } else {
        clibc::logf!("kernel panic: %s", buf.as_ptr() as u32, len as u32);
    }
    unsafe { core::arch::asm!("ebreak") };
    loop {}
}
