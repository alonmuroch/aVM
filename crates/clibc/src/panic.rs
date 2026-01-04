//! Panic helper and handler for guest programs.

/// Trap into the host with a panic message.
#[inline(always)]
pub fn vm_panic(msg: &[u8]) -> ! {
    #[cfg(target_arch = "riscv32")]
    unsafe {
        core::arch::asm!(
            "li a7, 3", // SYSCALL_PANIC
            "ecall",
            in("a1") msg.as_ptr(),
            in("a2") msg.len(),
            options(noreturn),
        );
    }

    #[cfg(not(target_arch = "riscv32"))]
    {
        panic!(
            "vm_panic: {}",
            core::str::from_utf8(msg).unwrap_or("<non-utf8>")
        );
    }
}

/// Guest panic handler for RISC-V builds (only when guest feature enabled).
#[cfg(all(target_arch = "riscv32", feature = "guest"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let mut buf = [0u8; 256];
    let len = {
        let mut writer = crate::BufferWriter::new(&mut buf);
        if write!(&mut writer, "{}", info).is_ok() {
            writer.len()
        } else {
            0
        }
    };
    if len == 0 {
        vm_panic(b"guest panic");
    } else {
        vm_panic(&buf[..len]);
    }
}
