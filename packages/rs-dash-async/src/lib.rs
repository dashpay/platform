//! Async-sync bridging utilities for Dash Platform.
//!
//! Provides [`block_on`] -- a function that bridges async futures into sync code,
//! handling multiple tokio runtime flavors (no runtime, current-thread, multi-thread).
//!
//! Also provides [`AtomicFlagGuard`] — a RAII guard for panic-safe `AtomicBool` flag resets,
//! and [`ThreadRegistry`] — a shared lifecycle engine for background OS-thread / tokio-task
//! workers (start, cancel, weight-ordered quiesce + join, orphan reap).

mod atomic;
mod block_on;
#[cfg(not(target_arch = "wasm32"))]
mod registry;

pub use atomic::{AtomicFlagGuard, RefcountedFlagGuard};
pub use block_on::{block_on, AsyncError};
#[cfg(not(target_arch = "wasm32"))]
pub use registry::{
    ClearingGuard, DrainHook, RegistryKey, ShutdownReport, ShutdownWeight, ThreadRegistry,
    WorkerConfig, WorkerStatus, DEFAULT_JOIN_BUDGET, DEFAULT_REAP_BACKSTOP,
};
