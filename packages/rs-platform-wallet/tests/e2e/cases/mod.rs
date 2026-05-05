//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_001_token_transfer;
pub mod tk_001b_token_transfer_zero;
pub mod tk_001c_token_transfer_after_reissue;
pub mod tk_002_token_claim_perpetual;
pub mod transfer;
