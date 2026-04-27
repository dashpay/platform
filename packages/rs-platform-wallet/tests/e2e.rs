//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary that wires up a shared `E2eContext` (bank
//! wallet, SDK, panic-safe registry) once per process and reuses
//! it across every test case under `cases/`. Submodules under
//! `framework/` provide the harness pieces; `cases/` hosts the
//! actual `#[tokio_shared_rt::test(shared)]` entries.
//!
//! The full design lives in
//! `/home/ubuntu/.claude/plans/ok-now-we-ll-get-prancy-biscuit.md`
//! (Module Layout section).
//!
//! `dead_code` / `unused_imports` remain allowed crate-wide for
//! this integration-test crate's module layout and helper surfaces
//! (e.g. the deferred SPV path retained for Task #15 re-enable);
//! the allow can be tightened as the e2e suite evolves.

#![allow(dead_code, unused_imports)]

// `tests/e2e.rs` is the integration-test crate root, so by default
// `mod cases;` would resolve to `tests/cases/...` — not what we
// want. Explicit `#[path = ...]` keeps the on-disk layout grouped
// under `tests/e2e/` (mirroring the plan's Module Layout) while
// still letting nested submodules use the default resolution rules
// relative to each parent file.
#[path = "e2e/cases/mod.rs"]
mod cases;
#[path = "e2e/framework/mod.rs"]
mod framework;
