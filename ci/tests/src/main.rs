#![no_std]
#![no_main]

#[cfg(not(feature = "stub-only"))] extern crate alloc;

use psp::test_runner::TestRunner;

mod bmp_screenshot_test;
mod math_test;
mod vram_test;
mod tls_test;

psp::module!("ci_tests", 1, 1);

fn psp_main() {
    let tests = &[
        // NOTE: should be first until screenshot test clears screen
        bmp_screenshot_test::test_main,
        vram_test::test_main,
        math_test::test_main,
        tls_test::test_main,
    ];

    let mut runner = TestRunner::new_file_runner();
    runner.start_run();

    for test in tests {
        runner.run(test);
    }

    runner.finish_run();
}
