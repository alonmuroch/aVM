extern crate alloc;

use alloc::vec::Vec;
use core::convert::TryInto;
use core::fmt;

use crate::result::Result;
use crate::transaction::Transaction;

/// Represents the result of a transaction execution.
#[derive(Debug, Clone)]
pub struct TransactionReceipt {
    /// Hash of the transaction.
    pub tx: Transaction,

    /// Result status and optional data.
    pub result: Result,

    /// List of log entries generated during execution.
    pub events: Vec<Vec<u8>>,
}

impl TransactionReceipt {
    /// Creates a new TransactionReceipt.
    pub fn new(tx: Transaction, result: Result) -> Self {
        TransactionReceipt {
            tx,
            result,
            events: Vec::new(),
        }
    }

    /// Adds an event to the receipt.
    pub fn add_event(&mut self, event: Vec<u8>) -> &TransactionReceipt {
        self.events.push(event);
        self
    }

    /// Optionally add multiple events at once.
    pub fn set_events(mut self, events: Vec<Vec<u8>>) -> Self {
        self.events = events;
        self
    }

    /// Encode this receipt into a flat little-endian buffer.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.tx.tx_type as u8);
        out.extend_from_slice(&self.tx.to.0);
        out.extend_from_slice(&self.tx.from.0);
        out.extend_from_slice(&(self.tx.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.tx.data);
        out.extend_from_slice(&self.tx.value.to_le_bytes());
        out.extend_from_slice(&self.tx.nonce.to_le_bytes());

        out.push(self.result.success as u8);
        out.extend_from_slice(&self.result.error_code.to_le_bytes());
        out.extend_from_slice(&self.result.data_len.to_le_bytes());
        let data_len = self.result.data_len as usize;
        out.extend_from_slice(&self.result.data[..data_len.min(self.result.data.len())]);

        out.extend_from_slice(&(self.events.len() as u32).to_le_bytes());
        for event in &self.events {
            out.extend_from_slice(&(event.len() as u32).to_le_bytes());
            out.extend_from_slice(event);
        }

        out
    }

    /// Decode a receipt from a buffer, returning the receipt and bytes consumed.
    pub fn decode(encoded: &[u8]) -> Option<(Self, usize)> {
        let mut cursor = 0usize;
        let mut read = |len: usize| -> Option<&[u8]> {
            if cursor + len > encoded.len() {
                return None;
            }
            let slice = &encoded[cursor..cursor + len];
            cursor += len;
            Some(slice)
        };

        let tx_type = *read(1)?.first()?;
        let tx_type = crate::transaction::TransactionType::from_u8(tx_type)?;

        let mut to = [0u8; 20];
        to.copy_from_slice(read(20)?);
        let mut from = [0u8; 20];
        from.copy_from_slice(read(20)?);

        let data_len = u32::from_le_bytes(read(4)?.try_into().ok()?) as usize;
        let data = read(data_len)?.to_vec();

        let value = u64::from_le_bytes(read(8)?.try_into().ok()?);
        let nonce = u64::from_le_bytes(read(8)?.try_into().ok()?);

        let success = *read(1)?.first()? != 0;
        let error_code = u32::from_le_bytes(read(4)?.try_into().ok()?);
        let result_data_len = u32::from_le_bytes(read(4)?.try_into().ok()?);
        let result_len = result_data_len as usize;
        let result_data = read(result_len)?;

        let mut data_buf = [0u8; crate::result::RESULT_DATA_SIZE];
        let copy_len = result_len.min(data_buf.len());
        data_buf[..copy_len].copy_from_slice(&result_data[..copy_len]);

        let mut result = Result {
            success,
            error_code,
            data_len: result_data_len,
            data: data_buf,
        };
        if result.data_len as usize > crate::result::RESULT_DATA_SIZE {
            result.data_len = crate::result::RESULT_DATA_SIZE as u32;
        }

        let event_count = u32::from_le_bytes(read(4)?.try_into().ok()?) as usize;
        let mut events = Vec::with_capacity(event_count);
        for _ in 0..event_count {
            let len = u32::from_le_bytes(read(4)?.try_into().ok()?) as usize;
            let bytes = read(len)?.to_vec();
            events.push(bytes);
        }

        let tx = Transaction {
            tx_type,
            to: crate::address::Address(to),
            from: crate::address::Address(from),
            data,
            value,
            nonce,
        };

        Some((TransactionReceipt { tx, result, events }, cursor))
    }

    /// Encode a receipts list with a count prefix and per-receipt length.
    pub fn encode_list(receipts: &[TransactionReceipt]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(receipts.len() as u32).to_le_bytes());
        for receipt in receipts {
            let encoded = receipt.encode();
            out.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
            out.extend_from_slice(&encoded);
        }
        out
    }

    /// Decode a receipts list produced by `encode_list`.
    pub fn decode_list(encoded: &[u8]) -> Option<Vec<TransactionReceipt>> {
        let mut cursor = 0usize;
        let mut read = |len: usize| -> Option<&[u8]> {
            if cursor + len > encoded.len() {
                return None;
            }
            let slice = &encoded[cursor..cursor + len];
            cursor += len;
            Some(slice)
        };
        let count = u32::from_le_bytes(read(4)?.try_into().ok()?) as usize;
        let mut receipts = Vec::with_capacity(count);
        for _ in 0..count {
            let len = u32::from_le_bytes(read(4)?.try_into().ok()?) as usize;
            let slice = read(len)?;
            let (receipt, consumed) = TransactionReceipt::decode(slice)?;
            if consumed != len {
                return None;
            }
            receipts.push(receipt);
        }
        Some(receipts)
    }
}

impl fmt::Display for TransactionReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Transaction Receipt ===")?;
        writeln!(f, "From: {:?}", self.tx.from)?;
        writeln!(f, "To: {:?}", self.tx.to)?;
        writeln!(f, "Result: {:?}", self.result)?;
        writeln!(f, "Events:")?;

        for (i, event) in self.events.iter().enumerate() {
            write!(f, "  [{}] ", i)?;
            for (j, byte) in event.iter().enumerate() {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:02x}", byte)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
