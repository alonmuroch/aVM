use clibc::logf;

use crate::global::{CURRENT_TX, RECEIPTS};
use crate::syscall::storage::{current_task_root_ppn, read_user_bytes};

pub(crate) fn sys_fire_event(args: [u32; 6]) -> u32 {
    let ptr = args[0];
    let len = args[1] as usize;

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 0,
    };

    let event_bytes = match read_user_bytes(root_ppn, ptr, len) {
        Some(bytes) => bytes,
        None => return 0,
    };

    let current_idx = unsafe { *CURRENT_TX.get_mut() };
    let receipts = unsafe { RECEIPTS.get_mut() };
    match receipts
        .as_mut()
        .and_then(|receipts| receipts.get_mut(current_idx))
    {
        Some(receipt) => {
            receipt.add_event(event_bytes);
        }
        None => {
            logf!(
                "sys_fire_event: missing receipt for tx %d",
                current_idx as u32
            );
        }
    }
    0
}
