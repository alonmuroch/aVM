extern crate alloc;

use alloc::vec::Vec;
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
