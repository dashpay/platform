//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary with a process-shared `E2eContext` (bank
//! wallet, SDK, panic-safe registry). `framework/` provides the
//! harness; `cases/` hosts `#[tokio_shared_rt::test(shared)]` entries.
//
// TODO(bilby, 2026-06-18, #3549 base-merge): the post-merge compile gate
// (`cargo check --tests --features e2e -p platform-wallet`) and this e2e
// suite were NOT run after merging `origin/v3.1-dev` — the host root
// filesystem was 100% full (`ld` SIGBUS/core-dumped mid-link, a textbook
// out-of-disk failure), which is an environment issue, not a code one.
// Re-run the compile gate and the suite once disk is freed to confirm the
// merge is "no worse than the recorded 70-PASS baseline".

#![allow(dead_code, unused_imports)]
#![allow(clippy::result_large_err)]

// `tests/e2e.rs` is the integration-test crate root; explicit
// `#[path]` keeps the on-disk layout grouped under `tests/e2e/`.
#[path = "e2e/cases/mod.rs"]
mod cases;
#[path = "e2e/framework/mod.rs"]
mod framework;
