use core::fmt;
use crate::O;
use crate::SerializeField;

pub const ADDRESS_LEN: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn to_bytes(&self) -> [u8; 20] {
        self.0
    }

    pub fn from_ptr(data: &[u8]) -> O<Self> {
        if data.len() != 20 {
            return O::None;
        }
        
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(data);
        O::Some(Address(bytes))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl SerializeField for Address {
    fn serialize_field(&self, buf: &mut [u8], offset: &mut usize) {
        let bytes = &self.0;
        if *offset + 20 <= buf.len() {
            buf[*offset..*offset + 20].copy_from_slice(bytes);
            *offset += 20;
        } else {
            panic!("Buffer overflow in Address serialization");
        }
    }
}
