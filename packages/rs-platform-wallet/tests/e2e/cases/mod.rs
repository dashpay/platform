//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod transfer;
