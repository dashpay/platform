//! Shared tokio runtime for blocking on async wallet operations.

/// Get the shared tokio runtime.
///
/// All async FFI functions use this runtime via `runtime().block_on(...)`.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi")
    });
    &RT
}
