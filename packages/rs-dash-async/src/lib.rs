//! Async-sync bridging utilities for Dash Platform.
//!
//! Provides [`block_on`] -- a function that bridges async futures into sync code,
//! handling multiple tokio runtime flavors (no runtime, current-thread, multi-thread).
//!
//! Also provides `ThreadRegistry` — a shared lifecycle engine for background
//! OS-thread workers (start, cancel, bounded quiesce + join, orphan reap).

mod block_on;
#[cfg(not(target_arch = "wasm32"))]
mod registry;

pub use block_on::{block_on, AsyncError};
#[cfg(not(target_arch = "wasm32"))]
pub use registry::{
    ClearingGuard, RegistryKey, ShutdownReport, ThreadRegistry, WorkerConfig, WorkerStatus,
    DEFAULT_JOIN_BUDGET, DEFAULT_REAP_BACKSTOP,
};
