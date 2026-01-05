use crate::SerializeField;

impl SerializeField for u8 {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        if *offset + 1 <= buf.len() {
            buf[*offset] = *self;
            *offset += 1;
        }
    }
}

impl SerializeField for bool {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        if *offset + 1 <= buf.len() {
            buf[*offset] = *self as u8;
            *offset += 1;
        }
    }
}

impl SerializeField for u32 {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        let bytes = self.to_le_bytes();
        if *offset + 4 <= buf.len() {
            buf[*offset..*offset + 4].copy_from_slice(&bytes);
            *offset += 4;
        }
    }
}

impl SerializeField for u64 {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        let bytes = self.to_le_bytes();
        if *offset + 8 <= buf.len() {
            buf[*offset..*offset + 8].copy_from_slice(&bytes);
            *offset += 8;
        }
    }
}

// ——— Array impl for any `[u8; N]` ——————————————————

impl<const N: usize> SerializeField for [u8; N] {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        if *offset + N <= buf.len() {
            buf[*offset..*offset + N].copy_from_slice(self);
            *offset += N;
        }
    }
}
