use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::vec::Vec;
use crate::Account;
use types::address::Address;

/// Represents the global state of the blockchain virtual machine.
/// 
/// EDUCATIONAL PURPOSE: This struct manages all accounts in the blockchain,
/// similar to how Ethereum's state trie works. It's the central data structure
/// that tracks all accounts, their balances, code, and storage.
/// 
/// BLOCKCHAIN STATE CONCEPTS:
/// - Each address can have an account
/// - Accounts can be regular (hold value) or contracts (hold code)
/// - All state changes are atomic (all succeed or all fail)
/// - State persists between transactions
/// 
/// REAL-WORLD BLOCKCHAIN COMPARISON:
/// This is a simplified version of Ethereum's state management. In Ethereum:
/// - State is stored in a Merkle Patricia Trie for efficient proofs
/// - Accounts have additional fields like storage root and code hash
/// - State changes are tracked for rollback capability
/// - Gas costs are associated with state operations
/// 
/// DATA STRUCTURE: Uses a HashMap for O(1) account lookups by address.
/// In production blockchains, this would be a more sophisticated data structure
/// like a Merkle tree for efficient state proofs.
/// 
/// MEMORY MANAGEMENT: All accounts are kept in memory for fast access.
/// In production systems, only frequently accessed accounts would be in memory,
/// with the rest stored on disk or in a database.
/// 
/// THREAD SAFETY: This implementation is not thread-safe. In a real blockchain,
/// the state would need to handle concurrent access from multiple transactions
/// and validators.
/// 
/// PERSISTENCE: The state can be reconstructed from storage, though the current
/// implementation is simplified. Real blockchains use sophisticated persistence
/// mechanisms to ensure data durability and recovery.
#[derive(Clone, Debug)]
pub struct State {
    /// Maps addresses to their corresponding accounts.
    /// 
    /// EDUCATIONAL: This is the core data structure that represents the
    /// entire blockchain state. Each entry contains an account with its
    /// balance, code, storage, and other metadata.
    pub accounts: BTreeMap<Address, Account>,
}

impl State {
    /// Creates a new empty state.
    /// 
    /// EDUCATIONAL PURPOSE: This represents the initial state of a blockchain
    /// before any transactions have been processed. In real blockchains,
    /// there might be genesis accounts with initial balances.
    /// 
    /// USAGE: Typically called when starting a new blockchain or when
    /// resetting the state for testing purposes.
    pub fn new() -> Self {
        Self { accounts: BTreeMap::new() }
    }

    /// Retrieves an account by address (immutable reference).
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates safe account access for reading.
    /// Returns None if the account doesn't exist, which is common in
    /// blockchain systems where addresses might not have accounts yet.
    /// 
    /// USAGE: Use this when you need to read account data but not modify it.
    /// This is the preferred method for read-only operations.
    /// 
    /// PARAMETERS:
    /// - addr: The address of the account to retrieve
    /// 
    /// RETURNS: Some(account) if the account exists, None otherwise
    pub fn get_account(&self, addr: &Address) -> Option<&Account> {
        self.accounts.get(addr)
    }

    /// Returns the current balance for an address (0 if missing).
    pub fn balance_of(&self, addr: &Address) -> u128 {
        self.accounts
            .get(addr)
            .map(|acc| acc.balance)
            .unwrap_or(0)
    }

    /// Retrieves an account by address (mutable reference), creating it if it doesn't exist.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates account creation on-demand.
    /// In blockchain systems, accounts are often created implicitly when
    /// they first receive a transaction or are called.
    /// 
    /// ACCOUNT CREATION: If the account doesn't exist, it creates a new one
    /// with default values (0 balance, 0 nonce, no code, not a contract).
    /// 
    /// LAZY INITIALIZATION: This pattern is common in blockchain systems
    /// because it saves storage space - accounts only exist when they're
    /// actually used. This is different from traditional databases where
    /// you might pre-allocate space for all possible accounts.
    /// 
    /// DEFAULT VALUES EXPLANATION:
    /// - nonce: 0 - No transactions have been sent from this account yet
    /// - balance: 0 - No funds have been transferred to this account
    /// - code: Vec::new() - No smart contract code deployed
    /// - is_contract: false - This is a regular account, not a contract
    /// - storage: BTreeMap::new() - No persistent storage allocated
    /// 
    /// MEMORY EFFICIENCY: Using BTreeMap for storage provides ordered
    /// iteration and efficient lookups while using less memory than HashMap
    /// for small datasets.
    /// 
    /// USAGE: Use this when you need to modify account data (e.g., update
    /// balance, deploy code, modify storage).
    /// 
    /// PARAMETERS:
    /// - addr: The address of the account to retrieve or create
    /// 
    /// RETURNS: Mutable reference to the account (guaranteed to exist)
    pub fn get_account_mut(&mut self, addr: &Address) -> &mut Account {
        self.accounts.entry(*addr).or_insert_with(|| Account {
            nonce: 0,                    // No transactions yet
            balance: 0,                  // No initial balance
            code: Vec::new(),            // No code (not a contract)
            is_contract: false,          // Regular account
            storage: BTreeMap::new(),    // Empty storage
        })
    }

    /// Transfers native balance between accounts. Returns false on insufficient funds or overflow.
    pub fn transfer(&mut self, from: &Address, to: &Address, value: u64) -> bool {
        let amount = value as u128;
        let from_balance = match self.get_account(from) {
            Some(account) => account.balance,
            None => return false,
        };
        if from_balance < amount {
            return false;
        }
        if from == to {
            return true;
        }
        let to_balance = self.balance_of(to);
        let new_to_balance = match to_balance.checked_add(amount) {
            Some(balance) => balance,
            None => return false,
        };

        {
            let from_account = self.get_account_mut(from);
            from_account.balance = from_balance - amount;
        }
        {
            let to_account = self.get_account_mut(to);
            to_account.balance = new_to_balance;
        }
        true
    }

