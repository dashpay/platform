//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_007_token_freeze;
pub mod tk_008_token_unfreeze;
pub mod tk_009_token_destroy_frozen;
pub mod transfer;
