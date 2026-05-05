//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_003_register_token_contract;
pub mod tk_004_token_transfer_round_trip;
pub mod transfer;
