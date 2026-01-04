use clibc::{log, logf};
use types::{Address, ADDRESS_LEN};

use state::State;

use crate::global::{CURRENT_TASK, KERNEL_TASK_SLOT, STATE};
use crate::memory::page_allocator as mmu;
use crate::syscall::alloc::sys_alloc;
use crate::syscall::storage::{current_task_root_ppn, read_user_bytes};
use crate::global::FROM_PTR_ADDR;

pub(crate) fn sys_transfer(args: [u32; 6]) -> u32 {
    let current = unsafe { *CURRENT_TASK.get_mut() };
    if current == KERNEL_TASK_SLOT {
        log!("sys_transfer: kernel task not allowed");
        return 1;
    }

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 1,
    };

    let to_ptr = args[1];
    let value = (args[2] as u64) | ((args[3] as u64) << 32);

    let from_bytes = match read_user_bytes(root_ppn, FROM_PTR_ADDR, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 1,
    };
    let to_bytes = match read_user_bytes(root_ppn, to_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 1,
    };
    if from_bytes.len() != ADDRESS_LEN || to_bytes.len() != ADDRESS_LEN {
        log!("sys_transfer: invalid address length");
        return 1;
    }

    let mut from_buf = [0u8; ADDRESS_LEN];
    let mut to_buf = [0u8; ADDRESS_LEN];
    from_buf.copy_from_slice(&from_bytes);
    to_buf.copy_from_slice(&to_bytes);
    let from = Address(from_buf);
    let to = Address(to_buf);

    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    let ok = state.transfer(&from, &to, value);
    if ok { 0 } else { 1 }
}

pub(crate) fn sys_balance(args: [u32; 6]) -> u32 {
    let current = unsafe { *CURRENT_TASK.get_mut() };
    if current == KERNEL_TASK_SLOT {
        log!("sys_balance: kernel task not allowed");
        return 0;
    }

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 0,
    };
    let addr_ptr = args[0];
    let address_bytes = match read_user_bytes(root_ppn, addr_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 0,
    };
    if address_bytes.len() != ADDRESS_LEN {
        log!("sys_balance: invalid address length");
        return 0;
    }
    let mut addr_buf = [0u8; ADDRESS_LEN];
    addr_buf.copy_from_slice(&address_bytes);
    let address = Address(addr_buf);

    let balance = unsafe { STATE.get_mut() }
        .as_ref()
        .map(|state| state.balance_of(&address))
        .unwrap_or(0);

    let addr = sys_alloc([16, 8, 0, 0, 0, 0]);
    if addr == 0 {
        log!("sys_balance: allocation failed");
        return 0;
    }
    let bytes = balance.to_le_bytes();
    if !mmu::copy(root_ppn, addr, &bytes) {
        logf!("sys_balance: failed to write to 0x%x", addr);
        return 0;
    }
    addr
}