    /// Checks if an address corresponds to a contract account.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates how to distinguish between
    /// regular accounts (that hold value) and contract accounts (that hold code).
    /// This is a fundamental concept in blockchain systems.
    /// 
    /// NOTE: This is currently a simplified implementation that always returns true.
    /// In a real system, this would check if the account has code deployed.
    /// 
    /// PARAMETERS:
    /// - _addr: The address to check
    /// 
    /// RETURNS: true if the address is a contract, false otherwise
    pub fn is_contract(&self, _addr: Address) -> bool {
        // EDUCATIONAL: In a real implementation, this would check if the account has code
        // self.accounts.get(addr).map_or(false, |acc| acc.code.is_some())
        return true;
    }   

    /// Encode state into a byte buffer for guest consumption.
    pub fn encode(&self) -> alloc::vec::Vec<u8> {
        let len = self.encoded_len();
        let mut out = alloc::vec![0u8; len];
        let _ = self.encode_into(&mut out);
        out
    }

    /// Returns the byte length of the encoded state.
    pub fn encoded_len(&self) -> usize {
        let mut total = 4usize; // account count
        for (addr, acc) in &self.accounts {
            let mut acc_len = 0usize;
            acc_len = acc_len.saturating_add(addr.0.len());
            acc_len = acc_len.saturating_add(16); // balance
            acc_len = acc_len.saturating_add(8); // nonce
            acc_len = acc_len.saturating_add(1); // is_contract
            acc_len = acc_len.saturating_add(4); // code len
            acc_len = acc_len.saturating_add(acc.code.len());
            acc_len = acc_len.saturating_add(4); // storage len
            for (k, v) in &acc.storage {
                acc_len = acc_len.saturating_add(4); // key len
                acc_len = acc_len.saturating_add(k.as_bytes().len());
                acc_len = acc_len.saturating_add(4); // val len
                acc_len = acc_len.saturating_add(v.len());
            }
            total = total.saturating_add(acc_len);
        }
        total
    }

    /// Encode state into a provided buffer. Returns bytes written on success.
    pub fn encode_into(&self, out: &mut [u8]) -> Option<usize> {
        let mut cursor = 0usize;
        let write = |buf: &mut [u8], cursor: &mut usize, bytes: &[u8]| -> Option<()> {
            if *cursor + bytes.len() > buf.len() {
                return None;
            }
            buf[*cursor..*cursor + bytes.len()].copy_from_slice(bytes);
            *cursor += bytes.len();
            Some(())
        };

        let count = self.accounts.len() as u32;
        write(out, &mut cursor, &count.to_le_bytes())?;

        for (addr, acc) in &self.accounts {
            write(out, &mut cursor, &addr.0)?;
            write(out, &mut cursor, &acc.balance.to_le_bytes())?;
            write(out, &mut cursor, &acc.nonce.to_le_bytes())?;
            write(out, &mut cursor, &[acc.is_contract as u8])?;
            let code_len = acc.code.len() as u32;
            write(out, &mut cursor, &code_len.to_le_bytes())?;
            write(out, &mut cursor, &acc.code)?;

            let storage_len = acc.storage.len() as u32;
            write(out, &mut cursor, &storage_len.to_le_bytes())?;
            for (k, v) in &acc.storage {
                let key_len = k.as_bytes().len() as u32;
                write(out, &mut cursor, &key_len.to_le_bytes())?;
                write(out, &mut cursor, k.as_bytes())?;
                let val_len = v.len() as u32;
                write(out, &mut cursor, &val_len.to_le_bytes())?;
                write(out, &mut cursor, v)?;
            }
        }

        Some(cursor)
    }

    /// Decode state produced by `encode`.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut cursor = 0usize;
        let mut read = |len: usize| -> Option<&[u8]> {
            if cursor + len > bytes.len() {
                return None;
            }
            let slice = &bytes[cursor..cursor + len];
            cursor += len;
            Some(slice)
        };

        let count = {
            let raw = read(4)?;
            let mut buf = [0u8; 4];
            buf.copy_from_slice(raw);
            u32::from_le_bytes(buf) as usize
        };

        let mut accounts = BTreeMap::new();
        for _ in 0..count {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(read(20)?);

            let balance = {
                let mut buf = [0u8; 16];
                buf.copy_from_slice(read(16)?);
                u128::from_le_bytes(buf)
            };

            let nonce = {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(read(8)?);
                u64::from_le_bytes(buf)
            };

            let is_contract = read(1)?.first().copied()? != 0;

            let code_len = {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(read(4)?);
                u32::from_le_bytes(buf) as usize
            };
            let code = read(code_len)?.to_vec();

            let storage_len = {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(read(4)?);
                u32::from_le_bytes(buf) as usize
            };
            let mut storage = BTreeMap::new();
            for _ in 0..storage_len {
                let key_len = {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(read(4)?);
                    u32::from_le_bytes(buf) as usize
                };
                let key = {
                    let raw = read(key_len)?;
                    core::str::from_utf8(raw).ok()?.to_string()
                };

                let val_len = {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(read(4)?);
                    u32::from_le_bytes(buf) as usize
                };
                let val = read(val_len)?.to_vec();

                storage.insert(key, val);
            }

            accounts.insert(
                Address(addr),
                Account {
                    nonce,
                    balance,
                    code,
                    is_contract,
                    storage,
                },
            );
        }

        Some(Self { accounts })
    }

}
