//! Simple parser for reading typed values from a byte slice.
use types::address::Address;
use crate::vm_panic;

pub struct DataParser<'a> {
    data: &'a [u8],
    offset: usize,
}

/// Lightweight hex codec for no_std contexts.
pub struct HexCodec;

impl HexCodec {
    pub fn decode_into(input: &[u8], out: &mut [u8]) {
        for (i, chunk) in input.chunks_exact(2).enumerate() {
            if i >= out.len() {
                break;
            }
            out[i] = (Self::hex_val(chunk[0]) << 4) | Self::hex_val(chunk[1]);
        }
    }

    /// Decode a 40-byte hex string into an Address.
    pub fn decode_address(hex: &[u8]) -> Address {
        if hex.len() != 40 {
            vm_panic(b"hex address must be 40 bytes");
        }
        let mut buf = [0u8; 20];
        Self::decode_into(hex, &mut buf);
        Address(buf)
    }

    /// Macro-friendly helper to parse a hex string literal into an Address.
    /// Prefer using the `hex_address!` macro for call-sites.
    pub fn decode_address_literal(hex: &'static [u8]) -> Address {
        Self::decode_address(hex)
    }

    pub fn encode<'a>(bytes: &[u8], out: &'a mut [u8]) -> &'a [u8] {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let needed = bytes.len() * 2;
        let max = out.len().min(needed);
        for (i, &b) in bytes.iter().enumerate() {
            let idx = i * 2;
            if idx + 1 >= max {
                break;
            }
            out[idx] = HEX[(b >> 4) as usize];
            out[idx + 1] = HEX[(b & 0x0f) as usize];
        }
        &out[..max]
    }

    fn hex_val(b: u8) -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => vm_panic(b"invalid hex"),
        }
    }
}

/// Convenience macro to parse a 40-hex-char string literal into an `Address`.
#[macro_export]
macro_rules! hex_address {
    ($hex:literal) => {{
        $crate::parser::HexCodec::decode_address_literal($hex.as_bytes())
    }};
}

impl<'a> DataParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn ensure(&self, bytes: usize) {
        if self.offset + bytes > self.data.len() {
            vm_panic(b"insufficient input data");
        }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    pub fn read_bytes(&mut self, len: usize) -> &'a [u8] {
        self.ensure(len);
        let start = self.offset;
        self.offset += len;
        &self.data[start..start + len]
    }

    /// Peek at the upcoming bytes without advancing the cursor.
    pub fn peek_bytes(&self, len: usize) -> &'a [u8] {
        self.ensure(len);
        &self.data[self.offset..self.offset + len]
    }

    /// Read a hex-encoded byte string (2 hex chars per byte) into `out`.
    /// Returns the slice of `out` that was filled.
    pub fn read_hex_into<'b>(&mut self, out: &'b mut [u8]) -> &'b [u8] {
        let hex_len = out.len().saturating_mul(2);
        self.ensure(hex_len);
        let hex_bytes = self.read_bytes(hex_len);

        HexCodec::decode_into(hex_bytes, out);

        &out[..out.len()]
    }

    /// Read a 40-byte hex-encoded address into an Address.
    pub fn read_hex_address(&mut self) -> Address {
        let mut buf = [0u8; 20];
        self.read_hex_into(&mut buf);
        Address(buf)
    }

    pub fn read_u32(&mut self) -> u32 {
        let bytes: [u8; 4] = self.read_bytes(4).try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    pub fn read_u64(&mut self) -> u64 {
        let bytes: [u8; 8] = self.read_bytes(8).try_into().unwrap();
        u64::from_le_bytes(bytes)
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_bytes(1)[0] != 0
    }

    pub fn read_address(&mut self) -> Address {
        let bytes = self.read_bytes(20);
        let mut arr = [0u8; 20];
        arr.copy_from_slice(bytes);
        Address(arr)
    }
}
