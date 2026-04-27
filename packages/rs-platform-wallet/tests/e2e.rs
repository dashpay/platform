//! End-to-end integration tests for `rs-platform-wallet`.
//!
//! Single test binary that wires up a shared `E2eContext` (bank
//! wallet, SDK, SPV runtime, panic-safe registry) once per process
//! and reuses it across every test case under `cases/`. Submodules
//! under `framework/` provide the harness pieces; `cases/` hosts the
//! actual `#[tokio_shared_rt::test(shared)]` entries.
//!
//! The full design lives in
//! `/home/ubuntu/.claude/plans/ok-now-we-ll-get-prancy-biscuit.md`
//! (Module Layout section).
//!
//! # Wave 2 status
//!
//! Skeleton only — module surfaces are stubbed with `todo!` /
//! `FrameworkError::NotImplemented`. Wave 3 fills in the bank,
//! signer, registry, cleanup, SDK, SPV, and ContextProvider bodies;
//! Wave 4 wires `framework::setup` and adds the first test case.
//!
//! `dead_code` / `unused_imports` are allowed crate-wide because
//! Wave 2's stubs intentionally don't reference one another yet —
//! Wave 3 turns those into hard wiring and the allow can be
//! tightened or removed at that point.

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
