//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary with a process-shared `E2eContext` (bank
//! wallet, SDK, panic-safe registry). `framework/` provides the
//! harness; `cases/` hosts `#[tokio_shared_rt::test(shared)]` entries.

#![allow(dead_code, unused_imports)]
#![allow(clippy::result_large_err)]

// `tests/e2e.rs` is the integration-test crate root; explicit
// `#[path]` keeps the on-disk layout grouped under `tests/e2e/`.
#[path = "e2e/cases/mod.rs"]
mod cases;
#[path = "e2e/framework/mod.rs"]
mod framework;
