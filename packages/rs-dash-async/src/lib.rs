//! Async-sync bridging utilities for Dash Platform.
//!
//! Provides [`block_on`] -- a function that bridges async futures into sync code,
//! handling multiple tokio runtime flavors (no runtime, current-thread, multi-thread).
//!
//! Also provides [`AtomicFlagGuard`] — a RAII guard for panic-safe `AtomicBool` flag resets.

mod atomic;
mod block_on;

pub use atomic::AtomicFlagGuard;
pub use block_on::{block_on, AsyncError};
