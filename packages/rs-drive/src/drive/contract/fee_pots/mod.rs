//! The two fee pots a data contract's document action fees accumulate in, and the epoch each
//! pot was last claimed in (protocol version 14).
//!
//! ```text
//! [40] PreFundedSpecializedBalances (sum tree)
//! ├── [64]  owner fee pots      (sum tree) -> <contract id> -> SumItem(credits)
//! ├── [128] voting balances
//! └── [192] moderators fee pots (sum tree) -> <contract id> -> SumItem(credits)
//!
//! [64] DataContractDocuments -> <contract id> -> [2] the contract's other tree
//!     ├── [32] epoch the owner pot was last claimed in       Item(u16 BE)   (after a claim)
//!     └── [96] epoch the moderators pot was last claimed in  Item(u16 BE)   (after a claim)
//! ```
//!
//! The pots sit under a root sum tree on purpose: the credits they hold left an identity's
//! balance and have not reached another one yet, so they must stay inside the sum of all
//! credits that `calculate_total_credits_balance` checks every block.

#[cfg(feature = "server")]
mod add_to_contract_fee_pot;
#[cfg(feature = "server")]
mod deduct_from_contract_fee_pot;
#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch_action_fee_multiplier;
#[cfg(feature = "server")]
mod fetch_contract_fee_pot;
#[cfg(feature = "server")]
mod insert_contract_fee_pot_trees;
#[cfg(feature = "server")]
mod prove_contract_fee_pots;
mod queries;
#[cfg(feature = "server")]
mod set_contract_last_fee_claim_epoch;
/// Result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests;

/// The stored size of a last claim epoch item: the epoch index as a u16.
pub const CONTRACT_LAST_FEE_CLAIM_EPOCH_VALUE_SIZE: u32 = 2;
