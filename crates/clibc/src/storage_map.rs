use crate::{require, types::O, types::address::Address};
use core::mem::{MaybeUninit, size_of};

/// Trait for types that can be used as storage keys in `StorageMap`.
pub trait StorageKey {
    /// Fills the provided buffer with the hex-encoded key.
    /// Returns the length of the encoded key.
    fn as_storage_key(&self) -> &[u8];
}

impl StorageKey for Address {
    fn as_storage_key(&self) -> &[u8] {
        &self.0
    }
}

pub struct StorageMap;

impl StorageMap {
    pub fn get<V>(address: &Address, domain: &[u8], key: &[u8]) -> O<V>
    where
        V: Copy + Default,
    {
        require(key.len() <= 64, b"key too long");
        require(domain.len() <= 64, b"domain too long");

        let mut full_key = [0u8; 64];
        full_key[..key.len()].copy_from_slice(key);

        #[cfg(target_arch = "riscv32")]
        unsafe {
            let packed_lens: u32 = ((key.len() as u32) << 16) | (domain.len() as u32);
            let mut value_ptr: u32;
            core::arch::asm!(
                "li a7, 1", // syscall_storage_read
                "ecall",
                in("a1") address.as_ref().as_ptr(), // a1 - address ptr
                in("a2") domain.as_ptr(), // a2 - domain ptr
                in("a3") full_key.as_ptr(), // a3 - key ptr
                in("a4") packed_lens, // a4 - packed lens (domain | key)
                out("a0") value_ptr, // a0
            );

            if value_ptr == 0 {
                return O::None;
            }

            let len_bytes = core::slice::from_raw_parts(value_ptr as *const u8, 4);
            let value_len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

            if value_len != size_of::<V>() {
                return O::None;
            }

            let data_ptr = (value_ptr + 4) as *const u8;
            let buf = core::slice::from_raw_parts(data_ptr, value_len);

            let mut val = MaybeUninit::<V>::uninit();
            core::ptr::copy_nonoverlapping(buf.as_ptr(), val.as_mut_ptr() as *mut u8, value_len);
            O::Some(val.assume_init())
        }

        #[cfg(not(target_arch = "riscv32"))]
        {
            let _ = address;
            // For non-RISC-V targets, return None
            O::None
        }
    }

    pub fn set<V>(address: &Address, domain: &[u8], key: &[u8], val: V)
    where
        V: Copy,
    {
        require(key.len() <= 64, b"key too long");
        require(domain.len() <= 64, b"domain too long");

        let mut full_key = [0u8; 64];
        full_key[..key.len()].copy_from_slice(key);

        let val_bytes =
            unsafe { core::slice::from_raw_parts((&val as *const V) as *const u8, size_of::<V>()) };

        #[cfg(target_arch = "riscv32")]
        unsafe {
            let packed_lens: u32 = ((key.len() as u32) << 16) | (domain.len() as u32);
            core::arch::asm!(
                "li a7, 2", // syscall_storage_write
                "ecall",
                in("a1") address.as_ref().as_ptr(), // a1 - address ptr
                in("a2") domain.as_ptr(), // a2 - domain ptr
                in("a3") full_key.as_ptr(), // a3 - key ptr
                in("a4") packed_lens, // a4 - packed lens (domain | key)
                in("a5") val_bytes.as_ptr(), // a5 - value ptr
                in("a6") val_bytes.len(), // a6 - value len
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

#[macro_export]
macro_rules! Map {
    ($name:ident) => {
        pub struct $name;

        impl $name {
            pub const DOMAIN_NAME: &'static str = stringify!($name);
            pub const DOMAIN_NAME_LEN: usize = stringify!($name).len();
            const MAX_KEY_LEN: usize = 64;

            fn build_key<K: $crate::StorageKey>(key: K, out: &mut [u8]) -> usize {
                let key_bytes = key.as_storage_key();
                let key_len = key_bytes.len();

                // 🛡 Copy key bytes into separate buffer to prevent aliasing
                let mut tmp = [0u8; 64];
                tmp[..key_len].copy_from_slice(key_bytes);

                // ✅ Write key to output buffer
                out[..key_len].copy_from_slice(&tmp[..key_len]);

                $crate::require(key_len <= Self::MAX_KEY_LEN, b"key too long");

                key_len
            }

            pub fn get<K, V>(
                address: &$crate::types::address::Address,
                key: K,
            ) -> $crate::types::O<V>
            where
                K: $crate::StorageKey,
                V: Copy + Default,
            {
                let mut buf = [0u8; Self::MAX_KEY_LEN];
                let total_len = Self::build_key(key, &mut buf);
                $crate::StorageMap::get::<V>(
                    address,
                    Self::DOMAIN_NAME.as_bytes(),
                    &buf[..total_len],
                )
            }

            pub fn set<K, V>(address: &$crate::types::address::Address, key: K, val: V)
            where
                K: $crate::StorageKey,
                V: Copy,
            {
                let mut buf = [0u8; Self::MAX_KEY_LEN];
                let total_len = Self::build_key(key, &mut buf);
                $crate::StorageMap::set::<V>(
                    address,
                    Self::DOMAIN_NAME.as_bytes(),
                    &buf[..total_len],
                    val,
                );
            }
        }
    };
}
