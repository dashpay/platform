//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].

pub mod id_001_register_identity_from_addresses;
pub mod id_002_top_up_identity;
pub mod id_003_identity_to_identity_transfer;
pub mod id_005_identity_to_addresses_transfer;
pub mod id_sweep_recovers_identity_credits;
pub mod transfer;
