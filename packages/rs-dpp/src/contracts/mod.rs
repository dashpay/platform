//! This module contains the contract's related data, which in JS version were imported
//! directly from contract repositories.

pub mod dashpay_contract;
pub mod dpns_contract;
pub mod feature_flags_contract;
pub mod masternode_reward_shares_contract;

/// Contains the system IDs.
//? The structure contains the heap-allocated String, which is not memory efficient.
//? Depending on contract-implementation we should consider changing the types to &str
pub struct SystemIDs {
    pub contract_id: String,
    pub owner_id: String,
}
