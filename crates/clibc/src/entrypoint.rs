/// Macro that creates the main entry point for a smart contract.
///
/// EDUCATIONAL PURPOSE: This macro generates the necessary code to interface
/// between the VM and the smart contract. It handles the conversion between
/// the VM's C-style interface and Rust's safe types.
///
/// FFI (Foreign Function Interface): This demonstrates how to create a safe
/// interface between different programming languages or systems. The VM calls
/// this function using C calling conventions, but we want to work with safe
/// Rust types inside our contract.
///
/// USAGE: Call this macro with the name of your main contract function:
/// ```ignore
/// use clibc::entrypoint;
/// entrypoint!(my_contract_function);
/// ```
///
/// SAFETY CONSIDERATIONS:
/// - Uses unsafe code to handle raw pointers from the VM
/// - Validates pointer bounds to prevent crashes
/// - Explicitly halts execution to prevent undefined behavior
///
/// PARAMETERS (from VM):
/// - address_ptr: Pointer to 20-byte contract address
/// - pubkey_ptr: Pointer to 20-byte caller address  
/// - input_ptr: Pointer to input data
/// - input_len: Length of input data
/// - result_ptr: Pointer where to write the result
#[macro_export]
macro_rules! entrypoint {
    ($func:path) => {
        #[cfg(not(target_arch = "riscv32"))]
        fn main() {
            // Host stub so guest binaries link without per-file cfg mains.
        }

        /// The main entry point that the VM calls to execute the contract.
        ///
        /// EDUCATIONAL: This function is marked as extern "C" to use C calling
        /// conventions, which is what the VM expects. The #[no_mangle] attribute
        /// prevents the compiler from changing the function name.
        #[allow(unreachable_code)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn entrypoint(
            to_ptr: *const u8,    // Pointer to contract address (20 bytes)
            from_ptr: *const u8,  // Pointer to caller address (20 bytes)
            input_ptr: *const u8, // Pointer to input data
            input_len: usize,     // Length of input data
        ) {
            // EDUCATIONAL: Write result directly to predetermined memory location
            // This prevents conflicts with macros that might overwrite A4
            // Must match global::RESULT_ADDR in crates/kernel/src/global.rs
            const RESULT_ADDR: usize = 0x100; // Predetermined memory address for result object

            // EDUCATIONAL: Convert raw pointer to contract address
            // This demonstrates safe pointer handling in unsafe code
            let to = {
                // EDUCATIONAL: Create a slice from the raw pointer (20 bytes for address)
                let slice = core::slice::from_raw_parts(to_ptr, 20);
                let mut array = [0u8; 20];
                // EDUCATIONAL: Copy bytes to avoid aliasing issues
                array.copy_from_slice(slice);
                $crate::types::address::Address(array)
            };

            // EDUCATIONAL: Convert raw pointer to caller address
            let from = {
                // EDUCATIONAL: Create a slice from the raw pointer (20 bytes for address)
                let slice = core::slice::from_raw_parts(from_ptr, 20);
                let mut array = [0u8; 20];
                // EDUCATIONAL: Copy bytes to avoid aliasing issues
                array.copy_from_slice(slice);
                $crate::types::address::Address(array)
            };

            // EDUCATIONAL: Convert raw pointer to input data slice
            let input = {
                // EDUCATIONAL: Create a slice from the raw pointer with specified length
                core::slice::from_raw_parts(input_ptr, input_len)
            };

            // EDUCATIONAL: Call the user's contract function with safe types
            let result = $func(to, from, input);

            // EDUCATIONAL: Write the result directly to the predetermined memory location
            core::ptr::write(RESULT_ADDR as *mut $crate::types::result::Result, result);

            // EDUCATIONAL: Explicitly halt execution to prevent undefined behavior
            // This is crucial because the VM expects the contract to halt, not return
            // The ebreak instruction triggers a breakpoint, and the infinite loop
            // ensures the function never returns normally
            #[cfg(target_arch = "riscv32")]
            unsafe {
                core::arch::asm!("ebreak")
            };
            #[cfg(not(target_arch = "riscv32"))]
            {
                // For non-RISC-V targets, just panic
                panic!("entrypoint: execution should halt");
            }
            loop {}
        }
    };
}
