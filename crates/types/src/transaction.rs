use alloc::vec::Vec;
use core::convert::TryInto;

use crate::address::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    /// Type 0 - Regular value transfer (not a contract)
    Transfer = 0,
    /// Type 1 - Account create with program data (contract deployment)
    CreateAccount = 1,
    /// Type 2 - Contract call (calling into existing code)
    ProgramCall = 2,
}

impl TransactionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TransactionType::Transfer),
            1 => Some(TransactionType::CreateAccount),
            2 => Some(TransactionType::ProgramCall),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub tx_type: TransactionType, // type of transaction
    pub to: Address,              // recipient address
    pub from: Address,            // sender public key/address
    pub data: Vec<u8>,            // input data
    pub value: u64,               // amount/value sent
    pub nonce: u64,               // transaction nonce
}

/// Holds a set of transactions to be processed as a unit.
#[derive(Debug, Clone)]
pub struct TransactionBundle {
    pub transactions: Vec<Transaction>,
}

impl TransactionBundle {
    pub fn new(transactions: Vec<Transaction>) -> Self {
        TransactionBundle { transactions }
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Encode the bundle into a flat little-endian buffer that can be copied into guest memory.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.transactions.len() as u32).to_le_bytes());

        for tx in &self.transactions {
            out.push(tx.tx_type as u8);
            out.extend_from_slice(&tx.to.0);
            out.extend_from_slice(&tx.from.0);
            out.extend_from_slice(&(tx.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&tx.data);
            out.extend_from_slice(&tx.value.to_le_bytes());
            out.extend_from_slice(&tx.nonce.to_le_bytes());
        }

        out
    }

    /// Decode a buffer produced by `encode` back into a bundle.
    pub fn decode(encoded: &[u8]) -> Option<Self> {
        let mut cursor = 0usize;

        let mut read = |len: usize| -> Option<&[u8]> {
            if cursor + len > encoded.len() {
                return None;
            }
            let slice = &encoded[cursor..cursor + len];
            cursor += len;
            Some(slice)
        };

        let tx_count_bytes = read(4)?;
        let tx_count = u32::from_le_bytes(tx_count_bytes.try_into().ok()?) as usize;
        let mut transactions = Vec::with_capacity(tx_count);

        for _ in 0..tx_count {
            let tx_type_byte = *read(1)?.first()?;
            let tx_type = TransactionType::from_u8(tx_type_byte)?;

            let mut to = [0u8; 20];
            to.copy_from_slice(read(20)?);
            let mut from = [0u8; 20];
            from.copy_from_slice(read(20)?);

            let data_len_bytes = read(4)?;
            let data_len = u32::from_le_bytes(data_len_bytes.try_into().ok()?) as usize;
            let data = read(data_len)?.to_vec();

            let value_bytes = read(8)?;
            let value = u64::from_le_bytes(value_bytes.try_into().ok()?);

            let nonce_bytes = read(8)?;
            let nonce = u64::from_le_bytes(nonce_bytes.try_into().ok()?);

            transactions.push(Transaction {
                tx_type,
                to: Address(to),
                from: Address(from),
                data,
                value,
                nonce,
            });
        }

        Some(TransactionBundle { transactions })
    }
}
