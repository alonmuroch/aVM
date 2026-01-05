extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use core::cmp;

use clibc::{log, logf};
use types::{ADDRESS_LEN, Address, SV32_DIRECT_MAP_BASE, SV32_PAGE_SIZE};

use crate::global::TO_PTR_ADDR;
use crate::global::{CURRENT_TASK, KERNEL_TASK_SLOT, STATE, TASKS};
use crate::memory::page_allocator as mmu;
use crate::syscall::alloc::sys_alloc;
use state::State;

pub(crate) fn sys_storage_get(args: [u32; 6]) -> u32 {
    let address_ptr = args[0];
    let domain_ptr = args[1];
    let key_ptr = args[2];
    let lens_packed = args[3] as usize;
    let domain_len = lens_packed & 0xffff;
    let key_len = lens_packed >> 16;

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 0,
    };

    let address_bytes = match read_user_bytes(root_ppn, address_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let mut addr_buf = [0u8; ADDRESS_LEN];
    if address_bytes.len() != ADDRESS_LEN {
        log!("sys_storage_get: invalid address length");
        return 0;
    }
    addr_buf.copy_from_slice(&address_bytes);
    let address = Address(addr_buf);
    if !caller_address_matches(root_ppn, &address) {
        log!("sys_storage_get: address mismatch with caller");
        return 0;
    }

    let domain_bytes = match read_user_bytes(root_ppn, domain_ptr, domain_len) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let domain = match core::str::from_utf8(&domain_bytes) {
        Ok(s) => s,
        Err(_) => {
            log!("sys_storage_get: invalid domain utf8");
            return 0;
        }
    };

    let key_bytes = match read_user_bytes(root_ppn, key_ptr, key_len) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let key_hex = hex_encode(&key_bytes);
    let composite_key = format!("{}:{}", domain, key_hex);

    let value = unsafe { STATE.get_mut() }
        .as_ref()
        .and_then(|state| state.get_account(&address))
        .and_then(|account| account.storage.get(&composite_key).cloned());

    let value = match value {
        Some(value) => value,
        None => return 0,
    };

    let total_len = match value.len().checked_add(4) {
        Some(len) => len,
        None => {
            log!("sys_storage_get: value too large");
            return 0;
        }
    };
    if total_len > u32::MAX as usize {
        log!("sys_storage_get: value exceeds u32 size");
        return 0;
    }

    let addr = sys_alloc([total_len as u32, 8, 0, 0, 0, 0]);
    if addr == 0 {
        log!("sys_storage_get: allocation failed");
        return 0;
    }

    let mut buf = Vec::with_capacity(total_len);
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(&value);

    if !mmu::copy(root_ppn, addr, &buf) {
        logf!("sys_storage_get: failed to write to 0x%x", addr);
        return 0;
    }

    addr
}

pub(crate) fn sys_storage_set(args: [u32; 6]) -> u32 {
    let address_ptr = args[0];
    let domain_ptr = args[1];
    let key_ptr = args[2];
    let lens_packed = args[3] as usize;
    let val_ptr = args[4];
    let val_len = args[5] as usize;

    let domain_len = lens_packed & 0xffff;
    let key_len = lens_packed >> 16;

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 0,
    };

    let address_bytes = match read_user_bytes(root_ppn, address_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 0,
    };
    if address_bytes.len() != ADDRESS_LEN {
        log!("sys_storage_set: invalid address length");
        return 0;
    }
    let mut addr_buf = [0u8; ADDRESS_LEN];
    addr_buf.copy_from_slice(&address_bytes);
    let address = Address(addr_buf);
    if !caller_address_matches(root_ppn, &address) {
        log!("sys_storage_set: address mismatch with caller");
        return 0;
    }

    let domain_bytes = match read_user_bytes(root_ppn, domain_ptr, domain_len) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let domain = match core::str::from_utf8(&domain_bytes) {
        Ok(s) => s,
        Err(_) => {
            log!("sys_storage_set: invalid domain utf8");
            return 0;
        }
    };

    let key_bytes = match read_user_bytes(root_ppn, key_ptr, key_len) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let key_hex = hex_encode(&key_bytes);

    let value = match read_user_bytes(root_ppn, val_ptr, val_len) {
        Some(bytes) => bytes,
        None => return 0,
    };

    let composite_key = format!("{}:{}", domain, key_hex);
    let state = unsafe { STATE.get_mut().get_or_insert_with(State::new) };
    state
        .get_account_mut(&address)
        .storage
        .insert(composite_key, value);
    0
}

pub(crate) fn current_task_root_ppn() -> Option<u32> {
    let current = unsafe { *CURRENT_TASK.get_mut() };
    let tasks = unsafe { TASKS.get_mut() };
    match tasks.get(current) {
        Some(task) => Some(task.addr_space.root_ppn),
        None => {
            logf!("sys_storage: no current task for slot %d", current as u32);
            None
        }
    }
}

pub(crate) fn read_user_bytes(root_ppn: u32, ptr: u32, len: usize) -> Option<Vec<u8>> {
    if len == 0 {
        return Some(Vec::new());
    }
    let mut buf = vec![0u8; len];
    let mut remaining = len;
    let mut dst_off = 0usize;
    let mut va = ptr;
    while remaining > 0 {
        let phys = match mmu::translate(root_ppn, va) {
            Some(p) => p,
            None => {
                logf!("sys_storage: invalid memory access 0x%x", va);
                return None;
            }
        };
        let page_off = (va as usize) & (SV32_PAGE_SIZE - 1);
        let to_copy = cmp::min(remaining, SV32_PAGE_SIZE - page_off);
        let src = SV32_DIRECT_MAP_BASE as usize + phys;
        unsafe {
            core::ptr::copy_nonoverlapping(
                src as *const u8,
                buf.as_mut_ptr().add(dst_off),
                to_copy,
            );
        }
        remaining -= to_copy;
        dst_off += to_copy;
        va = va.wrapping_add(to_copy as u32);
    }
    Some(buf)
}

pub(crate) fn caller_address_matches(root_ppn: u32, address: &Address) -> bool {
    let current = unsafe { *CURRENT_TASK.get_mut() };
    if current == KERNEL_TASK_SLOT {
        return true;
    }
    let caller_bytes = match read_user_bytes(root_ppn, TO_PTR_ADDR, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return false,
    };
    if caller_bytes.len() != ADDRESS_LEN {
        return false;
    }
    let mut caller_buf = [0u8; ADDRESS_LEN];
    caller_buf.copy_from_slice(&caller_bytes);
    Address(caller_buf) == *address
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0x0f) as usize]);
    }
    String::from_utf8(out).unwrap_or_default()
}
