use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

/// The record a contract keeps of a document one of its moderators deleted.
///
/// The document itself is gone; this is what is left to say that it was removed, not lost:
/// whose it was, who removed it, why and when, and a hash of what it was. It is stored under
/// the contract, by document type then document id, paid for by the moderator, and never
/// deleted. A document id commits to the nonce of its create transition and is produced at
/// most once, so the removed id can not be created again; what can bring the document back
/// is a moderator's restore, within `SystemLimits::contract_document_restore_window_ms` of
/// the removal, which marks the record restored and leaves it in place. A later deletion of
/// the restored document replaces the record with a fresh one.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractDocumentRemoval {
    /// The identity that owned the document when it was removed.
    pub document_owner_id: Identifier,
    /// The contract owner or moderator that removed it.
    pub moderator_id: Identifier,
    /// Why the moderator removed it. The code is a number the moderator sets and nothing
    /// checks; the text may be empty.
    pub reason: ContractModerationReason,
    /// The time of the block that removed it, in milliseconds.
    pub removed_at: TimestampMillis,
    /// A double SHA-256 of the document as it was serialized under its document type at the
    /// time of the removal (`Document::serialize`): what a restore must bring back, byte for
    /// byte.
    pub document_hash: [u8; 32],
    /// Set once a moderator restored the document: it is live again, at its id, as it was.
    pub restoration: Option<ContractDocumentRestoration>,
}

/// The mark a restore leaves on a removal record: who brought the document back, and when.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractDocumentRestoration {
    /// The contract owner or moderator that restored the document.
    pub moderator_id: Identifier,
    /// The time of the block that restored it, in milliseconds.
    pub restored_at: TimestampMillis,
}

impl ContractDocumentRemoval {
    /// Whether the document was restored: the record then describes a removal that was
    /// undone, and the document is live again.
    pub fn is_restored(&self) -> bool {
        self.restoration.is_some()
    }
}
