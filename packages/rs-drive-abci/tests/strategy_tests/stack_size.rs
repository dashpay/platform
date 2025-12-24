use std::{io::Write, thread};
use tracing::Level;

#[cfg(target_os = "linux")]
/// Logs the current thread's stack size at the specified log level.
///
/// If the log level is `None`, it will print to standard error.
///
/// Note: You need to enable the `tracing` crate in your project to use this function,
/// or use `-- --nocapture` to see the output in tests.
pub fn log_current_thread_stack_size(level: Option<Level>) {
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        let thread = libc::pthread_self();
        let label = thread::current()
            .name()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "unnamed_thread".to_string());
        if libc::pthread_getattr_np(thread, &mut attr) == 0 {
            let mut stack_addr: *mut libc::c_void = std::ptr::null_mut();
            let mut stack_size: usize = 0;
            if libc::pthread_attr_getstack(&attr, &mut stack_addr, &mut stack_size) == 0 {
                log_with_level(level, &label, Some(stack_size));
            } else {
                log_with_level(level, &label, None);
            }
            libc::pthread_attr_destroy(&mut attr);
        } else {
            log_with_level(level, &label, None);
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn log_current_thread_stack_size(_level: Level) {}

fn log_with_level(level: Option<Level>, label: &str, stack_size: Option<usize>) {
    // we must match here because macros cannot take variables for log levels
    match stack_size {
        Some(size) => match level {
            Some(Level::ERROR) => {
                tracing::error!(stack_size = size, label, "current thread stack size")
            }
            Some(Level::WARN) => {
                tracing::warn!(stack_size = size, label, "current thread stack size")
            }
            Some(Level::INFO) => {
                tracing::info!(stack_size = size, label, "current thread stack size")
            }
            Some(Level::DEBUG) => {
                tracing::debug!(stack_size = size, label, "current thread stack size")
            }
            Some(Level::TRACE) => {
                tracing::trace!(stack_size = size, label, "current thread stack size")
            }
            None => {
                eprintln!("{}: current thread stack size: {}", label, size);
                std::io::stderr().flush().expect("Failed to flush stderr");
            }
        },
        None => match level {
            Some(Level::ERROR) => {
                tracing::error!(label, "failed to determine current thread stack size")
            }
            Some(Level::WARN) => {
                tracing::warn!(label, "failed to determine current thread stack size")
            }
            Some(Level::INFO) => {
                tracing::info!(label, "failed to determine current thread stack size")
            }
            Some(Level::DEBUG) => {
                tracing::debug!(label, "failed to determine current thread stack size")
            }
            Some(Level::TRACE) => {
                tracing::trace!(label, "failed to determine current thread stack size")
            }
            None => {
                eprintln!("{}: failed to determine current thread stack size", label);
                std::io::stderr().flush().expect("Failed to flush stderr");
            }
        },
    }
}
