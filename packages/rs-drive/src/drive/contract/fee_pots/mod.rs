//! The two fee pots a data contract's document action fees accumulate in, and the last claim of
//! each pot: its epoch, its block time and who claimed it (protocol version 14).
//!
//! ```text
//! [40] PreFundedSpecializedBalances (sum tree)
//! ├── [64]  owner fee pots      (sum tree) -> <contract id> -> SumItem(credits)
//! ├── [128] voting balances
//! └── [192] moderators fee pots (sum tree) -> <contract id> -> SumItem(credits)
//!
//! [64] DataContractDocuments -> <contract id> -> [2] the contract's other tree
//!     ├── [32] last claim of the owner pot       Item(epoch u16 BE | time u64 BE | claimant id)
//!     └── [96] last claim of the moderators pot  Item(epoch u16 BE | time u64 BE | claimant id)
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
mod set_contract_last_fee_claim;
/// Result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
