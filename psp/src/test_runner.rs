use crate::sys::{self, SceUid};
use core::ffi::c_void;

pub const OUTPUT_FILENAME: &str = "psp_output_file.log";
pub const OUTPUT_FIFO: &str = "psp_output_pipe.fifo";

pub const STARTING_TOKEN: &str = "STARTING_TESTS";
pub const SUCCESS_TOKEN: &str = "FINAL_SUCCESS";
pub const FAILURE_TOKEN: &str = "FINAL_FAILURE";

use alloc::format;
use alloc::vec::Vec;
use core::fmt::Arguments;

pub struct TestRunner {
    _mode: TestRunnerMode,
    fd: SceUid,
    failure: bool,
}

enum TestRunnerMode {
    FIFO,
    FILE,
}

impl TestRunner {
    pub fn new_fifo_runner() -> Self {
        let fd = get_test_output_pipe();
        Self {
            fd,
            _mode: TestRunnerMode::FIFO,
            failure: false,
        }
    }

    pub fn new_file_runner() -> Self {
        let fd = get_test_output_file();
        Self {
            fd,
            _mode: TestRunnerMode::FILE,
            failure: false,
        }
    }

    pub fn start(&self) {
        self.write_args(format_args!("\n\n{}\n", STARTING_TOKEN));
    }

    pub fn finish(self) {
        if self.failure {
            self.write_args(format_args!("{}\n", FAILURE_TOKEN));
        } else {
            self.write_args(format_args!("{}\n", SUCCESS_TOKEN));
        }
        self.quit();
    }

    pub fn check_fns_do_not_panic(&self, tests: &[(&str, &dyn Fn())]) {
        for (testcase_name, f) in tests {
            f();
            self.pass(testcase_name, "");
        }
    }

    pub fn check<T>(&mut self, testcase_name: &str, l: T, r: T)
    where
        T: core::fmt::Debug + PartialEq,
    {
        if l == r {
            self.pass(testcase_name, &format!("{:?} == {:?}", l, r));
        } else {
            self.fail(testcase_name, &format!("{:?} != {:?}", l, r));
        }

    }
    pub fn check_list<T>(&mut self, val_pairs: &[(&str, T, T)])
    where
        T: core::fmt::Debug + PartialEq,
    {
        for (testcase_name, l, r) in val_pairs {
            self.check(testcase_name, l, r)
        }
    }

    pub fn _check_return_values<T>(&mut self, val_pairs: &[(&str, &dyn Fn() -> T, T)])
    where
        T: core::fmt::Debug + PartialEq + Eq + Clone,
    {
        self.check_list(
            &val_pairs
                .iter()
                .map(|(testcase_name, f, v)| (*testcase_name, f(), v.clone()))
                .collect::<Vec<(&str, T, T)>>(),
        )
    }

    pub fn pass(&self, testcase_name: &str, msg: &str) {
        self.write_args(format_args!("[PASS]: ({}) {}\n", testcase_name, msg));
    }

    pub fn fail(&mut self, testcase_name: &str, msg: &str) {
        self.failure = true;
        self.write_args(format_args!("[FAIL]: ({}) {}\n", testcase_name, msg));
    }

    pub fn write_args(&self, args: Arguments) {
        write_to_psp_output_fd(self.fd, &format!("{}", args));
    }

    fn quit(self) {
        close_psp_file_and_quit_game(self.fd);
    }
}

fn get_test_output_pipe() -> SceUid {
    unsafe {
        let fd = sys::sceIoOpen(
            psp_filename(OUTPUT_FIFO),
            sys::IoOpenFlags::APPEND | sys::IoOpenFlags::WR_ONLY,
            0o777,
        );
        if fd.0 < 0 {
            panic!(
                "Unable to open pipe \"{}\" for output! \
                You must create it yourself with `mkfifo`."
            );
        }
        return fd;
    }
}

fn get_test_output_file() -> SceUid {
    unsafe {
        let fd = sys::sceIoOpen(
            psp_filename(OUTPUT_FILENAME),
            sys::IoOpenFlags::TRUNC | sys::IoOpenFlags::CREAT | sys::IoOpenFlags::RD_WR,
            0o777,
        );
        if fd.0 < 0 {
            panic!("Unable to open file \"{}\" for output!", OUTPUT_FILENAME);
        }
        return fd;
    }
}

fn psp_filename(filename: &str) -> *const u8 {
    format!("host0:/{}\0", filename).as_bytes().as_ptr()
}

fn write_to_psp_output_fd(fd: SceUid, msg: &str) {
    unsafe {
        sys::sceIoWrite(
            fd,
            msg.as_bytes().as_ptr() as *const u8 as *const c_void,
            msg.len(),
        );
    }
}

fn close_psp_file_and_quit_game(fd: SceUid) {
    unsafe {
        sys::sceIoClose(fd);
        sys::sceKernelExitGame();
    }
}