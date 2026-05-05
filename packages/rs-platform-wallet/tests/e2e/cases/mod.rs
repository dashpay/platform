//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_005_token_mint;
pub mod tk_005b_token_mint_to_other;
pub mod tk_006_token_burn;
pub mod transfer;
