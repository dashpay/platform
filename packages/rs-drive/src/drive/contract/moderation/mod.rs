//! Contract moderation: the banlist, the suspension list and the warning list a moderated
//! data contract keeps under its own subtree (protocol version 14).
//!
//! ```text
//! [64] DataContractDocuments
//! └── <contract id>
//!     ├── [0] the contract (or its history subtree)
//!     ├── [1] documents
//!     └── [2] other
//!         ├── [48]  moderation action counts -> <identity id> -> Item(count)  (elected contracts)
//!         ├── [64]  contract version item
//!         ├── [128] banlist     -> <identity id> -> Item(reason)               (when declared)
//!         ├── [192] suspensions -> <identity id> -> Item(until ‖ reason)       (when declared)
//!         └── [224] warnings    -> <identity id> -> Item((warned at ‖ len ‖ reason)+)  (when declared)
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
//! `until` is a u64 of block time in milliseconds, big-endian. A reason is a tag byte (bit 0: a
//! code, bit 1: documents, bit 2: a reason document), what the tag announces, and the text as
//! UTF-8 up to the end of the value: see [`types::encode_ban`] and
//! [`types::encode_suspension`]. A moderation action count is a u32, big-endian, without
//! storage flags: the seated team's member whose action writes it pays for it, and the settle
//! of the moderators pot that deletes it refunds nobody. A warning
//! list entry holds every warning the identity carries, oldest first, each its block time as a
//! u64 big-endian, the length of its reason as a u16 big-endian and the reason: see
//! [`types::encode_warnings`]. A warning bars nothing.
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
mod add_contract_warning;
#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch_contract_document_removals;
#[cfg(feature = "server")]
mod fetch_contract_moderation_action_counts;
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
mod remove_contract_moderation_action_counts;
#[cfg(feature = "server")]
mod remove_contract_suspension;
#[cfg(feature = "server")]
mod remove_contract_warnings;
#[cfg(feature = "server")]
mod set_contract_moderation_action_count;
/// Query and result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod action_count_tests;
#[cfg(test)]
#[cfg(feature = "server")]
mod document_removal_tests;
#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
