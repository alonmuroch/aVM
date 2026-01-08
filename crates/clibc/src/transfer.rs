use types::address::Address;

const SYSCALL_TRANSFER: u32 = 9;
const SYSCALL_BALANCE: u32 = 10;

/// Executes a native AM token transfer via syscall. Returns true on success.
#[inline(always)]
pub fn transfer(to: &Address, value: u64) -> bool {
    let mut ok: u32;
    unsafe {
        core::arch::asm!(
            "li a7, {transfer}",
            "ecall",
            in("a1") 0u32,
            in("a2") to.0.as_ptr(),
            in("a3") value as u32,
            in("a4") (value >> 32) as u32,
            lateout("a0") ok,
            transfer = const SYSCALL_TRANSFER,
        );
    }
    ok == 0
}

/// Returns the current balance of an address via syscall.
#[inline(always)]
pub fn balance(addr: &Address) -> u128 {
    let mut ptr: u32;
    unsafe {
        core::arch::asm!(
            "li a7, {balance}",
            "ecall",
            in("a1") addr.0.as_ptr(),
            lateout("a0") ptr,
            balance = const SYSCALL_BALANCE,
        );
    }
    if ptr == 0 {
        return 0;
    }
    let mut bytes = [0u8; 16];
    unsafe {
        let src = ptr as *const u8;
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = *src.add(i);
        }
    }
    u128::from_le_bytes(bytes)
}

/// Convenience macro to invoke a transfer from a contract.
#[macro_export]
macro_rules! transfer {
    ($to:expr, $value:expr) => {{ $crate::transfer::transfer($to, $value) }};
}

/// Macro wrapper for `balance`.
#[macro_export]
macro_rules! balance {
    ($addr:expr) => {{ $crate::transfer::balance($addr) }};
}
