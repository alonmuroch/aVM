# clibc (Chain Libc)

`clibc` is the smart contract runtime library for the Rust VM. It provides the
guest-facing API surface for syscalls, ABI helpers, storage primitives, logging,
and cross-program calls. The crate is `no_std` and built to run inside the VM.

## Features
- `guest`: APIs intended for contract code running inside the VM.
- `kernel`: helpers used by the kernel/runtime side.

## Module Overview
- `allocator`: VM-backed global allocator (enabled for RISC-V guest builds).
- `call`: cross-program call helper (`call`).
- `entrypoint`: `entrypoint!` macro for defining contract entry functions.
- `event`: `event!` definitions plus `fire_event!` dispatch.
- `integers`: simple integer readers (e.g., `read_u32`).
- `log`: logging macros (`log!`, `logf!`, `concat!`, `concat_str!`) and
  `BufferWriter`.
- `panic`: `vm_panic` helper and guest panic handler.
- `parser`: `DataParser` and `HexCodec` utilities, plus `hex_address!` macro.
- `router`: `decode_calls`, `route`, and `FuncCall` for ABI routing.
- `storage`: `persist_struct!` macro and `Persistent` helpers.
- `storage_map`: `StorageMap`, `StorageKey`, and `Map!` macro for typed domains.
- `syscalls`: shared syscall IDs (storage, events, allocation, transfer).
- `transfer`: `transfer`, `balance`, and convenience macros.

## Macros and Helpers
- `entrypoint!`: declare a contract entry function with a consistent ABI.
- `persist_struct!`: generate storage-backed struct load/store helpers.
- `Map!`: declare a typed storage map domain with get/set helpers.
- `event!` and `fire_event!`: define events and emit them via syscall.
- `log!`/`logf!`: basic logging and formatted logging.
- `transfer!`/`balance!`: concise wrappers for token transfer and balance.
- `hex_address!`: compile-time address parsing helper.
- `require`: guard helper that aborts execution with `vm_panic` on failure.

## Usage
Add the crate to your workspace and import it as `clibc`:

```rust
use clibc::{entrypoint, log, logf};
```
