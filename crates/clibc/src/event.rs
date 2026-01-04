// use types::SerializeField;
// use crate::vm_panic;

// ——— Blanket fallback: any `T: BytesSerialization` —————

// impl<T: BytesSerialization> SerializeField for T {
//     fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
//         // let bytes = self.as_bytes();
//         // let len = bytes.len();
//         // if *offset + len <= buf.len() {
//         //     buf[*offset..*offset + len].copy_from_slice(bytes);
//         //     *offset += len;
//         // }
//         vm_panic(b"SerializeField not implemented for this type");
//     }
// }

// ——— The `event!` macro —————————————————————————————

#[macro_export]
macro_rules! event {
    (
        $name:ident {
            $(
                $fname:ident => $ftype:ty
            ),* $(,)?
        }
    ) => {
        pub struct $name {
            pub id: [u8; 32],
            $(pub $fname: $ftype),*
        }

        impl $name {
            /// Creates a new event, auto-initializing `id` to the first
            /// 32 bytes of the type name, and setting each field.
            pub fn new($($fname: $ftype),*) -> Self {
                // id from name
                let mut id = [0u8; 32];
                let name_bytes = stringify!($name).as_bytes();
                let copy_len = core::cmp::min(name_bytes.len(), 32);
                id[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

                Self {
                    id,
                    $($fname),*
                }
            }
        }

        impl $name {
            /// Serialize all fields (including the 32-byte id) into `buf`.
            pub fn write_bytes(&self, buf: &mut [u8]) -> usize {
                let mut offset = 0;
                // first serialize the id
                buf[..32].copy_from_slice(&self.id);
                offset += 32;
                // then each declared field
                $(
                    <$ftype as $crate::types::SerializeField>::serialize_field(
                        &self.$fname,
                        buf,
                        &mut offset,
                    );
                )*
                offset
            }
        }
    };
}

// ——— And (optionally) your existing fire_event! macro ———
#[macro_export]
macro_rules! fire_event {
    ($event:expr) => {{
        let mut buffer = [0u8; 256];
        let written = $event.write_bytes(&mut buffer);
        let ptr = buffer.as_ptr() as u32;
        let len = written as u32;
        #[cfg(target_arch = "riscv32")]
        unsafe {
            core::arch::asm!(
                "li a7, 6",
                "ecall",
                in("a1") ptr,
                in("a2") len,
            );
        }
        #[cfg(not(target_arch = "riscv32"))]
        {
            // For non-RISC-V targets, do nothing
        }
    }};
}
