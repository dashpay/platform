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
//!         └── <document type name>            (a type that sets `moderatorAbilities.delete`)
//!             └── <document id> -> Item(document owner id ‖ moderator id ‖ removed at ‖
//!                                       document hash ‖ tag ‖ restoration? ‖ kept fields? ‖
//!                                       reason)
//! ```
//!
//! and the actions an elected contract's seated moderation team votes on, under key `24` (when
//! the contract has a document type that sets `moderatorAbilities.deleteSettled`), shaped like a
//! token group's actions:
//!
//! ```text
//!         [24] team actions
//!         ├── M (active)  -> <action id> -> I -> Item(action)
//!         │                              └─ S -> SumTree(<member id> -> SumItem(1))
//!         └── X (closed)  -> <action id> -> I, S  (moved here, without flags, when it ran)
//! ```
//!
//! See [`types::encode_contract_team_action`]. A member proposes an action, which is its own
//! approval, and the others approve it by its id, each approval its own sum item flagged with
//! the member that paid for it; nothing is ever rewritten. An approval that could meet the
//! action's rule reads the team and deletes the approvals of members who left, refunding each
//! to its member. The approval that meets the rule runs the action and moves it, with the
//! approvals that counted, to the closed actions, refunding each to its member. An action that
//! never gets there stays active.
//!
//! `removed at` is a u64 of block time in milliseconds, big-endian; the tag says whether a
//! restoration (bit 0) and kept fields (bit 1, `moderatorAbilities.deleteKeepsFields`)
//! follow: see [`types::encode_document_removal`]. The moderator pays for the record and nothing ever
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
mod add_contract_team_action_signature;
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
mod fetch_contract_team_action;
#[cfg(feature = "server")]
mod fetch_contract_team_action_signers;
#[cfg(feature = "server")]
mod fetch_contract_team_actions;
#[cfg(feature = "server")]
mod insert_contract_document_removal_trees;
#[cfg(feature = "server")]
mod insert_contract_moderation_trees;
#[cfg(feature = "server")]
mod insert_contract_team_action_trees;
#[cfg(feature = "server")]
mod prove_contract_document_removals;
#[cfg(feature = "server")]
mod prove_contract_moderation_action_counts;
#[cfg(feature = "server")]
mod prove_contract_moderation_entries;
#[cfg(feature = "server")]
mod prove_contract_moderation_status;
#[cfg(feature = "server")]
mod prove_contract_team_action_signers;
#[cfg(feature = "server")]
mod prove_contract_team_actions;
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
#[cfg(feature = "server")]
mod team_actions;
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
mod team_action_tests;
#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
