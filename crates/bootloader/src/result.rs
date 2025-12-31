use core::mem;

use types::kernel_result::KERNEL_RESULT_ADDR;
use types::{KernelResult, TransactionReceipt};
use state::State;
use vm::memory::{Memory as MmuRef, VirtualAddress};

pub struct KernelRunResult {
    pub receipts: Vec<TransactionReceipt>,
    pub state: Option<State>,
}

pub(crate) fn read_kernel_result(memory: &MmuRef) -> Option<KernelRunResult> {
    let header_size = mem::size_of::<KernelResult>() as u32;
    let header_end = KERNEL_RESULT_ADDR.checked_add(header_size)?;
    let header_slice = memory.mem_slice(
        VirtualAddress(KERNEL_RESULT_ADDR),
        VirtualAddress(header_end),
    )?;
    let header_bytes = header_slice.as_ref();
    if header_bytes.len() < header_size as usize {
        return None;
    }
    let receipts_ptr = u32::from_le_bytes(header_bytes[0..4].try_into().ok()?);
    let receipts_len = u32::from_le_bytes(header_bytes[4..8].try_into().ok()?);
    let state_ptr = u32::from_le_bytes(header_bytes[8..12].try_into().ok()?);
    let state_len = u32::from_le_bytes(header_bytes[12..16].try_into().ok()?);
    if receipts_ptr == 0 || receipts_len == 0 {
        return None;
    }
    let receipts_end = receipts_ptr.checked_add(receipts_len)?;
    let receipts_slice = memory.mem_slice(
        VirtualAddress(receipts_ptr),
        VirtualAddress(receipts_end),
    )?;
    let receipts = TransactionReceipt::decode_list(receipts_slice.as_ref())?;

    let state = if state_ptr != 0 && state_len != 0 {
        let state_end = state_ptr.checked_add(state_len)?;
        let state_slice = memory.mem_slice(
            VirtualAddress(state_ptr),
            VirtualAddress(state_end),
        )?;
        State::decode(state_slice.as_ref())
    } else {
        None
    };
    Some(KernelRunResult { receipts, state })
}
