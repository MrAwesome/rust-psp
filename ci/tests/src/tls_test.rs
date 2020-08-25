use core::ptr::null_mut;
use core::ffi::c_void;
use psp::sys::{sceKernelGetTlsAddr, sceKernelCreateTlspl, SceUid};
use psp::test_runner::TestRunner;

// TODO: add error enums, check if is error
// TODO: in error enum funcs, have a generic catch-all for 0x800xxxxx?
fn TODO_quick_check_is_err(ptr: *mut c_void) -> bool {
    (ptr as usize & 0xfff00000) == 0x80000000
}

pub fn test_main(test_runner: &mut TestRunner) {
    let options = null_mut();
    let part = 2;
    unsafe {
        let pl =
            sceKernelCreateTlspl(
                b"muh_pool\0" as _,
                part,
                0,
                16,
                32,
                options
            );

        test_runner.dbg("pool_address", pl);
        test_runner.check_true("pool_created_successfully", pl.0 > 0);

        let valid_address = sceKernelGetTlsAddr(pl);
        test_runner.dbg("valid_addr", valid_address);
        test_runner.check_true("pool_address_non_null", !valid_address.is_null());
        test_runner.check_true("pool_address_valid", !TODO_quick_check_is_err(valid_address));

        let invalid_address = sceKernelGetTlsAddr(SceUid(12321321));
        test_runner.dbg("invalid_addr", invalid_address);
        test_runner.check_true("invalid_uid_gives_null_addr", invalid_address.is_null());
    }
}
