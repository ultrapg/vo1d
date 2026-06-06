#[cfg(windows)]
mod imp {
    use std::io::Write;

    extern "C" {
        fn _dup(fd: i32) -> i32;
        fn _dup2(fd1: i32, fd2: i32) -> i32;
        fn _close(fd: i32) -> i32;
        fn _open(path: *const i8, flags: i32) -> i32;
    }

    const _O_WRONLY: i32 = 0x0001;
    const _O_BINARY: i32 = 0x8000;
    const STDERR_FILENO: i32 = 2;

    pub struct StderrGuard {
        saved_fd: i32,
    }

    impl StderrGuard {
        pub fn suppress() -> Self {
            unsafe {
                let saved_fd = _dup(STDERR_FILENO);
                if saved_fd < 0 {
                    return StderrGuard { saved_fd: -1 };
                }
                let nul_fd = _open("NUL\0".as_ptr() as *const i8, _O_WRONLY | _O_BINARY);
                if nul_fd < 0 {
                    _close(saved_fd);
                    return StderrGuard { saved_fd: -1 };
                }
                _dup2(nul_fd, STDERR_FILENO);
                _close(nul_fd);
                StderrGuard { saved_fd }
            }
        }
    }

    impl Drop for StderrGuard {
        fn drop(&mut self) {
            if self.saved_fd >= 0 {
                unsafe {
                    _dup2(self.saved_fd, STDERR_FILENO);
                    _close(self.saved_fd);
                }
            }
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
