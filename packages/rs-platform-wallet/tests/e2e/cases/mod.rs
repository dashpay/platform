//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_013_token_claim_pre_programmed;
pub mod tk_014_token_group_action;
pub mod transfer;
