use types::{O, address::Address};

/// Domain constant for persistent storage
pub const PERSISTENT_DOMAIN: &str = "P";

/// Trait for persistent structs
pub trait Persistent {
    fn load(address: &Address) -> O<Self>
    where
        Self: Sized;

    fn store(&self, address: &Address);
}

/// Macro that defines persistent structs with embedded static key
#[macro_export]
macro_rules! persist_struct {
    (
        $name:ident {
            $($field:ident : $type:ty),* $(,)?
        }
    ) => {
        #[repr(C)]
        #[derive(Copy, Clone, Debug)]
        pub struct $name {
            $(pub $field: $type),*
        }

        impl $name {
            const PERSIST_KEY: &'static [u8] = stringify!($name).as_bytes();

            fn key_ptr() -> *const u8 {
                Self::PERSIST_KEY.as_ptr()
            }

            fn key_len() -> usize {
                Self::PERSIST_KEY.len()
            }
            pub fn as_bytes(&self) -> &[u8] {
                let ptr = self as *const _ as *const u8;
                let len = core::mem::size_of::<Self>();
                unsafe { core::slice::from_raw_parts(ptr, len) }
            }

            pub fn from_bytes(bytes: &[u8]) -> $crate::types::O<Self> {
                if bytes.len() != core::mem::size_of::<Self>() {
                    return $crate::types::O::None;
                }

                let mut val = core::mem::MaybeUninit::<Self>::uninit();
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        val.as_mut_ptr() as *mut u8,
                        bytes.len(),
                    );
                    $crate::types::O::Some(val.assume_init())
                }
            }

            pub fn load(address: &$crate::types::address::Address) -> $crate::types::O<Self> {
                <$name as $crate::Persistent>::load(address)
            }

            pub fn store(&self, address: &$crate::types::address::Address) {
                <$name as $crate::Persistent>::store(self, address)
            }
        }

        impl $crate::Persistent for $name {
            fn load(address: &$crate::types::address::Address) -> $crate::types::O<Self> {
                #[cfg(target_arch = "riscv32")]
                unsafe {
                    let key_ptr = $name::key_ptr();
                    let key_len = $name::key_len();
                    let packed_lens: u32 = ((key_len as u32) << 16) | ($crate::PERSISTENT_DOMAIN.len() as u32);

                    if key_len == 0 {
                        $crate::vm_panic(
                            concat!("❌ persistent key for `", stringify!($name), "` is empty").as_bytes()
                        );
                    }

                    let mut value_ptr: u32;
                    core::arch::asm!(
                        "li a7, 1",  // syscall_storage_read
                        "ecall",
                        in("a1") address.as_ref().as_ptr(), // address ptr
                        in("a2") $crate::PERSISTENT_DOMAIN.as_ptr(), // domain ptr - use constant
                        in("a3") key_ptr, // key ptr
                        in("a4") packed_lens, // packed lens (domain | key)
                        out("a0") value_ptr,
                    );

                    if value_ptr == 0 {
                        return $crate::types::O::None;
                    }

                    let len_bytes = core::slice::from_raw_parts(value_ptr as *const u8, 4);
                    let value_len = u32::from_le_bytes([
                        len_bytes[0],
                        len_bytes[1],
                        len_bytes[2],
                        len_bytes[3],
                    ]) as usize;

                    if value_len == 0 {
                        $crate::require(value_len > 0, b"Decoded value len is 0 for bytes");
                        return $crate::types::O::None;
                    }

                    let data_ptr = (value_ptr + 4) as *const u8;
                    let value_buf = core::slice::from_raw_parts(data_ptr, value_len);

                    Self::from_bytes(value_buf)
                }

                #[cfg(not(target_arch = "riscv32"))]
                {
                    let _ = address;
                    // For non-RISC-V targets, return None
                    $crate::types::O::None
                }
            }

            fn store(&self, address: &$crate::types::address::Address) {
                #[cfg(target_arch = "riscv32")]
                unsafe {
                    let key_ptr = $name::key_ptr();
                    let key_len = $name::key_len();
                    let packed_lens: u32 = ((key_len as u32) << 16) | ($crate::PERSISTENT_DOMAIN.len() as u32);

                    if key_len == 0 {
                        $crate::vm_panic(
                            concat!("❌ persistent key for `", stringify!($name), "` is empty").as_bytes()
                        );
                    }

                    let val_buf = self.as_bytes();

                    let mut buf: [u8; core::mem::size_of::<Self>()] = core::mem::zeroed();
                    let len = buf.len();
                    core::ptr::copy_nonoverlapping(val_buf.as_ptr(), buf.as_mut_ptr(), len);

                    let val_ptr = buf.as_ptr();
                    let val_len = len;

                    core::arch::asm!(
                        "li a7, 2", // syscall_storage_write
                        "ecall",
                        in("a1") address.as_ref().as_ptr(), // address ptr
                        in("a2") $crate::PERSISTENT_DOMAIN.as_ptr(), // domain ptr - use constant
                        in("a3") key_ptr, // key ptr
                        in("a4") packed_lens, // packed lens (domain | key)
                        in("a5") val_ptr, // value ptr
                        in("a6") val_len, // value len
                        options(readonly, nostack, preserves_flags)
                    );
                }

                #[cfg(not(target_arch = "riscv32"))]
                {
                    let _ = address;
                    // For non-RISC-V targets, do nothing
                }
            }
        }
    };
}
