use std::io::{Write, stdout};

/// RAII guard for raw terminal mode that preserves standard output processing (OPOST / ONLCR).
pub struct RawModeGuard;

impl RawModeGuard {
    pub fn enter() -> Option<Self> {
        if crossterm::terminal::enable_raw_mode().is_err() {
            return None;
        }

        #[cfg(unix)]
        unsafe {
            if let Ok(file) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
            {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                let mut termios = std::mem::MaybeUninit::uninit();
                if libc::tcgetattr(fd, termios.as_mut_ptr()) == 0 {
                    let mut termios = termios.assume_init();
                    termios.c_oflag |= libc::OPOST | libc::ONLCR;
                    libc::tcsetattr(fd, libc::TCSANOW, &termios);
                }
            }
            for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
                let mut termios = std::mem::MaybeUninit::uninit();
                if libc::tcgetattr(fd, termios.as_mut_ptr()) == 0 {
                    let mut termios = termios.assume_init();
                    termios.c_oflag |= libc::OPOST | libc::ONLCR;
                    libc::tcsetattr(fd, libc::TCSANOW, &termios);
                }
            }
        }

        Some(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Prints a line to standard output with explicit carriage return and newline (\r\n) to guarantee column 0 alignment.
pub fn term_println(text: &str) {
    let mut out = stdout().lock();
    if text.is_empty() {
        let _ = out.write_all(b"\r\n");
        let _ = out.flush();
        return;
    }

    for sub_line in text.split(['\r', '\n']) {
        if !sub_line.is_empty() {
            let _ = out.write_all(sub_line.as_bytes());
            let _ = out.write_all(b"\r\n");
        }
    }
    let _ = out.flush();
}

/// Prints text to standard output without an automatic newline, flushing immediately.
pub fn term_print(text: &str) {
    let mut out = stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}
