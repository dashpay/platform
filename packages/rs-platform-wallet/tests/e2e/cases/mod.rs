//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod tk_010_token_pause_resume;
pub mod tk_011_token_price_purchase;
pub mod tk_012_token_update_config;
pub mod transfer;
