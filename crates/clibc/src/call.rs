use types::address::Address;
use types::result::Result;

pub fn call(from: &Address, to: &Address, input_data: &[u8]) -> Option<Result> {
    unsafe {
        let mut result_ptr: u32;
        core::arch::asm!(
            "li a7, 5",        // syscall ID for call_contract
            "ecall",
            in("x11") to.0.as_ptr(), // a1
            in("x12") from.0.as_ptr(), // a2
            in("x13") input_data.as_ptr(), // a3
            in("x14") input_data.len(), // a4
            out("x10") result_ptr, // a0
        );

        if result_ptr == 0 {
            return None;
        }

        Result::from_ptr(result_ptr)
    }
}
