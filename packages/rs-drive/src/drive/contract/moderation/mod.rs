//! Contract moderation: the banlist and the suspension list a moderated data contract keeps
//! under its own subtree (protocol version 14).
//!
//! ```text
//! [64] DataContractDocuments
//! └── <contract id>
//!     ├── [0] the contract (or its history subtree)
//!     ├── [1] documents
//!     ├── [2] contract version item
//!     ├── [3] banlist       -> <identity id> -> Item([])                      (when declared)
//!     └── [4] suspensions   -> <identity id> -> Item(until, u64 BE millis)     (when declared)
//! ```
//!
//! An entry's storage flags name the moderator that wrote it, so the storage refund of its
//! deletion goes to that moderator whichever transition deletes it: an explicit unban or
//! unsuspend, a ban over a suspension, or the first document transition of the identity that
//! runs after its suspension lapsed.

#[cfg(feature = "server")]
mod add_contract_ban;
#[cfg(feature = "server")]
mod add_contract_suspension;
#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch_contract_moderation_entries;
#[cfg(feature = "server")]
mod fetch_contract_moderation_status;
#[cfg(feature = "server")]
mod insert_contract_moderation_trees;
#[cfg(feature = "server")]
mod prove_contract_moderation_entries;
#[cfg(feature = "server")]
mod prove_contract_moderation_status;
mod queries;
#[cfg(feature = "server")]
mod remove_contract_ban;
#[cfg(feature = "server")]
mod remove_contract_suspension;
/// Query and result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests;

/// The stored size of a suspension entry: `until` as a u64.
pub const CONTRACT_SUSPENSION_VALUE_SIZE: u32 = 8;
