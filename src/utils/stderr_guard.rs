#[cfg(windows)]
mod imp {
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;

    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> isize;
        fn SetStdHandle(nStdHandle: u32, hHandle: isize) -> i32;
    }

    const STD_ERROR_HANDLE: u32 = 0xFFFFFFF4;

    pub struct StderrGuard {
        old_handle: isize,
        _sink: File,
    }

    impl StderrGuard {
        pub fn suppress() -> Self {
            let sink = File::create("NUL").expect("failed to open NUL");
            let sink_handle = sink.as_raw_handle() as isize;
            let old = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
            unsafe { SetStdHandle(STD_ERROR_HANDLE, sink_handle) };
            StderrGuard { old_handle: old, _sink: sink }
        }
    }

    impl Drop for StderrGuard {
        fn drop(&mut self) {
            unsafe { SetStdHandle(STD_ERROR_HANDLE, self.old_handle) };
            let _ = std::io::stderr().flush();
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct StderrGuard;
    impl StderrGuard {
        pub fn suppress() -> Self {
            StderrGuard
        }
    }
}

pub use imp::StderrGuard;
