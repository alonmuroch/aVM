#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Represents persistent storage for the blockchain virtual machine.
/// 
/// EDUCATIONAL PURPOSE: This struct provides a simple key-value storage
/// system that simulates how blockchain systems store persistent data.
/// In real blockchains, this would be implemented using databases, file
/// systems, or distributed storage systems.
/// 
/// STORAGE CONCEPTS:
/// - Key-value pairs for simple data storage
/// - Persistent across VM restarts
/// - Thread-safe access using RefCell
/// - Ordered storage using BTreeMap
/// - Domain-based storage organization
/// 
/// REAL-WORLD BLOCKCHAIN COMPARISON:
/// In Ethereum, storage is organized differently:
/// - Each contract has its own storage space
/// - Storage is accessed via 32-byte keys and values
/// - Storage is part of the global state trie
/// - Gas costs are associated with storage operations
/// 
/// MEMORY MANAGEMENT: Uses BTreeMap for ordered iteration and efficient
/// lookups. The RefCell wrapper provides interior mutability, allowing
/// the storage to be modified even when borrowed immutably.
/// 
/// PERSISTENCE: This is an in-memory implementation. In production,
/// data would be persisted to disk or a database to survive system
/// restarts and crashes.
/// 
/// THREAD SAFETY: The current implementation is not thread-safe. In a
/// real blockchain, storage would need to handle concurrent access
/// from multiple transactions and validators.
#[derive(Debug, Default)]
pub struct Storage {
    /// Internal key-value storage using BTreeMap for ordered iteration
    /// 
    /// EDUCATIONAL: BTreeMap provides O(log n) lookups and ordered
    /// iteration, which is useful for debugging and deterministic
    /// behavior. RefCell allows interior mutability while maintaining
    /// Rust's borrowing rules at runtime.
    pub map: RefCell<BTreeMap<String, Vec<u8>>>,
}

impl Storage {
    /// Creates a new empty storage instance.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates storage initialization.
    /// In blockchain systems, storage is typically initialized empty
    /// and populated as transactions are processed.
    /// 
    /// MEMORY ALLOCATION: Creates an empty BTreeMap that will grow
    /// as data is added. This is memory-efficient for systems that
    /// start with little data.
    /// 
    /// USAGE: Use this when starting a new blockchain or when
    /// resetting storage for testing purposes.
    pub fn new() -> Self {
        Self::with_map(BTreeMap::new())
    }

    /// Creates a new storage instance with pre-populated data.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates how storage can be
    /// initialized with existing data, such as when loading from
    /// a database or file system.
    /// 
    /// USAGE: Useful for testing with known data sets or when
    /// restoring storage from a backup or snapshot.
    /// 
    /// PARAMETERS:
    /// - initial: Pre-existing key-value pairs to populate storage
    pub fn with_map(initial: BTreeMap<String, Vec<u8>>) -> Self {
        Self {
            map: RefCell::new(initial),
        }
    }

    /// Builds a composite key from domain and key components.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates how to create hierarchical
    /// storage keys that include domain information for better organization.
    /// 
    /// KEY FORMAT: The composite key is formatted as "domain:key" to
    /// ensure proper separation and avoid key collisions between domains.
    /// 
    /// PARAMETERS:
    /// - domain: The storage domain (e.g., contract name, module name)
    /// - key: The specific key within that domain
    /// 
    /// RETURNS: A composite key string in the format "domain:key"
    fn build_composite_key(domain: &str, key: &str) -> String {
        format!("{}:{}", domain, key)
    }

    /// Retrieves a value from storage by domain and key.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates safe storage access with domain
    /// separation. Returns None if the key doesn't exist, which is common in
    /// blockchain systems where storage is sparse.
    /// 
    /// MEMORY SAFETY: Uses RefCell::borrow() to get immutable access
    /// to the storage map. This ensures thread safety at runtime.
    /// 
    /// CLONING: The value is cloned to avoid lifetime issues. In a
    /// production system, you might use references or more sophisticated
    /// memory management to avoid copying large values.
    /// 
    /// PARAMETERS:
    /// - domain: The storage domain
    /// - key: The storage key to look up within the domain
    /// 
    /// RETURNS: Some(value) if the key exists, None otherwise
    pub fn get(&self, domain: &str, key: &str) -> Option<Vec<u8>> {
        let composite_key = Self::build_composite_key(domain, key);
        self.map.borrow().get(&composite_key).cloned()
    }

    /// Stores a value in storage with the specified domain and key.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates persistent storage updates with
    /// domain organization. In blockchain systems, storage changes are part
    /// of the transaction and are committed atomically with the rest of the
    /// state changes.
    /// 
    /// MUTABILITY: Uses RefCell::borrow_mut() to get mutable access
    /// to the storage map. This allows modification while maintaining
    /// Rust's borrowing rules.
    /// 
    /// KEY OWNERSHIP: The composite key is converted to a String to ensure
    /// ownership. In a production system, you might use more efficient
    /// key representations or avoid unnecessary allocations.
    /// 
    /// PARAMETERS:
    /// - domain: The storage domain
    /// - key: The storage key within the domain
    /// - value: The data to store
    pub fn set(&self, domain: &str, key: &str, value: Vec<u8>) {
        let composite_key = Self::build_composite_key(domain, key);
        self.map.borrow_mut().insert(composite_key, value);
    }

    /// Dumps the contents of persistent storage for debugging.
    /// 
    /// EDUCATIONAL PURPOSE: This demonstrates how to inspect persistent storage,
    /// which is crucial for understanding how programs store data between runs.
    /// 
    /// STORAGE vs MEMORY: Storage persists between program runs, while memory
    /// is cleared each time. This is like the difference between a hard drive
    /// and RAM in a real computer.
    /// 
    /// OUTPUT FORMAT: Shows each key-value pair in storage, with the value
    /// displayed in hexadecimal format.
    #[cfg(feature = "std")]
    pub fn dump(&self) {
        std::println!("--- Storage Dump ---");
        for (key, value) in self.map.borrow().iter() {
            let key_str = key;
            let value_hex: Vec<String> = value.iter().map(|b| format!("{:02x}", b)).collect();
            std::println!("Key: {:<20} | Value ({} bytes): {}", key_str, value.len(), value_hex.join(" "));
        }
        std::println!("--------------------");
    }
}
