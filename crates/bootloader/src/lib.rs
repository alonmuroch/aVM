#![cfg_attr(target_arch = "riscv32", no_std)]
//! Deterministic OS scaffold for blockchain-style execution.
//!
//! This crate provides a bootloader skeleton that:
//! - loads a kernel program into fresh pages,
//! - hands the loaded image off to a future kernel runtime.
//!
//! Memory utilities are provided by the VM crate.

pub mod bootloader;
pub mod result;

pub mod syscalls;

pub use vm::sys_call;

pub use syscalls::DefaultSyscallHandler;
