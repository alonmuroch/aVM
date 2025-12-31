# 🚀 Rust VM: Learn Blockchain Virtual Machines from the Ground Up

> **"The best way to understand how something works is to build it yourself."**

**rust-vm** is an educational blockchain virtual machine written in Rust that teaches you how virtual machines, smart contracts, and blockchain systems actually work. Instead of just reading about these concepts, you can explore a real implementation with comprehensive documentation that explains every design decision.

---

## 📖 **Start Here: Read the Blog Series**

Before diving into the code, we recommend reading our comprehensive blog series that walks you through the entire project:

### 🎯 **Chapter 1: Building AVM: My Rust Journey Through a RISC-V Smart Contract Virtual Machine**
**[Read the full article on Medium →](https://alonmuroch-65570.medium.com/building-avm-my-rust-journey-through-a-risc-v-smart-contract-virtual-machine-e06de4021e05)**

This first chapter introduces the fundamental concepts behind blockchain virtual machines and explains why we built this project. You'll learn:
- What virtual machines are and why they're crucial for blockchain technology
- How smart contracts actually work under the hood
- The design philosophy behind our educational approach
- What you'll learn by studying this implementation

### 🎯 **Chapter 2: AVM Report #2, Now Supports A Full ERC-20 Implementation**
**[Read the full article on Medium →](https://alonmuroch-65570.medium.com/avm-report-2-now-supports-a-full-erc-20-implementation-feed40e297ba)**

This second chapter covers the evolution of AVM from a basic instruction interpreter into a modular, extensible smart contract runtime. You'll learn about:
- Cross-program contract calls and composability
- RISC-V IMAC instruction compliance testing
- Persistent key-value storage systems
- A complete ERC-20 implementation with events and logging
- Transaction receipts and event decoding

*More chapters coming soon covering advanced features, optimizations, and real-world applications!*

---

**💡 Pro Tip**: Read each blog chapter first, then explore the corresponding code sections. This combination of theory and practice will give you the deepest understanding.

---

## 🎯 Why This Project Exists

Blockchain technology is revolutionizing how we think about trust, decentralization, and programmable money. But understanding how it all works under the hood can be daunting. This project bridges that gap by providing:

- **🧠 Complete Implementation**: A working RISC-V-based virtual machine that can execute smart contracts
- **📚 Comprehensive Documentation**: Every function, struct, and design decision is thoroughly explained
- **🎓 Educational Focus**: Designed specifically for learning, not production use
- **🔍 Transparent Code**: No magic, no black boxes - everything is visible and understandable

---

## 🌟 What You'll Learn

### 🏗️ **Virtual Machine Architecture**
- How CPUs fetch, decode, and execute instructions
- RISC-V instruction set and binary encoding
- Memory management and isolation between contracts
- Deterministic execution for blockchain environments

### 🔗 **Blockchain Concepts**
- Smart contract deployment and execution
- Account-based state management (like Ethereum)
- Persistent storage and state transitions
- Transaction processing and atomicity

### 🦀 **Rust Systems Programming**
- Zero-copy data structures and memory safety
- Interior mutability with `RefCell` and `Rc`
- No-std programming for embedded systems
- Defensive programming and error handling

### 🧩 **System Design Patterns**
- Modular architecture with clear boundaries
- Strategy patterns for extensibility
- Resource management and safety limits
- Error handling and graceful degradation

---

## ✨ Key Features

### 🧠 **RISC-V Instruction Set**
- **RV32IM**: Integer + multiply/divide (M). Atomics (A) are implemented; compressed (C) decoding is present but experimental—core target is RV32IM.

### 🦀 **Rust Smart Contracts**
- Write contracts in pure Rust (no WebAssembly needed)
- Direct compilation to VM bytecode
- Rich standard library with storage, logging, and validation
- Type-safe contract interfaces

### 🪶 **Minimal & Educational Design**
- **Clear Architecture**: Each component has a single, well-defined responsibility
- **Comprehensive Comments**: Every function explains its purpose, design decisions, and educational value
- **Real-World Comparisons**: See how this relates to Ethereum, Bitcoin, and other blockchain systems
- **Performance Insights**: Understand the trade-offs between simplicity and efficiency

### 🔐 **Blockchain-Ready Features**
- **Deterministic Execution**: Same input always produces same output
- **Memory Isolation**: Contracts can't interfere with each other
- **State Management**: Account-based system with persistent storage
- **Transaction Processing**: Atomic execution with rollback capability

---

## 📁 Project Structure

```
rust-vm/
├── crates/
│   ├── avm/           # Application Virtual Machine - main orchestrator
│   ├── compiler/      # Rust-to-bytecode compiler
│   ├── examples/      # Smart contract examples and tutorials
│   │   ├── README.md  # 📖 [Detailed guide to all examples](crates/examples/README.md)
│   │   └── src/
│   │       ├── simple.rs      # Basic smart contract example
│   │       ├── erc20.rs       # Full ERC-20 implementation
│   │       ├── storage.rs     # Storage system examples
│   │       ├── multi_func.rs  # Multi-function contract
│   │       └── call_program.rs # Cross-program calls
│   ├── clibc/         # Chain Libc smart contract runtime library
│   ├── state/         # Blockchain state management
│   ├── storage/       # Persistent storage system
│   ├── types/         # Common types and data structures
│   └── vm/            # RISC-V virtual machine core
│       └── tests/     # RISC-V compliance tests
│           └── riscv-tests/   # Official RISC-V test suite
├── Cargo.toml         # Workspace configuration
├── Makefile           # Build and test automation
└── README.md          # This file
```

### 🏛️ **Architecture Overview**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Transaction   │    │   Smart Contract│    │   RISC-V VM     │
│   Processing    │───▶│   Execution     │───▶│   Core          │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   State Mgmt    │    │   Memory Mgmt   │    │   Instruction   │
│   (Accounts)    │    │   (Pages)       │    │   Decoder       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Storage       │    │   Context Stack │    │   CPU & Regs    │
│   (Persistent)  │    │   (Call Chain)  │    │   (Execution)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

---

## 🚀 Getting Started

### Prerequisites
- Rust 1.70+ with Cargo
- Basic understanding of Rust syntax
- Curiosity about how computers work at a low level

### Quick Start

1. **Clone and Explore**
   ```bash
   git clone https://github.com/your-username/rust-vm.git
   cd rust-vm
   ```

2. **Explore the Examples**
   We provide comprehensive examples with detailed documentation:
   - **📖 [Examples Guide](crates/examples/README.md)** - Complete guide to all smart contract examples
   - Start with `crates/examples/src/simple.rs` - a basic smart contract that compares two numbers
   - Progress through increasingly complex examples like ERC-20 tokens and cross-contract calls

3. **Explore the Core Components**
   - **CPU**: `crates/vm/src/cpu.rs` - See how instructions are executed
   - **Decoder**: `crates/vm/src/decoder.rs` - Learn how binary data becomes instructions
   - **State**: `crates/state/src/state.rs` - Understand blockchain state management
   - **AVM**: `crates/avm/src/avm.rs` - The main virtual machine orchestrator

4. **Build and Run Tests**
   ```bash
   make all
   ```

---

## 🔍 Deep Dive: Key Components

### 🧠 **The CPU (`vm/src/cpu.rs`)**
The heart of the virtual machine. Learn about:
- **Instruction Cycle**: Fetch → Decode → Execute
- **Register Management**: 32 general-purpose registers
- **Program Counter**: How execution flows through code
- **Error Handling**: Graceful handling of invalid instructions

### 🔧 **The Decoder (`vm/src/decoder.rs`)**
Converts binary data into executable instructions:
- **RISC-V Formats**: R-type, I-type, S-type, B-type, U-type, J-type
- **Bit Manipulation**: How to extract fields from 32-bit words
- **Compressed Instructions**: 16-bit variants for code size optimization
- **Immediate Extraction**: Different patterns for different instruction types

### 🏦 **The State Manager (`state/src/state.rs`)**
Manages the blockchain's global state:
- **Account Model**: Like Ethereum's account-based system
- **Lazy Initialization**: Accounts created only when needed
- **Contract Deployment**: How smart contracts are installed
- **State Transitions**: Atomic updates across all accounts

### 💾 **The Storage System (`storage/src/lib.rs`)**
Provides persistent data storage:
- **Key-Value Store**: Simple but effective storage model
- **Memory Safety**: Using Rust's type system for safety
- **Persistence**: Data survives across transactions
- **Thread Safety**: Considerations for concurrent access

---

## 🎯 Real-World Applications

Understanding this VM helps you understand:

- **Ethereum's EVM**: How smart contracts are executed
- **Bitcoin's Script**: How transaction validation works
- **Solana's Runtime**: How high-performance blockchains operate
- **WebAssembly**: How portable code execution works
- **Operating Systems**: How processes and memory management work

---

## 🤝 Contributing

This project is designed for learning and education. Contributions that improve:

- **Documentation**: Better explanations, more examples, clearer concepts
- **Examples**: More smart contract examples, tutorials, use cases
- **Educational Value**: Better learning materials, diagrams, explanations
- **Code Clarity**: Cleaner implementations, better comments, more readable code

...are especially welcome!

### How to Contribute
1. Fork the repository
2. Create a feature branch
3. Make your changes with educational focus
4. Add comprehensive documentation
5. Submit a pull request

---

## 📖 Further Reading

### Books
- "Computer Organization and Design" by Patterson & Hennessy
- "Programming Rust" by Blandy & Orendorff
- "Mastering Ethereum" by Antonopoulos & Wood

### Papers
- [RISC-V Specification](https://riscv.org/specifications/)
- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
- [Bitcoin Whitepaper](https://bitcoin.org/bitcoin.pdf)

### Online Resources
- [RISC-V Foundation](https://riscv.org/)
- [RISC-V Specification](https://riscv.org/specifications/) (also available in `crates/vm/tests/riscv-tests/`)
- [Ethereum Documentation](https://ethereum.org/developers/)
- [Rust Book](https://doc.rust-lang.org/book/)

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

This project was created to help students and developers understand the fascinating world of virtual machines and blockchain technology. Special thanks to:

- The RISC-V Foundation for the open instruction set architecture
- The Rust community for the amazing programming language
- The blockchain community for pushing the boundaries of what's possible
- All the students and developers who will learn from this project

---

**Ready to dive deep into the world of virtual machines? Start exploring the code! 🚀**

> *"The only way to learn a new programming language is by writing programs in it."* - Dennis Ritchie
