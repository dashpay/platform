//! Contract moderation: the banlist and the suspension list a moderated data contract keeps
//! under its own subtree (protocol version 14).
//!
//! ```text
//! [64] DataContractDocuments
//! └── <contract id>
//!     ├── [0] the contract (or its history subtree)
//!     ├── [1] documents
//!     └── [2] other
//!         ├── [64]  contract version item
//!         ├── [128] banlist     -> <identity id> -> Item(reason)               (when declared)
//!         └── [192] suspensions -> <identity id> -> Item(until ‖ reason)       (when declared)
//! ```
//!
//! and the records of the documents the contract's moderators deleted, under the same other
//! tree at key `16` (when the contract has a document type that says so):
//!
//! ```text
//!         [16] document removals
//!         └── <document type name>            (a type that sets `canBeDeletedByModerators`)
//!             └── <document id> -> Item(document owner id ‖ moderator id ‖ removed at ‖ reason)
//! ```
//!
//! `removed at` is a u64 of block time in milliseconds, big-endian: see
//! [`types::encode_document_removal`]. The moderator pays for the record and nothing ever
//! deletes it; the deleted document's own storage refund goes to nobody.
//!
//! `until` is a u64 of block time in milliseconds, big-endian. A reason is a tag byte (`0`: no
//! code, `1`: a code), the code as a big-endian u16 when tagged, and the text as UTF-8 up to
//! the end of the value: see [`types::encode_ban`] and [`types::encode_suspension`].
//!
//! An entry's storage flags name the moderator that wrote it, so the storage refund of its
//! deletion goes to that moderator whichever transition deletes it: an explicit unban or
//! unsuspend, a ban over a suspension, or the first document transition of the identity that
//! runs after its suspension lapsed.

#[cfg(feature = "server")]
mod add_contract_ban;
#[cfg(feature = "server")]
mod add_contract_document_removal;
#[cfg(feature = "server")]
mod add_contract_suspension;
#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch_contract_document_removals;
#[cfg(feature = "server")]
mod fetch_contract_moderation_entries;
#[cfg(feature = "server")]
mod fetch_contract_moderation_status;
#[cfg(feature = "server")]
mod insert_contract_document_removal_trees;
#[cfg(feature = "server")]
mod insert_contract_moderation_trees;
#[cfg(feature = "server")]
mod prove_contract_document_removals;
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
mod document_removal_tests;
#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
