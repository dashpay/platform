use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

/// The record a contract keeps of a document one of its moderators deleted.
///
/// The document itself is gone; this is what is left to say that it was removed, not lost:
/// whose it was, who removed it, why and when. It is stored under the contract, by document
/// type then document id, paid for by the moderator, and never deleted. It is final: a
/// document id commits to the nonce of its create transition and is produced at most once, so
/// the removed id can not be created again and no second removal can replace the record.
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
}
