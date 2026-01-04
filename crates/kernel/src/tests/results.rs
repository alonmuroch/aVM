#[repr(C)]
#[derive(Clone, Copy)]
pub struct TestResults {
    pub status: u32,
    pub detail: u32,
}

impl TestResults {
    pub const fn pass(detail: u32) -> Self {
        Self {
            status: 0,
            detail,
        }
    }

    pub const fn fail(detail: u32) -> Self {
        Self {
            status: 1,
            detail,
        }
    }
}

pub unsafe fn write_results(results: TestResults) {
    let ptr = kernel::global::KERNEL_RESULT_ADDR as *mut TestResults;
    unsafe {
        ptr.write_volatile(results);
    }
}
