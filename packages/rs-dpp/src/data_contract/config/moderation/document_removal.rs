use crate::data_contract::config::moderation::ContractModerationReason;
use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::{Document, DocumentV0Getters};
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The record a contract keeps of a document one of its moderators deleted.
///
/// The document itself is gone; this is what is left to say that it was removed, not lost:
/// whose it was, who removed it, why and when, a hash of what it was, and the values of the
/// fields its type keeps public (`moderatorAbilities.deleteKeepsFields`). It is stored under
/// the contract, by document type then document id, paid for by the moderator, and never
/// deleted. A document id commits to the nonce of its create transition and is produced at
/// most once, so the removed id can not be created again; what can bring the document back
/// is a moderator's restore, within `SystemLimits::contract_document_restore_window_ms` of
/// the removal, which marks the record restored and leaves it in place. A later deletion of
/// the restored document replaces the record with a fresh one.
#[derive(
    Debug, Clone, PartialEq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
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
    /// The values the record keeps of the document, by the property path its type lists
    /// under `moderatorAbilities.deleteKeepsFields`, copied from the document as it was
    /// removed (see [`kept_field_values`]): what of it stays public once it is gone. A path
    /// the document held no value at is left out. Empty on a type that lists none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kept_fields: BTreeMap<String, Value>,
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

/// The system properties a removal record can keep (`moderatorAbilities.deleteKeepsFields`):
/// the timestamps and block heights a document carries when its type lists them in
/// `required`. `$id` and `$ownerId` are in every record already.
pub const KEEPABLE_SYSTEM_PROPERTIES: [&str; 9] = [
    CREATED_AT,
    UPDATED_AT,
    TRANSFERRED_AT,
    CREATED_AT_BLOCK_HEIGHT,
    UPDATED_AT_BLOCK_HEIGHT,
    TRANSFERRED_AT_BLOCK_HEIGHT,
    CREATED_AT_CORE_BLOCK_HEIGHT,
    UPDATED_AT_CORE_BLOCK_HEIGHT,
    TRANSFERRED_AT_CORE_BLOCK_HEIGHT,
];

/// The values a removal record keeps of `document`: for each path of `kept_fields`, the value
/// the document holds there, a property at any depth or one of the
/// [`KEEPABLE_SYSTEM_PROPERTIES`] (a time as a `U64` of milliseconds, a block height as a
/// `U64`, a core block height as a `U32`). A path the document holds no value at, or a null,
/// is left out.
pub fn kept_field_values(
    document: &Document,
    kept_fields: &BTreeSet<String>,
) -> BTreeMap<String, Value> {
    kept_fields
        .iter()
        .filter_map(|path| {
            let value = match path.as_str() {
                CREATED_AT => document.created_at().map(Value::U64),
                UPDATED_AT => document.updated_at().map(Value::U64),
                TRANSFERRED_AT => document.transferred_at().map(Value::U64),
                CREATED_AT_BLOCK_HEIGHT => document.created_at_block_height().map(Value::U64),
                UPDATED_AT_BLOCK_HEIGHT => document.updated_at_block_height().map(Value::U64),
                TRANSFERRED_AT_BLOCK_HEIGHT => {
                    document.transferred_at_block_height().map(Value::U64)
                }
                CREATED_AT_CORE_BLOCK_HEIGHT => {
                    document.created_at_core_block_height().map(Value::U32)
                }
                UPDATED_AT_CORE_BLOCK_HEIGHT => {
                    document.updated_at_core_block_height().map(Value::U32)
                }
                TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
                    document.transferred_at_core_block_height().map(Value::U32)
                }
                path => document.get(path).filter(|value| !value.is_null()).cloned(),
            }?;
            Some((path.clone(), value))
        })
        .collect()
}

impl ContractDocumentRemoval {
    /// Whether the document was restored: the record then describes a removal that was
    /// undone, and the document is live again.
    pub fn is_restored(&self) -> bool {
        self.restoration.is_some()
    }
}
