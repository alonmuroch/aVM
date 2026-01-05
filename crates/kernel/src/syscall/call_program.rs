use clibc::logf;
use types::{ADDRESS_LEN, Address};

use crate::global::{CURRENT_TASK, MAX_INPUT_LEN, TASKS};
use crate::syscall::SyscallContext;
use crate::syscall::storage::{caller_address_matches, current_task_root_ppn, read_user_bytes};
use crate::task::prep_program_task;
use crate::user_program::with_program_image;

const REG_COUNT: usize = 32;
const REG_PC: usize = 32;

pub(crate) fn sys_call_program(args: [u32; 6], ctx: &mut SyscallContext<'_>) -> u32 {
    let to_ptr = args[0];
    let from_ptr = args[1];
    let input_ptr = args[2];
    let input_len = args[3] as usize;

    if input_len > MAX_INPUT_LEN {
        logf!("sys_call_program: input too large");
        return 0;
    }

    let root_ppn = match current_task_root_ppn() {
        Some(root) => root,
        None => return 0,
    };

    let to_bytes = match read_user_bytes(root_ppn, to_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let from_bytes = match read_user_bytes(root_ppn, from_ptr, ADDRESS_LEN) {
        Some(bytes) => bytes,
        None => return 0,
    };
    let input = match read_user_bytes(root_ppn, input_ptr, input_len) {
        Some(bytes) => bytes,
        None => return 0,
    };

    if to_bytes.len() != ADDRESS_LEN || from_bytes.len() != ADDRESS_LEN {
        logf!("sys_call_program: invalid address length");
        return 0;
    }

    let mut to_buf = [0u8; ADDRESS_LEN];
    let mut from_buf = [0u8; ADDRESS_LEN];
    to_buf.copy_from_slice(&to_bytes);
    from_buf.copy_from_slice(&from_bytes);
    let to = Address(to_buf);
    let from = Address(from_buf);

    if !caller_address_matches(root_ppn, &from) {
        logf!("sys_call_program: caller address mismatch");
        return 0;
    }

    let task = match with_program_image(&to, |image| {
        prep_program_task(&to, &from, image.code, &input, image.entry_off)
    }) {
        Some(task) => task,
        None => return 0,
    };

    let task_idx = unsafe {
        let tasks = TASKS.get_mut();
        if tasks.push(task).is_err() {
            logf!("sys_call_program: task list full");
            return 0;
        }
        tasks.len().saturating_sub(1)
    };

    let caller_idx = unsafe { *CURRENT_TASK.get_mut() };
    unsafe {
        let tasks = TASKS.get_mut();
        let caller_task = match tasks.get_mut(caller_idx) {
            Some(task) => task,
            None => {
                logf!(
                    "sys_call_program: missing caller task %d",
                    caller_idx as u32
                );
                return 0;
            }
        };
        for (idx, value) in ctx.regs.iter().take(REG_COUNT).enumerate() {
            caller_task.tf.regs[idx] = *value;
        }
        caller_task.tf.pc = ctx.regs[REG_PC].wrapping_add(4);
    }

    crate::run_task(task_idx);
    0
}
