/// Maximum size of data that can be stored in a Result
pub const RESULT_DATA_SIZE: usize = 256;

/// Total size of the Result struct in bytes
pub const RESULT_SIZE: usize = 1 + 4 + 4 + RESULT_DATA_SIZE; // success + error_code + data_len + data

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Result {
    pub success: bool,
    pub error_code: u32,
    pub data_len: u32,
    pub data: [u8; RESULT_DATA_SIZE],
}

impl Result {
    /// Creates a new Result with the given success status and error code
    pub fn new(success: bool, error_code: u32) -> Self {
        Self {
            success,
            error_code,
            data_len: 0,
            data: [0; RESULT_DATA_SIZE],
        }
    }

    /// Creates a new Result with data
    pub fn new_with_data(success: bool, error_code: u32, data: &[u8]) -> Self {
        let mut result = Self::new(success, error_code);
        result.set_data(data);
        result
    }

    /// Sets the data field with the provided bytes
    pub fn set_data(&mut self, data: &[u8]) {
        let len = data.len().min(RESULT_DATA_SIZE);
        self.data_len = len as u32;
        self.data[..len].copy_from_slice(&data[..len]);
    }

    /// Creates a Result with success=true and the u32 value stored in data
    pub fn with_u32(value: u32) -> Self {
        let mut result = Self::new(true, 0);
        result.set_data(&value.to_le_bytes());
        result
    }

    /// Creates a Result with success=false and the u32 error code stored in data
    pub fn with_u32_error(error_code: u32) -> Self {
        let mut result = Self::new(false, error_code);
        result.set_data(&error_code.to_le_bytes());
        result
    }

    /// Gets the data as a u32 value (assumes data contains a u32 in little-endian format)
    pub fn get_u32_data(&self) -> Option<u32> {
        if self.data_len >= 4 {
            let bytes = &self.data[..4];
            Some(u32::from_le_bytes(bytes.try_into().unwrap()))
        } else {
            None
        }
    }

    /// # Safety
    /// Caller must ensure `ptr` points to at least 5 valid bytes in memory.
    #[cfg(target_arch = "riscv32")]
    pub fn from_ptr(result_ptr: u32) -> Option<Self> {
        // SAFETY: We assume the memory starting at `result_ptr` is valid and readable.
        let ptr = result_ptr as *const u8;
        if ptr.is_null() {
            return None;
        }

        unsafe {
            // Read the `success` byte
            let success = *ptr != 0;

            // Read the next 4 bytes as the error code
            let raw = core::slice::from_raw_parts(ptr.add(1), 4);
            let error_code = u32::from_le_bytes(raw.try_into().unwrap());

            // Read the next 4 bytes as the data_len
            let data_len_raw = core::slice::from_raw_parts(ptr.add(5), 4);
            let data_len = u32::from_le_bytes(data_len_raw.try_into().unwrap());

            // Read the next RESULT_DATA_SIZE bytes as the data
            let data_raw = core::slice::from_raw_parts(ptr.add(9), RESULT_DATA_SIZE);
            let data = data_raw.try_into().unwrap();

            Some(Result {
                success,
                error_code,
                data_len,
                data,
            })
        }
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub unsafe fn from_ptr(_result_ptr: u32) -> Option<Self> {
        panic!("not implemented")
    }
}
