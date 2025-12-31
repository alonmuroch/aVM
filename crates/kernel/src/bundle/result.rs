use kernel::Config;
use kernel::global::{CURRENT_TX, LAST_COMPLETED_TASK, RECEIPTS, TASKS};
use clibc::{log, logf};
use types::{KernelResult, TransactionReceipt};

pub(crate) fn update_receipt_from_task() {
    let (tx_idx, task_idx) = unsafe {
        let tx_idx = *CURRENT_TX.get_mut();
        let task_idx = (*LAST_COMPLETED_TASK.get_mut()).take();
        (tx_idx, task_idx)
    };
    let task_idx = match task_idx {
        Some(idx) => idx,
        None => {
            log!("resume_bundle: no completed task to update receipt");
            return;
        }
    };
    let result = unsafe {
        let tasks = TASKS.get_mut();
        tasks
            .get(task_idx)
            .and_then(|task| task.last_result)
    };
    let result = match result {
        Some(res) => res,
        None => {
            log!("resume_bundle: completed task missing result");
            return;
        }
    };
    unsafe {
        if let Some(receipts) = RECEIPTS.get_mut().as_mut() {
            if let Some(receipt) = receipts.get_mut(tx_idx) {
                receipt.result = result;
            } else {
                logf!("resume_bundle: invalid receipt index %d", tx_idx as u32);
            }
        } else {
            log!("resume_bundle: receipts missing");
        }
    }
}

pub(crate) fn write_kernel_result() {
    let encoded = unsafe {
        RECEIPTS
            .get_mut()
            .as_ref()
            .map(|receipts| TransactionReceipt::encode_list(receipts))
    };
    let encoded = match encoded {
        Some(data) => data,
        None => {
            log!("kernel_result: receipts missing");
            return;
        }
    };
    let len = encoded.len() as u32;
    // The bootloader maps the kernel window at VA 0, so this VA is also a
    // physical address in the current setup.
    let ptr = encoded.as_ptr() as u32;
    core::mem::forget(encoded);
    let header = KernelResult {
        receipts_ptr: ptr,
        receipts_len: len,
    };
    unsafe {
        core::ptr::write_volatile(Config::KERNEL_RESULT_ADDR as *mut KernelResult, header);
    }
    logf!(
        "kernel_result: receipts_ptr=0x%x receipts_len=%d",
        ptr,
        len
    );
}
