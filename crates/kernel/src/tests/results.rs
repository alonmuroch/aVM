#[repr(C)]
#[derive(Clone, Copy)]
pub struct TestResults {
    pub status: u32,
    pub detail: u32,
}

pub const TEST_RESULTS_ADDR: u32 = 0x0003_f000;

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
    let ptr = TEST_RESULTS_ADDR as *mut TestResults;
    ptr.write_volatile(results);
}
