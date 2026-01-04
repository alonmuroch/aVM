# OS (Alon's OS)

Alon's OS (OS) is a minimal operating system purpose-built for deterministic, blockchain-style program execution. It replaces the current `avm` crate with a layered OS: a bootloader for trust establishment, a kernel that orchestrates stateful execution, and `liba`, the standard library that application programs link against.

## Goals
- Deterministic, replayable execution for consensus environments
- Small, auditable surface area with a clear chain of trust from ROM to apps
- Opinionated syscall and runtime model tailored for blockchain state transitions
- First-class support for smart-contract style programs through `liba`
- Ergonomic developer experience while keeping kernel/runtime minimal

## Layered Architecture
1. **Bootloader**: First-stage loader that verifies the kernel and `liba` images, measures them, and passes a concise boot manifest to the kernel. Runs in a restricted environment with no dynamic allocation.
2. **Kernel**: Manages memory layout, page tables, capabilities, and the syscall surface. Provides deterministic scheduling and ties block context (slot, leader, parent state root) to every execution.
3. **Execution Runtime**: The blockchain-aware executor that will supersede `avm`. It coordinates transaction/block execution, drives the VM, and emits receipts and event logs.
4. **liba (Standard Library)**: Successor of the `clibc` crate, offering safe wrappers over syscalls (storage, logs, crypto, messaging), ABI helpers, and contract-to-contract call utilities.
5. **Tooling**: Compiler and host utilities reused from the existing workspace to build and package aOS images and applications.

## Component Details
### Bootloader
- Validates and measures the kernel and `liba` artifacts before execution.
- Builds a `BootInfo` payload (memory map, entry points, config) handed to the kernel.
- Provides a minimal diagnostic console for early-boot errors.

### Kernel
- Sets up paging and isolates execution contexts (per-transaction or per-program).
- Implements a deterministic scheduler and resource accounting suited for block production.
- Exposes a constrained syscall table: storage access, logging/events, crypto primitives, time/slot metadata, inter-program calls.
- Tracks state roots and receipts to keep execution verifiable.

### Execution Runtime
- Drives the VM for each transaction, wiring block context into the kernel-provided syscalls.
- Applies state transitions via the `state` and `storage` crates and emits receipts for verification.
- Provides hooks for precompiles and deterministic host functions.

### liba (Application Standard Library)
- Derived from the existing `clibc` crate and tailored to aOS.
- Offers ABI types, context helpers, and safe wrappers around syscalls exposed by the kernel.
- Ships default modules for storage, logging/events, cross-program calls, and crypto utilities.

## Program Lifecycle (Happy Path)
1. Bootloader measures kernel + `liba`, builds `BootInfo`, and jumps to the kernel.
2. Kernel sets up memory and installs syscall table based on the boot manifest.
3. For each block, the runtime constructs execution contexts with block metadata and state snapshots.
4. Transactions enter the VM; `liba` mediates syscalls to kernel services.
5. State and receipts are persisted; the resulting state root/receipts are exposed to consensus.

## Relationship to Existing Workspace
- `os` replaces the `avm` crate as the orchestrator/runtime.
- `clibc` is internalized as `liba` inside this crate (module structure will mirror the current APIs).
- `vm`, `state`, `storage`, `types`, and `compiler` remain the core building blocks for CPU execution, state transitions, persistence, shared types, and toolchain support.

## Roadmap (Initial Steps)
- Define kernel <-> runtime <-> `liba` interfaces and shared types.
- Port `clibc` into `liba` and expose it as the supported app-facing API.
- Migrate `avm` responsibilities into the aOS runtime and deprecate `avm`.
- Add a boot manifest format and minimal bootloader stubs to validate and launch the kernel.
- Document syscall semantics and determinism guarantees for contract authors.
