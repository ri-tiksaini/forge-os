#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener;
use std::process;

/// Accepts FD 3 from systemd socket activation (`LISTEN_FDS=1`), if present.
pub fn take_systemd_socket() -> Option<UnixListener> {
    let fds: usize = env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if fds < 1 {
        return None;
    }
    if let Ok(pid_str) = env::var("LISTEN_PID") {
        if pid_str.parse::<u32>().ok()? != process::id() {
            return None;
        }
    }
    // SAFETY: systemd guarantees FD 3 is an open listening Unix socket when LISTEN_FDS >= 1
    // and LISTEN_PID matches our PID.
    let listener = unsafe { UnixListener::from_raw_fd(3) };
    Some(listener)
}
