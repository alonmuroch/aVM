# Rust VM Examples

This directory contains example smart contracts and programs that demonstrate various features of the Rust VM. Each example is designed to showcase different capabilities and patterns for writing smart contracts that run on the RISC-V-based virtual machine.

## Building the Examples

To build all examples and generate their ABIs:
```bash
make all
```

This will:
1. Compile all Rust source files to RISC-V ELF binaries
2. Generate ABI JSON files for each contract
3. Generate client code for interacting with the contracts

## Available Examples

### 1. **simple.rs** - Basic Integer Comparison
A minimal smart contract that compares two 32-bit integers and returns the larger value.
- **Purpose**: Learn the basics of smart contract structure and data handling
- **Features**: Input parsing, basic computation, result formatting
- **Use cases**: Simple validation logic, conditional execution

### 2. **multi_func.rs** - Multi-Function Router
Demonstrates a contract with multiple functions selected via a routing mechanism.
- **Purpose**: Show how to build contracts with multiple entry points
- **Features**: Function routing, selector-based dispatch
- **Use cases**: Complex contracts with multiple operations

### 3. **storage.rs** - Persistent Storage Demo
Shows how to work with persistent key-value storage in smart contracts.
- **Purpose**: Demonstrate state management and data persistence
- **Features**: Store and retrieve user profiles and configuration data
- **Use cases**: User data management, contract configuration storage

### 4. **erc20.rs** - ERC-20 Token Implementation
A complete ERC-20 token implementation with standard functionality.
- **Purpose**: Showcase a real-world smart contract implementation
- **Features**: Token transfers, approvals, balance tracking, events
- **Use cases**: Fungible tokens, DeFi applications

### 5. **allocator_demo.rs** - Memory Allocation
Demonstrates heap memory allocation using VM syscalls.
- **Purpose**: Show advanced memory management in contracts
- **Features**: Dynamic memory allocation, vector operations
- **Use cases**: Complex data structures, dynamic arrays

### 6. **lib_import.rs** - External Library Usage
Shows how to import and use external Rust libraries (sha2 in this example).
- **Purpose**: Demonstrate library integration in smart contracts
- **Features**: SHA-256 hashing using the sha2 crate
- **Use cases**: Cryptographic operations, data integrity verification

### 7. **call_program.rs** - Cross-Contract Calls
Demonstrates how one contract can call another contract.
- **Purpose**: Show contract composability and interaction
- **Features**: External contract calls, result handling
- **Use cases**: DeFi protocols, modular contract systems

### 8. **logging.rs** - Logging and Tracing
Emits structured logs from inside the VM.
- **Purpose**: Show runtime logging patterns
- **Features**: Kernel log output, string formatting
- **Use cases**: Debugging contract behavior

### 9. **native_transfer.rs** - Native Transfer Syscall
Uses the transfer syscall from a guest program.
- **Purpose**: Demonstrate native token transfers
- **Features**: Syscall usage, parameter encoding
- **Use cases**: Simple payments, account funding

### 10. **dex.rs** - Simple AMM
Implements a basic AMM between native AM and an ERC-20 token.
- **Purpose**: Show multi-contract interactions and liquidity flows
- **Features**: Add/remove liquidity, swaps
- **Use cases**: DeFi primitives

### 11. **ecdsa_verify.rs** - Signature Verification
Verifies an ECDSA signature inside the VM.
- **Purpose**: Demonstrate cryptographic verification
- **Features**: Signature parsing, hashing, verification
- **Use cases**: Auth, permit-style flows

## Project Structure

```
examples/
├── src/                 # Source files for all example contracts
├── bin/                 # Compiled binaries and generated files
│   ├── *.elf           # RISC-V ELF binaries
│   ├── *.abi.json      # Contract ABI definitions
│   └── *_client.rs     # Auto-generated client code
├── Makefile            # Build system
├── Cargo.toml          # Rust dependencies
└── generate_abis.sh    # ABI generation script
```

## Running Tests

Example integration tests run through the `aTester` crate in the workspace:
```bash
make run_examples
```

## Key Concepts Demonstrated

1. **Contract Structure**: How to structure a smart contract with proper entry points
2. **Data Handling**: Parsing input data and formatting output results
3. **State Management**: Using persistent storage for contract state
4. **Function Routing**: Building multi-function contracts with selectors
5. **Contract Interaction**: Making calls between different contracts
6. **ABI Generation**: Automatic generation of contract interfaces
7. **Memory Management**: Dynamic allocation in a constrained environment
8. **External Libraries**: Integrating third-party Rust libraries

## Learning Path

If you're new to smart contract development, we recommend exploring the examples in this order:

1. Start with `simple.rs` to understand basic contract structure
2. Move to `multi_func.rs` to learn about function routing
3. Explore `storage.rs` to understand state persistence
4. Examine `call_program.rs` for cross-contract interactions
5. Review `erc20.rs` for a complete real-world implementation
7. Advanced: `allocator_demo.rs` and `lib_import.rs` for specialized features

## Additional Resources

- Main project README: [../../README.md](../../README.md)
- Blog series: [Building AVM on Medium](https://alonmuroch-65570.medium.com/building-avm-my-rust-journey-through-a-risc-v-smart-contract-virtual-machine-e06de4021e05)
- VM implementation: [../vm/](../vm/)
- Compiler tools: [../compiler/](../compiler/)
