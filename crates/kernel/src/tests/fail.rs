use crate::{results, utils};

pub fn fail(code: u32) -> ! {
    unsafe {
        results::write_results(results::TestResults {
            status: 1,
            detail: code,
        })
    };
    utils::halt();
}
