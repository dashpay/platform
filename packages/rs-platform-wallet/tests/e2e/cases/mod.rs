//! End-to-end test cases.
//!
//! Each submodule under `cases/` hosts one or more
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`]. The shared runtime
//! is what amortises the SPV / bank / SDK init across the whole
//! suite — see the harness module docs for the rationale.

pub mod transfer;
