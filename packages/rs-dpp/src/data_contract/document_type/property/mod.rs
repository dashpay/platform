use std::collections::BTreeMap;
use std::convert::TryInto;

use std::io::{BufReader, Cursor, Read};

use crate::data_contract::errors::DataContractError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};

use crate::consensus::basic::decode::DecodingError;
use crate::consensus::basic::document::DocumentPropertyNotDistinctError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::v1::DataContractConfigGettersV1;
use crate::data_contract::config::v2::DataContractConfigGettersV2;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{property_names, DocumentTypeRef};
use crate::data_contract::DataContract;
use crate::document::property_names::{CREATOR_ID, ID, OWNER_ID};
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::identity::{IdentityPublicKey, Purpose};
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use array::{ArrayItemType, TypedArrayProperty};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use integer_encoding::{VarInt, VarIntReader};
use itertools::Itertools;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueMapPathHelper};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub mod array;
pub mod encrypted_for;
pub mod list_element_reference;
pub mod reference_expression;
pub mod reference_lookup;

pub use encrypted_for::{EncryptedFor, EncryptedForRecipient, EncryptionScheme};
pub use list_element_reference::ListElementReference;
pub use reference_expression::{
    ReferenceCombinator, ReferenceOperands, COMBINABLE_REFERENCE_TARGET_TYPES,
    MAX_REFERENCE_EXPRESSION_DECODE_DEPTH,
};
pub use reference_lookup::{DocumentReferenceLookup, LookupKeySource};

#[cfg(test)]
mod byte_array_encoding_flip_tests;

// This struct will be changed in future to support more validation logic and serialization
// It will become versioned and it will be introduced by a new document type version
// @append_only
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct DocumentProperty {
    pub property_type: DocumentPropertyType,
    pub required: bool,
    pub transient: bool,
    /// The contract version this property is required from (`requiredSince`).
    /// `None` for plain-required properties (required at every version) and
    /// for optional properties. Only ever `Some` when `required` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_since: Option<u32>,
    /// What this identifier property's value must differ from (`distinctFrom`):
    /// the document's `$ownerId` or another identifier property of the same
    /// document type. `None` for every property that declares nothing, which
    /// is every property parsed before protocol version 14.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct_from: Option<DistinctFrom>,
    /// How the property's bytes were encrypted (`encryptedFor`): the recipient,
    /// the key ids and the scheme. Only ever `Some` on a byte array property,
    /// and only on contracts parsed from protocol version 14 on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_for: Option<EncryptedFor>,
}

/// What a `distinctFrom` identifier property must differ from.
///
/// Declared as `"distinctFrom": "$ownerId"` or `"distinctFrom": "<dotted property path>"`
/// on an identifier property, or on the `items` of a typed array of identifiers, where it
/// binds every element (meta-schema v3, protocol version 14). A pure structure rule:
/// consensus compares the property's value with the named one when the document is created
/// or replaced, and refuses an equal pair with `DocumentPropertyNotDistinctError` (10419).
/// When the named property is absent from the document there is nothing to differ from,
/// so the rule passes. A transfer to, or a purchase by, the identity an `$ownerId`
/// declaration names is refused the same way, judged against the stored document. The
/// target is checked at contract registration and update: it must
/// be `$ownerId` or an existing identifier property of the same document type other than
/// the declaring one.
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(into = "String")]
pub enum DistinctFrom {
    /// The document's `$ownerId`, which the write transition carries.
    OwnerId,
    /// The dotted path of another identifier property of the same document type.
    Property(String),
}

impl DistinctFrom {
    /// The declaration a wire name spells: `$ownerId` or a property path. Any other
    /// `$`-prefixed name is refused, since no other system property is an identifier the
    /// rule could compare against.
    pub fn from_wire_name(name: &str) -> Result<Self, DataContractError> {
        if name == OWNER_ID {
            return Ok(DistinctFrom::OwnerId);
        }
        if name.starts_with('$') {
            return Err(DataContractError::InvalidContractStructure(format!(
                "distinctFrom must name \"{OWNER_ID}\" or a property of the same document type, \
                 not system property \"{name}\""
            )));
        }
        if name.is_empty() || name.len() > 256 {
            return Err(DataContractError::InvalidContractStructure(
                "distinctFrom property paths must be between 1 and 256 characters".to_string(),
            ));
        }
        Ok(DistinctFrom::Property(name.to_string()))
    }

    /// The wire name, as the schema spells it.
    pub fn as_str(&self) -> &str {
        match self {
            DistinctFrom::OwnerId => OWNER_ID,
            DistinctFrom::Property(path) => path.as_str(),
        }
    }

    /// The collision this declaration finds for one value: the error to refuse the write
    /// with when `value`, the declaring property's own value, equals what it must differ
    /// from, and `None` when the two differ or when the named property is absent from
    /// `data` (there is nothing to differ from). `value` is passed on its own rather than
    /// read from `data` so that an array item can be judged by the same rule with the
    /// item's value; `path` is the declaring property's dotted path, for the error.
    ///
    /// A value on either side that is not a 32-byte identifier cannot collide: the schema
    /// validation that precedes this check refuses such a document on its own.
    pub fn violation(
        &self,
        document_type_name: &str,
        path: &str,
        value: &Value,
        data: &BTreeMap<String, Value>,
        owner_id: Identifier,
    ) -> Option<DocumentPropertyNotDistinctError> {
        let Ok(value) = value.to_identifier() else {
            return None;
        };
        let other = match self {
            DistinctFrom::OwnerId => owner_id,
            DistinctFrom::Property(target) => {
                // A lookup error (an intermediate that is not an object) is the same as
                // absence here: the schema forbids the shape, so nothing to compare against.
                let Ok(Some(other)) = data.get_optional_at_path(target) else {
                    return None;
                };
                let Ok(other) = other.to_identifier() else {
                    return None;
                };
                other
            }
        };
        (value == other).then(|| {
            DocumentPropertyNotDistinctError::new(
                document_type_name.to_string(),
                path.to_string(),
                self.as_str().to_string(),
            )
        })
    }
}

impl From<DistinctFrom> for String {
    fn from(distinct_from: DistinctFrom) -> Self {
        distinct_from.as_str().to_string()
    }
}

impl DocumentProperty {
    /// Whether this property is required for a document whose bytes conform to
    /// `contract_version` (the document's stamp). `None` means the document
    /// was serialized before format 3, which predates every `requiredSince`
    /// annotation, so only unconditionally required properties count.
    pub fn required_at(&self, contract_version: Option<u32>) -> bool {
        self.required
            && match self.required_since {
                None => true,
                Some(since) => contract_version.is_some_and(|version| version >= since),
            }
    }

    /// Whether this property is required regardless of any document's
    /// contract-version stamp: `required` without a `requiredSince` gate.
    /// This is the requiredness the pre-stamp serialization formats (0–2)
    /// encode, and it equals `required_at(None)` — an unstamped document
    /// predates every `requiredSince` annotation.
    pub fn always_required(&self) -> bool {
        self.required && self.required_since.is_none()
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct StringPropertySizes {
    pub min_length: Option<u16>,
    pub max_length: Option<u16>,
    /// The most UTF-8 bytes a value may take (`maxBytes`, meta-schema v3,
    /// protocol version 14). `max_length` counts characters, which are up to
    /// four bytes each. `None` on every string that declares none, which is
    /// every string parsed before protocol version 14.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u16>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ByteArrayPropertySizes {
    pub min_size: Option<u16>,
    pub max_size: Option<u16>,
}

/// What a `contract` reference requires of the contract it points at, beyond its existence.
///
/// Declared as `refersTo: { "type": "contract", "contractRequirements": { ... } }`: each key names an
/// aspect of the referenced contract and its value the requirement on it. Consensus checks the
/// requirements when the referring document is written, against the contract it has already
/// fetched for the existence check and the write itself (its owner and block time), so a
/// requirement costs no further read. An unmet one refuses the write with
/// `ReferencedContractRequirementNotMetError` (40135).
#[derive(
    Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize, Encode, Decode, DecodeUntrusted,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractReferenceRequirements {
    /// The moderation the referenced contract must declare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation: Option<ContractReferenceModeration>,
    /// How long, in seconds, the referenced contract must have existed when the referring
    /// document is written: its recorded creation time plus this many seconds must not be
    /// after the block time. A contract that never recorded a creation time does not meet it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_age_seconds: Option<u32>,
    /// How long, in seconds, the referenced contract must have been unchanged when the
    /// referring document is written: the later of its recorded creation and last update
    /// times plus this many seconds must not be after the block time. A contract that never
    /// recorded a creation time does not meet it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_seconds_since_update: Option<u32>,
    /// Who must own the referenced contract, relative to the writer of the referring document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<ContractReferenceOwner>,
    /// Whether the referenced contract must be read-only (its config's `readonly`), a
    /// contract that can never be updated again. Only `true` is declarable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
    /// Whether the referenced contract must keep its history (its config's `keepsHistory`).
    /// Only `true` is declarable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keeps_history: Option<bool>,
    /// Whether the elected moderation declaration of the referenced contract must protect
    /// (`true`), or must not protect (`false`), the contract owner from the team, its
    /// `ownerProtected`. Either value implies elected moderation: a contract with no
    /// moderation, or with moderation of another kind, meets neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_protected: Option<bool>,
}

/// The write of a referring document, what a contract reference's requirements are checked
/// against beside the referenced contract itself.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ReferringWrite {
    /// The `$ownerId` of the referring document: the owner of the transition writing it.
    pub owner_id: Identifier,
    /// The time of the block writing it.
    pub block_time_ms: TimestampMillis,
}

/// Who a `contract` reference may require to own the referenced contract, relative to the
/// writer of the referring document.
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Encode, Decode, DecodeUntrusted,
)]
pub enum ContractReferenceOwner {
    /// The writer itself: the contract's owner is the `$ownerId` of the referring document,
    /// a write gate like the `$ownerId` property agreement of a document reference.
    #[serde(rename = "self")]
    Writer,
    /// Anyone but the writer.
    #[serde(rename = "other")]
    Other,
}

impl ContractReferenceOwner {
    /// The wire name, the value of `contractRequirements.owner`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractReferenceOwner::Writer => "self",
            ContractReferenceOwner::Other => "other",
        }
    }

    /// The owner relation a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            "self" => Some(ContractReferenceOwner::Writer),
            "other" => Some(ContractReferenceOwner::Other),
            _ => None,
        }
    }

    /// Whether `contract`, owned as it is, meets this for a referring document owned by
    /// `writer_id`.
    pub fn is_met_by(&self, contract: &DataContract, writer_id: &Identifier) -> bool {
        match self {
            ContractReferenceOwner::Writer => contract.owner_id() == *writer_id,
            ContractReferenceOwner::Other => contract.owner_id() != *writer_id,
        }
    }
}

/// The moderation a `contract` reference may require of the referenced contract.
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Encode, Decode, DecodeUntrusted,
)]
#[serde(rename_all = "camelCase")]
pub enum ContractReferenceModeration {
    /// The contract declares an elected moderation team (`ContractModerators::Elected`),
    /// whatever its interim, whether a team is seated yet and whether its election delay
    /// has passed. A charter proposal declares this, so teams can form during the notice
    /// the contract gives before its first election.
    Elected,
    /// The contract declares an elected moderation team whose own `electionDelay`, counted
    /// from the contract's creation, has passed at the block time of the write, or which
    /// declares no delay. The delay is the contract's, not the reference's: the charter
    /// that opens the contest declares this and carries no number.
    ElectionOpen,
}

impl ContractReferenceModeration {
    /// The wire name, the value of `contractRequirements.moderation`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractReferenceModeration::Elected => "elected",
            ContractReferenceModeration::ElectionOpen => "electionOpen",
        }
    }

    /// The wire names, for the message that refuses another.
    pub const WIRE_NAMES: &'static [&'static str] = &["elected", "electionOpen"];

    /// The moderation a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            "elected" => Some(ContractReferenceModeration::Elected),
            "electionOpen" => Some(ContractReferenceModeration::ElectionOpen),
            _ => None,
        }
    }

    /// How the requirement reads after "a contract with".
    pub fn describe(&self) -> &'static str {
        match self {
            ContractReferenceModeration::Elected => "elected moderation",
            ContractReferenceModeration::ElectionOpen => "its moderation election open",
        }
    }

    /// Whether `contract` declares what this requires at `block_time_ms`, the time of the
    /// block writing the referring document.
    pub fn is_met_by(&self, contract: &DataContract, block_time_ms: TimestampMillis) -> bool {
        let elected = contract
            .config()
            .moderation()
            .and_then(|moderation| moderation.moderators.elected());
        match self {
            ContractReferenceModeration::Elected => elected.is_some(),
            ContractReferenceModeration::ElectionOpen => elected.is_some_and(|elected| {
                elected.election_is_open(contract.created_at(), block_time_ms)
            }),
        }
    }
}

/// One requirement of a [`ContractReferenceRequirements`] declaration, named the way the
/// declaration spells it, for the error that reports it unmet.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContractReferenceRequirement {
    Moderation(ContractReferenceModeration),
    MinimumAgeSeconds(u32),
    MinimumSecondsSinceUpdate(u32),
    Owner(ContractReferenceOwner),
    Readonly(bool),
    KeepsHistory(bool),
    OwnerProtected(bool),
}

impl ContractReferenceRequirement {
    /// The `contractRequirements` key the requirement was declared under.
    pub fn field(&self) -> &'static str {
        match self {
            ContractReferenceRequirement::Moderation(_) => property_names::MODERATION,
            ContractReferenceRequirement::MinimumAgeSeconds(_) => {
                property_names::MINIMUM_AGE_SECONDS
            }
            ContractReferenceRequirement::MinimumSecondsSinceUpdate(_) => {
                property_names::MINIMUM_SECONDS_SINCE_UPDATE
            }
            ContractReferenceRequirement::Owner(_) => property_names::OWNER,
            ContractReferenceRequirement::Readonly(_) => property_names::READONLY,
            ContractReferenceRequirement::KeepsHistory(_) => property_names::KEEPS_HISTORY,
            ContractReferenceRequirement::OwnerProtected(_) => property_names::OWNER_PROTECTED,
        }
    }

    /// The value the declaration requires, as spelled in the schema.
    pub fn required(&self) -> String {
        match self {
            ContractReferenceRequirement::Moderation(moderation) => moderation.as_str().to_string(),
            ContractReferenceRequirement::MinimumAgeSeconds(seconds)
            | ContractReferenceRequirement::MinimumSecondsSinceUpdate(seconds) => {
                seconds.to_string()
            }
            ContractReferenceRequirement::Owner(owner) => owner.as_str().to_string(),
            ContractReferenceRequirement::Readonly(flag)
            | ContractReferenceRequirement::KeepsHistory(flag)
            | ContractReferenceRequirement::OwnerProtected(flag) => flag.to_string(),
        }
    }

    /// Whether `contract` meets this requirement for `write`, the write of the referring
    /// document.
    pub fn is_met_by(&self, contract: &DataContract, write: ReferringWrite) -> bool {
        match self {
            ContractReferenceRequirement::Moderation(moderation) => {
                moderation.is_met_by(contract, write.block_time_ms)
            }
            ContractReferenceRequirement::MinimumAgeSeconds(seconds) => {
                Self::minimum_age_is_met(contract.created_at(), *seconds, write.block_time_ms)
            }
            ContractReferenceRequirement::MinimumSecondsSinceUpdate(seconds) => {
                Self::minimum_age_is_met(
                    Self::last_change_time(contract),
                    *seconds,
                    write.block_time_ms,
                )
            }
            ContractReferenceRequirement::Owner(owner) => {
                owner.is_met_by(contract, &write.owner_id)
            }
            ContractReferenceRequirement::Readonly(required) => {
                contract.config().readonly() == *required
            }
            ContractReferenceRequirement::KeepsHistory(required) => {
                contract.config().keeps_history() == *required
            }
            // Either value needs an elected declaration to read the flag from: a contract
            // without one meets neither
            ContractReferenceRequirement::OwnerProtected(required) => contract
                .config()
                .moderation()
                .and_then(|moderation| moderation.moderators.elected())
                .is_some_and(|elected| elected.owner_protected == *required),
        }
    }

    /// Whether something that happened at `since` is at least `minimum_seconds` in the past
    /// at `block_time_ms`. A contract without the recorded time (one created before contracts
    /// recorded it) is of unknown age and does not meet any minimum.
    pub fn minimum_age_is_met(
        since: Option<TimestampMillis>,
        minimum_seconds: u32,
        block_time_ms: TimestampMillis,
    ) -> bool {
        let Some(since) = since else {
            return false;
        };
        let old_enough_at =
            since.saturating_add(TimestampMillis::from(minimum_seconds).saturating_mul(1000));
        block_time_ms >= old_enough_at
    }

    /// When `contract` last changed: its last update, or its creation for a contract never
    /// updated. `None` when it recorded neither.
    pub fn last_change_time(contract: &DataContract) -> Option<TimestampMillis> {
        match (contract.created_at(), contract.updated_at()) {
            (Some(created_at), Some(updated_at)) => Some(created_at.max(updated_at)),
            (created_at, updated_at) => updated_at.or(created_at),
        }
    }
}

impl ContractReferenceRequirements {
    /// Whether the declaration requires nothing beyond the contract's existence.
    pub fn is_empty(&self) -> bool {
        self.moderation.is_none()
            && self.minimum_age_seconds.is_none()
            && self.minimum_seconds_since_update.is_none()
            && self.owner.is_none()
            && self.readonly.is_none()
            && self.keeps_history.is_none()
            && self.owner_protected.is_none()
    }

    /// The requirements, in declaration order.
    pub fn requirements(&self) -> impl Iterator<Item = ContractReferenceRequirement> + '_ {
        self.moderation
            .into_iter()
            .map(ContractReferenceRequirement::Moderation)
            .chain(
                self.minimum_age_seconds
                    .into_iter()
                    .map(ContractReferenceRequirement::MinimumAgeSeconds),
            )
            .chain(
                self.minimum_seconds_since_update
                    .into_iter()
                    .map(ContractReferenceRequirement::MinimumSecondsSinceUpdate),
            )
            .chain(
                self.owner
                    .into_iter()
                    .map(ContractReferenceRequirement::Owner),
            )
            .chain(
                self.readonly
                    .into_iter()
                    .map(ContractReferenceRequirement::Readonly),
            )
            .chain(
                self.keeps_history
                    .into_iter()
                    .map(ContractReferenceRequirement::KeepsHistory),
            )
            .chain(
                self.owner_protected
                    .into_iter()
                    .map(ContractReferenceRequirement::OwnerProtected),
            )
    }

    /// The first requirement `contract` does not meet for `write`, the write of the referring
    /// document, `None` when it meets them all.
    pub fn first_unmet_by(
        &self,
        contract: &DataContract,
        write: ReferringWrite,
    ) -> Option<ContractReferenceRequirement> {
        self.requirements()
            .find(|requirement| !requirement.is_met_by(contract, write))
    }
}

/// What an `identityPublicKey` reference requires of the key it points at, beyond its existence
/// and its not being disabled.
///
/// Declared as `refersTo: { "type": "identityPublicKey", "keyIdProperty": ..., "keyRequirements":
/// { ... } }`: each key names an aspect of the referenced key and its value the requirement on it.
/// Consensus checks the requirements when the referring document is written, against the key it
/// has already fetched for the existence check, so a requirement costs no further read. An unmet
/// one refuses the write with `ReferencedIdentityKeyRequirementNotMetError` (40136). Keys are
/// added to this object as new requirements arrive (a security level, say); a requirement is
/// never a new reference type.
#[derive(
    Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize, Encode, Decode, DecodeUntrusted,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityKeyReferenceRequirements {
    /// The purpose the referenced key must have, spelled by its wire name (`"decryption"`).
    /// Any purpose but `SYSTEM`, which no identity key of a user carries.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "purpose_wire_name"
    )]
    pub purpose: Option<Purpose>,
    /// The document type of the declaring contract the referenced key must be bound to: its
    /// contract bounds must be `SingleContractDocumentType` naming the declaring contract and
    /// exactly this type. A whole-contract bound or a contract group bound never meets it, even
    /// where the group holds the type: the check reads nothing beyond the key. Validated when
    /// the contract is registered to name a document type of the declaring contract that a key
    /// of the required purpose can be bound to, so the check never needs a second contract
    /// fetch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_to: Option<String>,
}

/// Serde for [`IdentityKeyReferenceRequirements::purpose`]: the purpose's wire name, not the
/// number `Purpose` serializes to on an identity key, so the value matches the schema keyword.
mod purpose_wire_name {
    use crate::identity::Purpose;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        purpose: &Option<Purpose>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        purpose
            .as_ref()
            .map(Purpose::wire_name)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Purpose>, D::Error> {
        let name: Option<String> = Option::deserialize(deserializer)?;
        name.map(|name| {
            // The purposes a user's key can carry, every one but SYSTEM, as the schema
            // parser admits them
            Purpose::from_wire_name(&name)
                .filter(|purpose| Purpose::full_range().contains(purpose))
                .ok_or_else(|| D::Error::custom(format!("unknown key purpose {name:?}")))
        })
        .transpose()
    }
}

/// One requirement of an [`IdentityKeyReferenceRequirements`] declaration, named the way the
/// declaration spells it, for the error that reports it unmet.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IdentityKeyReferenceRequirement<'a> {
    Purpose(Purpose),
    BoundTo(&'a str),
}

impl IdentityKeyReferenceRequirement<'_> {
    /// The `keyRequirements` key the requirement was declared under.
    pub fn field(&self) -> &'static str {
        match self {
            IdentityKeyReferenceRequirement::Purpose(_) => property_names::PURPOSE,
            IdentityKeyReferenceRequirement::BoundTo(_) => property_names::BOUND_TO,
        }
    }

    /// The value the declaration requires, as spelled in the schema.
    pub fn required(&self) -> String {
        match self {
            IdentityKeyReferenceRequirement::Purpose(purpose) => purpose.wire_name().to_string(),
            IdentityKeyReferenceRequirement::BoundTo(document_type_name) => {
                document_type_name.to_string()
            }
        }
    }

    /// Whether `key`, a key of an identity, meets this requirement for a reference declared by
    /// the contract `declaring_contract_id`.
    pub fn is_met_by(&self, key: &IdentityPublicKey, declaring_contract_id: Identifier) -> bool {
        match self {
            IdentityKeyReferenceRequirement::Purpose(purpose) => key.purpose() == *purpose,
            IdentityKeyReferenceRequirement::BoundTo(document_type_name) => matches!(
                key.contract_bounds(),
                Some(ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name: bound_document_type_name,
                }) if *id == declaring_contract_id && bound_document_type_name == document_type_name
            ),
        }
    }

    /// What `key` has where the declaration requires [`Self::required`], for the error that
    /// reports the requirement unmet.
    pub fn actual_of(&self, key: &IdentityPublicKey) -> String {
        match self {
            IdentityKeyReferenceRequirement::Purpose(_) => key.purpose().wire_name().to_string(),
            IdentityKeyReferenceRequirement::BoundTo(_) => match key.contract_bounds() {
                None => "no contract bounds".to_string(),
                Some(ContractBounds::SingleContract { id }) => {
                    format!("whole contract {id}, not a document type")
                }
                Some(ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name,
                }) => format!("contract {id} document type {document_type_name}"),
                Some(ContractBounds::ContractGroup { id }) => {
                    format!("contract group {id}, which never meets a document type bound")
                }
            },
        }
    }
}

impl IdentityKeyReferenceRequirements {
    /// Whether the declaration requires nothing beyond the key's existence.
    pub fn is_empty(&self) -> bool {
        self.purpose.is_none() && self.bound_to.is_none()
    }

    /// The requirements, in declaration order.
    pub fn requirements(&self) -> impl Iterator<Item = IdentityKeyReferenceRequirement<'_>> + '_ {
        self.purpose
            .into_iter()
            .map(IdentityKeyReferenceRequirement::Purpose)
            .chain(
                self.bound_to
                    .as_deref()
                    .map(IdentityKeyReferenceRequirement::BoundTo),
            )
    }

    /// The first requirement `key` does not meet for a reference declared by the contract
    /// `declaring_contract_id`, `None` when it meets them all.
    pub fn first_unmet_by(
        &self,
        key: &IdentityPublicKey,
        declaring_contract_id: Identifier,
    ) -> Option<IdentityKeyReferenceRequirement<'_>> {
        self.requirements()
            .find(|requirement| !requirement.is_met_by(key, declaring_contract_id))
    }
}

// This enum is embedded in consensus errors, so it is consensus-serialized.
// @append_only
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Serialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[serde(rename_all = "lowercase")]
pub enum DocumentPropertyReferenceTarget {
    Identity,
    /// A data contract, which must exist when the referring document is written and meet the
    /// declared [`ContractReferenceRequirements`], if any.
    Contract {
        #[serde(
            default,
            skip_serializing_if = "ContractReferenceRequirements::is_empty"
        )]
        contract_requirements: ContractReferenceRequirements,
    },
    Token,
    /// A document of a document type whose documents can never be deleted
    /// (`canBeDeleted: false`). Only such document types may be referenced:
    /// together with document types being non-removable and the
    /// `canBeDeleted` flag being immutable on contract updates, this
    /// guarantees a validated reference can never dangle.
    #[serde(rename = "permanentDocument")]
    PermanentDocument {
        /// The contract the referenced document type lives in; `None` means
        /// the declaring contract itself
        contract_id: Option<Identifier>,
        document_type_name: String,
        /// Property agreement: each `{referring property: referenced
        /// property}` pair must hold as an EQUALITY between the referring
        /// document's value and the referenced document's value, checked by
        /// consensus at document write time (the referenced document is
        /// already fetched for existence validation, so agreement adds no
        /// reads). The referring side is a schema property of the declaring
        /// document type or its own `$ownerId`, the writer, which makes the
        /// pair a write gate (see [`REFERRING_SYSTEM_AGREEMENT_PROPERTIES`]).
        /// The referenced side is a schema
        /// property of the referenced document type or one of the system
        /// identifiers in [`REFERENCED_SYSTEM_AGREEMENT_PROPERTIES`]:
        /// `$ownerId`, which follows the referenced document through
        /// transfers, or `$creatorId`, set once at creation. Declarations
        /// are validated at contract registration: both sides must exist
        /// and share one value kind (an identifier on the referring side
        /// for the system names), and `$creatorId` needs a referenced
        /// document type that records creator ids at all.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        property_agreement: BTreeMap<String, String>,
    },
    /// A specific public key of an identity: the property value holds the
    /// identity id and the named sibling property of the same document type
    /// holds the key id. Identity keys can be disabled but never removed, so
    /// an existing reference can never dangle; at write time the key must
    /// exist, must not be disabled and must meet the declared
    /// [`IdentityKeyReferenceRequirements`], if any. The referenced key is
    /// the (identity id, key id) pair, so a replace that changes either
    /// property re-validates the reference.
    #[serde(rename = "identityPublicKey")]
    IdentityPublicKey {
        /// The property of the same document type whose value carries the
        /// referenced key id
        key_id_property: String,
        /// What the referenced key must be beyond existing: a purpose, a
        /// binding to a document type of the declaring contract
        #[serde(
            default,
            skip_serializing_if = "IdentityKeyReferenceRequirements::is_empty"
        )]
        key_requirements: IdentityKeyReferenceRequirements,
    },
    /// A document of a document type whose documents CAN be deleted: the
    /// counterpart of [`Self::PermanentDocument`], disjoint from it, so a
    /// declaration always states which guarantee the reference carries.
    /// The declaration shape and the write-time validation are the same:
    /// the referenced document must exist, and every `property_agreement`
    /// pair must hold, when the referring document is written. Nothing is
    /// promised afterwards: the referenced document may be deleted, the
    /// deletion is not blocked by referring documents, and a reader must
    /// expect the reference to resolve to nothing. It can not come back
    /// pointing at something else: a document id commits to the nonce of
    /// its create transition, so an id is produced at most once and a
    /// reference means that one document or nothing (its lookup form,
    /// [`Self::DeletableDocumentLookup`], promises less: a key may find a new
    /// document once the one it found is deleted). A WRITER may not leave
    /// it that way: every replace of the referring document re-validates
    /// the reference, so a dead one has to be repointed at a document that
    /// exists or cleared (on an `immutable` property, clearing is the only
    /// move, and the immutable check lets it through). Features that lean
    /// on the target staying in state (`preallocated` index trees) are not
    /// available through it.
    #[serde(rename = "deletableDocument")]
    DeletableDocument {
        /// The contract the referenced document type lives in; `None` means
        /// the declaring contract itself
        contract_id: Option<Identifier>,
        document_type_name: String,
        /// See [`Self::PermanentDocument`]'s `property_agreement`.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        property_agreement: BTreeMap<String, String>,
    },
    /// A `permanentDocument` reference declared with a `lookup`: the
    /// property's value (or each element of a typed array) is NOT the
    /// referenced document's id, but one part of a key; the referenced
    /// document is the one the named unique index of the referenced document
    /// type finds for the key the [`DocumentReferenceLookup`] assembles from
    /// the referring document. Everything else is as for
    /// [`Self::PermanentDocument`]: the referenced type must forbid deletion,
    /// the agreement pairs are checked against the document found, and the
    /// key must stay with that document (its parts cannot be changed by a
    /// replace, a transfer or a purchase), so the reference can not dangle
    /// either. Its deletable form is [`Self::DeletableDocumentLookup`].
    ///
    /// A variant of its own rather than a field of
    /// [`Self::PermanentDocument`], appended as this enum's rule requires: an
    /// id reference keeps its consensus encoding (the enum is embedded in
    /// reference errors), and code matching `PermanentDocument` as "the value
    /// is a document id" can not mistake a lookup for one. It serializes
    /// under the same `permanentDocument` tag, with a `lookup` field (the
    /// enum is serialize-only, so the shared tag is never read back).
    #[serde(rename = "permanentDocument")]
    PermanentDocumentLookup {
        /// The contract the referenced document type lives in; `None` means
        /// the declaring contract itself
        contract_id: Option<Identifier>,
        document_type_name: String,
        /// See [`Self::PermanentDocument`]'s `property_agreement`.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        property_agreement: BTreeMap<String, String>,
        /// How the referenced document is found.
        lookup: DocumentReferenceLookup,
    },
    /// Two or more operands, declared as `{ "anyOf": [operand, ...] }`: the
    /// reference holds if at least one of them holds. An operand is a leaf,
    /// an ordinary declaration of an `identity` or a `permanentDocument` (by
    /// id or through a lookup), or an [`Self::AllOf`] (see
    /// [`ReferenceOperands`] for the rules and why the other kinds are left
    /// out). At write time the operands are checked in declared order and
    /// the first that holds ends the check; every read is billed, and when
    /// none holds the write is refused with the error of the last operand,
    /// so a reference error never carries this variant. A
    /// `propertyAgreement` belongs to its leaf and is checked only against
    /// that leaf's document.
    ///
    /// Not a document reference as a whole
    /// ([`Self::as_any_document_reference`] is `None`): code that checks
    /// each declaration walks [`Self::leaves`], and code that needs one
    /// target (joins, preallocated indexes) refuses it.
    #[serde(rename = "anyOf")]
    AnyOf(ReferenceOperands),
    /// Two or more operands, declared as `{ "allOf": [operand, ...] }`: the
    /// reference holds if every one of them holds for the same value. An
    /// operand is a leaf, as for [`Self::AnyOf`], or an [`Self::AnyOf`]. At
    /// write time the operands are checked in declared order and the first
    /// that fails ends the check, refusing the write with its error; every
    /// read is billed. Otherwise as [`Self::AnyOf`].
    #[serde(rename = "allOf")]
    AllOf(ReferenceOperands),
    /// An element of a list: the value must be one of the identifiers the
    /// typed array [`ListElementReference::in_list`] holds on the one
    /// document of a permanent document type that agrees with the referring
    /// document on every `propertyAgreement` pair, found by the pair whose
    /// referenced side is `$id`. A document reference in every other respect
    /// ([`Self::as_any_document_reference`] carries it with `in_list` set):
    /// the value is neither the document's id nor a lookup key, so
    /// [`Self::as_document_reference`] leaves it out. The list's document can
    /// never be deleted and its list never changes (checked at registration),
    /// so an accepted value stays an element for good. Appended, so every
    /// earlier variant keeps its consensus encoding.
    #[serde(rename = "listElement")]
    ListElement(ListElementReference),
    /// A `deletableDocument` reference declared with a `lookup`: the value is
    /// one part of a key, as for [`Self::PermanentDocumentLookup`], into a
    /// document type whose documents CAN be deleted. The document the key
    /// finds must exist, and the agreement pairs hold against it, when the
    /// referring document is written, and every replace re-validates it, as a
    /// [`Self::DeletableDocument`] reference is. It promises less than the id
    /// form: once the document it found is deleted, the same key may find
    /// another one filed later, so the reference says "a document with this
    /// key exists now", not "this document". That is what a membership gate
    /// needs, such as "the writer is currently an added moderator of this
    /// charter" (`ownerRefersTo`, the one doctype reference that takes it).
    /// An immutable property can not hold one (a replace could neither keep a
    /// dead one nor clear it), and it may be an operand of a reference
    /// expression, which is then re-validated on every replace as well.
    ///
    /// Appended, so every earlier variant keeps its consensus encoding. It
    /// serializes under the `deletableDocument` tag, with a `lookup` field.
    #[serde(rename = "deletableDocument")]
    DeletableDocumentLookup {
        /// The contract the referenced document type lives in; `None` means
        /// the declaring contract itself
        contract_id: Option<Identifier>,
        document_type_name: String,
        /// See [`Self::PermanentDocument`]'s `property_agreement`.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        property_agreement: BTreeMap<String, String>,
        /// How the referenced document is found.
        lookup: DocumentReferenceLookup,
    },
}

/// The declaration content every document reference target shares:
/// [`DocumentPropertyReferenceTarget::PermanentDocument`] and
/// [`DocumentPropertyReferenceTarget::DeletableDocument`], whose value is the
/// referenced document's id, [`DocumentPropertyReferenceTarget::PermanentDocumentLookup`]
/// (`lookup` set), whose value is one part of a key, and
/// [`DocumentPropertyReferenceTarget::ListElement`] (`in_list` set), whose
/// value is an element of the referenced document's list. Only
/// [`DocumentPropertyReferenceTarget::as_document_reference`] promises the
/// value is a document id.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DocumentReferenceDeclaration<'a> {
    /// The contract the referenced document type lives in; `None` means
    /// the declaring contract itself
    pub contract_id: Option<Identifier>,
    /// The referenced document type
    pub document_type_name: &'a str,
    /// The `{referring property: referenced property}` equalities
    pub property_agreement: &'a BTreeMap<String, String>,
    /// Whether the referenced document type must forbid deletion
    /// (`permanentDocument`, by id or through a lookup, and `listElement`)
    /// or must allow it (`deletableDocument`)
    pub permanent: bool,
    /// How the referenced document is found when the value is not its id
    /// ([`DocumentPropertyReferenceTarget::PermanentDocumentLookup`] and
    /// [`DocumentPropertyReferenceTarget::DeletableDocumentLookup`]); `None`
    /// when the value is the referenced document's id. Only
    /// [`DocumentPropertyReferenceTarget::as_any_document_reference`] ever
    /// returns a declaration carrying one.
    pub lookup: Option<&'a DocumentReferenceLookup>,
    /// The typed array of identifiers the value must be an element of
    /// ([`DocumentPropertyReferenceTarget::ListElement`]); `None` when the
    /// value is the referenced document's id or a lookup key part. Only
    /// [`DocumentPropertyReferenceTarget::as_any_document_reference`] ever
    /// returns a declaration carrying one.
    pub in_list: Option<&'a str>,
}

impl DocumentPropertyReferenceTarget {
    /// The declaration of a reference whose value is a DOCUMENT's id, of
    /// either kind; `None` for every other target, a lookup reference or a
    /// list element included, whose value is not a document id. This is the
    /// accessor for code that treats the value as the referenced document's
    /// `$id` (by-id joins); code that validates every kind of document
    /// reference uses [`Self::as_any_document_reference`].
    pub fn as_document_reference(&self) -> Option<DocumentReferenceDeclaration<'_>> {
        self.as_any_document_reference()
            .filter(|declaration| declaration.lookup.is_none() && declaration.in_list.is_none())
    }

    /// The declaration of any reference to a DOCUMENT: of either kind, and
    /// found by its id, through a `lookup` (then `lookup` is `Some`, and the
    /// value is not the document's id) or by a `$id` agreement pair with the
    /// value an element of its list (then `in_list` is `Some`). `None` for
    /// every other target.
    pub fn as_any_document_reference(&self) -> Option<DocumentReferenceDeclaration<'_>> {
        match self {
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name,
                property_agreement,
            } => Some(DocumentReferenceDeclaration {
                contract_id: *contract_id,
                document_type_name,
                property_agreement,
                permanent: true,
                lookup: None,
                in_list: None,
            }),
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id,
                document_type_name,
                property_agreement,
                lookup,
            } => Some(DocumentReferenceDeclaration {
                contract_id: *contract_id,
                document_type_name,
                property_agreement,
                permanent: true,
                lookup: Some(lookup),
                in_list: None,
            }),
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id,
                document_type_name,
                property_agreement,
            } => Some(DocumentReferenceDeclaration {
                contract_id: *contract_id,
                document_type_name,
                property_agreement,
                permanent: false,
                lookup: None,
                in_list: None,
            }),
            DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id,
                document_type_name,
                property_agreement,
                lookup,
            } => Some(DocumentReferenceDeclaration {
                contract_id: *contract_id,
                document_type_name,
                property_agreement,
                permanent: false,
                lookup: Some(lookup),
                in_list: None,
            }),
            DocumentPropertyReferenceTarget::ListElement(reference) => {
                Some(DocumentReferenceDeclaration {
                    contract_id: reference.contract_id,
                    document_type_name: &reference.document_type_name,
                    property_agreement: &reference.property_agreement,
                    permanent: true,
                    lookup: None,
                    in_list: Some(&reference.in_list),
                })
            }
            DocumentPropertyReferenceTarget::Identity
            | DocumentPropertyReferenceTarget::Contract { .. }
            | DocumentPropertyReferenceTarget::Token
            | DocumentPropertyReferenceTarget::IdentityPublicKey { .. }
            | DocumentPropertyReferenceTarget::AnyOf(_)
            | DocumentPropertyReferenceTarget::AllOf(_) => None,
        }
    }

    /// The declaration of a `listElement` reference; `None` for every other
    /// target.
    pub fn as_list_element_reference(&self) -> Option<&ListElementReference> {
        match self {
            DocumentPropertyReferenceTarget::ListElement(reference) => Some(reference),
            _ => None,
        }
    }

    /// The combinator and operands of a reference expression, `None` for a
    /// single target (a leaf).
    pub fn combinator(&self) -> Option<(ReferenceCombinator, &ReferenceOperands)> {
        match self {
            DocumentPropertyReferenceTarget::AnyOf(operands) => {
                Some((ReferenceCombinator::AnyOf, operands))
            }
            DocumentPropertyReferenceTarget::AllOf(operands) => {
                Some((ReferenceCombinator::AllOf, operands))
            }
            _ => None,
        }
    }

    /// The single targets this declaration is made of, in declared order,
    /// depth first: the leaves of a reference expression, or the declaration
    /// itself. Code that checks every declaration (registration, the lookup
    /// sources) walks these, so a leaf of an expression is checked exactly as
    /// the same target declared alone. A leaf appearing twice is listed twice.
    pub fn leaves(&self) -> Vec<&DocumentPropertyReferenceTarget> {
        self.leaves_with_paths()
            .into_iter()
            .map(|(_, leaf)| leaf)
            .collect()
    }

    /// [`Self::leaves`] with where each sits in the expression, as an error
    /// names it: `anyOf[1].allOf[0]`, the empty string for a single target.
    pub fn leaves_with_paths(&self) -> Vec<(String, &DocumentPropertyReferenceTarget)> {
        fn walk<'a>(
            target: &'a DocumentPropertyReferenceTarget,
            path: String,
            leaves: &mut Vec<(String, &'a DocumentPropertyReferenceTarget)>,
        ) {
            match target.combinator() {
                None => leaves.push((path, target)),
                Some((combinator, operands)) => {
                    for (index, operand) in operands.operands().iter().enumerate() {
                        let separator = if path.is_empty() { "" } else { "." };
                        walk(
                            operand,
                            format!("{path}{separator}{}[{index}]", combinator.wire_name()),
                            leaves,
                        );
                    }
                }
            }
        }
        let mut leaves = Vec::new();
        walk(self, String::new(), &mut leaves);
        leaves
    }

    /// How many combinators the deepest leaf sits under: 0 for a single
    /// target, 1 for a flat `anyOf` or `allOf`.
    pub fn expression_depth(&self) -> usize {
        match self.combinator() {
            None => 0,
            Some((_, operands)) => {
                1 + operands
                    .operands()
                    .iter()
                    .map(DocumentPropertyReferenceTarget::expression_depth)
                    .max()
                    .unwrap_or(0)
            }
        }
    }
}

/// A property's `refersTo` declaration and what holds the reference: the
/// property's own value, every element of a typed array of identifiers, or,
/// for a key reference declared on the key id itself, the key id. Returned
/// by [`DocumentPropertyType::reference`], which is how the registration and
/// write-time validators, the per-document reference bound and the client
/// bindings enumerate a document type's references, so no kind can be
/// skipped by a caller matching one property type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PropertyReference<'a> {
    /// An identifier property: its value is the referenced id.
    Value(&'a DocumentPropertyReferenceTarget),
    /// A typed array whose elements are identifiers carrying `refersTo`
    /// (declared on its `items`): each element is a referenced id, all of
    /// them to `target`, at most `max_items` of them. Never an
    /// [`DocumentPropertyReferenceTarget::IdentityPublicKey`], which the
    /// parser refuses on an element.
    Elements {
        target: &'a DocumentPropertyReferenceTarget,
        max_items: u16,
    },
    /// A key id property carrying an `identityPublicKey` declaration that
    /// names whose key it is ([`DocumentPropertyType::KeyIdWithReference`]).
    KeyId(&'a KeyIdReference),
}

impl<'a> PropertyReference<'a> {
    /// The declaration of an identifier or element reference; `None` for a
    /// key reference on the key id, which has no identifier target.
    pub fn target(&self) -> Option<&'a DocumentPropertyReferenceTarget> {
        match self {
            PropertyReference::Value(target) | PropertyReference::Elements { target, .. } => {
                Some(target)
            }
            PropertyReference::KeyId(_) => None,
        }
    }

    /// How many references one document can carry through this declaration,
    /// each a billed state read when the document is written: `max_items`
    /// for a typed array, one otherwise, times the number of leaves of a
    /// reference expression, every one of which may be read for one value.
    pub fn max_references(&self) -> u32 {
        let values = match self {
            PropertyReference::Elements { max_items, .. } => u32::from(*max_items),
            PropertyReference::Value(_) | PropertyReference::KeyId(_) => 1,
        };
        let leaves = self.target().map_or(1, |target| target.leaves().len());
        values.saturating_mul(u32::try_from(leaves).unwrap_or(u32::MAX))
    }
}

/// Where a reference declaration of a document type sits, and so where the
/// value it checks comes from. Paired with each declaration by
/// [`DocumentTypeRef::reference_declarations`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReferenceHolder<'a> {
    /// The document type's `ownerRefersTo`: the value is the document's
    /// `$ownerId`, the writer.
    Owner,
    /// The document type's `creatorRefersTo`: the value is the document's
    /// `$creatorId`, its creator, which never changes.
    Creator,
    /// A property, by its flattened path: the value is the property's, or each
    /// element's for a typed array.
    Property(&'a str),
}

impl<'a> ReferenceHolder<'a> {
    /// The path the reference errors name the declaration by: the property's,
    /// `$ownerId` for the owner reference or `$creatorId` for the creator one.
    pub fn path(&self) -> &'a str {
        match self {
            ReferenceHolder::Owner => OWNER_ID,
            ReferenceHolder::Creator => CREATOR_ID,
            ReferenceHolder::Property(path) => path,
        }
    }

    /// How contract structure errors name the declaration.
    pub fn describe(&self) -> String {
        match self {
            ReferenceHolder::Owner => property_names::OWNER_REFERS_TO.to_string(),
            ReferenceHolder::Creator => property_names::CREATOR_REFERS_TO.to_string(),
            ReferenceHolder::Property(path) => format!("property \"{path}\" refersTo"),
        }
    }
}

impl<'a> DocumentTypeRef<'a> {
    /// Every reference declaration of the document type with its holder: the
    /// type's `ownerRefersTo` and `creatorRefersTo` first (a type declares at
    /// most one of them), then each property's own
    /// ([`DocumentPropertyType::reference`]) in schema order. This is how the
    /// registration and write-time validators, the per-document reference
    /// bound and the client bindings enumerate a type's references, so none of
    /// them can skip a holder.
    pub fn reference_declarations(
        self,
    ) -> impl Iterator<Item = (ReferenceHolder<'a>, PropertyReference<'a>)> {
        let (owner_reference, creator_reference, flattened_properties) = match self {
            DocumentTypeRef::V0(v0) => (None, None, &v0.flattened_properties),
            DocumentTypeRef::V1(v1) => (None, None, &v1.flattened_properties),
            DocumentTypeRef::V2(v2) => (
                v2.owner_reference.as_ref(),
                v2.creator_reference.as_ref(),
                &v2.flattened_properties,
            ),
        };
        owner_reference
            .map(|target| (ReferenceHolder::Owner, PropertyReference::Value(target)))
            .into_iter()
            .chain(
                creator_reference
                    .map(|target| (ReferenceHolder::Creator, PropertyReference::Value(target))),
            )
            .chain(flattened_properties.iter().filter_map(|(path, property)| {
                property
                    .property_type
                    .reference()
                    .map(|reference| (ReferenceHolder::Property(path.as_str()), reference))
            }))
    }
}

/// The system properties of a referenced document that the referenced side
/// of a `propertyAgreement` pair may name, next to the referenced document
/// type's schema properties: `$ownerId`, the current owner (which follows
/// the document through transfers), `$creatorId`, the original creator
/// (set once, and only recorded by transferable or tradeable document types
/// of a format-1 contract), and `$id`, the document's own id, which never
/// changes (a `listElement` reference finds its document by such a pair).
/// All are identifiers, so the referring side must be an identifier
/// property. The referring side is a schema property or the writer's own
/// `$ownerId`, see [`REFERRING_SYSTEM_AGREEMENT_PROPERTIES`].
pub const REFERENCED_SYSTEM_AGREEMENT_PROPERTIES: [&str; 3] = [OWNER_ID, CREATOR_ID, ID];

/// Whether `name` is one of [`REFERENCED_SYSTEM_AGREEMENT_PROPERTIES`].
pub fn is_referenced_system_agreement_property(name: &str) -> bool {
    REFERENCED_SYSTEM_AGREEMENT_PROPERTIES.contains(&name)
}

/// The system properties of the REFERRING document that the referring side
/// of a `propertyAgreement` pair may name, next to the declaring document
/// type's schema properties: only `$ownerId`, the writer. Such a pair is a
/// write gate: consensus refuses a create or replace unless the writer's id
/// equals the referenced side, so the referenced document's owner (or its
/// creator, or a named identifier) is the only identity that may write
/// referring documents. It is checked on every create and on EVERY replace
/// of the referring document, not only when the reference changes, since
/// either document may have been transferred in between; a transfer itself
/// is not re-checked, so on a transferable referring type the gate governs
/// writing, not holding. The writer's id lives on the transition rather
/// than in the document data, which is why it is threaded into write-time
/// validation separately.
pub const REFERRING_SYSTEM_AGREEMENT_PROPERTIES: [&str; 1] = [OWNER_ID];

/// Whether `name` is one of [`REFERRING_SYSTEM_AGREEMENT_PROPERTIES`].
pub fn is_referring_system_agreement_property(name: &str) -> bool {
    REFERRING_SYSTEM_AGREEMENT_PROPERTIES.contains(&name)
}

/// Whether the property at the dotted `path` of `document_type`, or an object
/// around it, is transient: either way its value is never stored.
/// `transient_fields()` holds the paths as declared, so a leaf of a transient
/// object is found only through the object's path, a prefix of its own.
pub fn is_transient(document_type: DocumentTypeRef, path: &str) -> bool {
    let transient_fields = document_type.transient_fields();
    path.match_indices('.')
        .map(|(end, _)| &path[..end])
        .chain(std::iter::once(path))
        .any(|prefix| transient_fields.contains(prefix))
}

impl std::fmt::Display for DocumentPropertyReferenceTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentPropertyReferenceTarget::Identity => write!(f, "identity"),
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements,
            } => {
                write!(f, "contract")?;
                if let Some(moderation) = contract_requirements.moderation {
                    write!(f, " with {}", moderation.describe())?;
                }
                if let Some(seconds) = contract_requirements.minimum_age_seconds {
                    write!(f, " at least {seconds} seconds old")?;
                }
                if let Some(seconds) = contract_requirements.minimum_seconds_since_update {
                    write!(f, " unchanged for at least {seconds} seconds")?;
                }
                match contract_requirements.owner {
                    Some(ContractReferenceOwner::Writer) => write!(f, " owned by the writer")?,
                    Some(ContractReferenceOwner::Other) => write!(f, " not owned by the writer")?,
                    None => {}
                }
                if contract_requirements.readonly == Some(true) {
                    write!(f, " read-only")?;
                }
                if contract_requirements.keeps_history == Some(true) {
                    write!(f, " keeping history")?;
                }
                match contract_requirements.owner_protected {
                    Some(true) => write!(f, " with the owner protected")?,
                    Some(false) => write!(f, " with the owner unprotected")?,
                    None => {}
                }
                Ok(())
            }
            DocumentPropertyReferenceTarget::Token => write!(f, "token"),
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name,
                ..
            } => write_document_reference(f, "permanent", *contract_id, document_type_name, None),
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id,
                document_type_name,
                lookup,
                ..
            } => write_document_reference(
                f,
                "permanent",
                *contract_id,
                document_type_name,
                Some(lookup),
            ),
            DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id,
                document_type_name,
                lookup,
                ..
            } => write_document_reference(
                f,
                "deletable",
                *contract_id,
                document_type_name,
                Some(lookup),
            ),
            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property,
                key_requirements,
            } => {
                write!(f, "identity public key (key id property {key_id_property})")?;
                if let Some(purpose) = key_requirements.purpose {
                    write!(f, " with purpose {}", purpose.wire_name())?;
                }
                if let Some(document_type_name) = &key_requirements.bound_to {
                    write!(f, " bound to document type {document_type_name}")?;
                }
                Ok(())
            }
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id,
                document_type_name,
                ..
            } => write_document_reference(f, "deletable", *contract_id, document_type_name, None),
            DocumentPropertyReferenceTarget::ListElement(reference) => reference.fmt(f),
            DocumentPropertyReferenceTarget::AnyOf(operands)
            | DocumentPropertyReferenceTarget::AllOf(operands) => {
                let (name, joiner) = match self {
                    DocumentPropertyReferenceTarget::AnyOf(_) => ("any of", " or "),
                    _ => ("all of", " and "),
                };
                write!(f, "{name} (")?;
                for (index, operand) in operands.operands().iter().enumerate() {
                    if index > 0 {
                        write!(f, "{joiner}")?;
                    }
                    write!(f, "{operand}")?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Whose key a `refersTo: identityPublicKey` declaration on a KEY ID property
/// names: its `identityProperty`. The declaring property carries the key id
/// (a `u32`, so an integer property with `minimum` 0 and `maximum`
/// 4294967295) and this names the identity the key belongs to: the document's
/// owner, its creator, or an identifier property of the same document type.
/// It is the inverse of [`DocumentPropertyReferenceTarget::IdentityPublicKey`],
/// where the declaring property carries the identity id and `keyIdProperty`
/// names the sibling carrying the key id; a declaration is one form or the
/// other, never both.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum KeyReferenceIdentityProperty {
    /// `"$ownerId"`: the writer's own identity. The document's owner signs
    /// the transition, which already proved the identity exists, so the
    /// reference costs the key fetch alone. The owner can change through a
    /// transfer or a purchase, so every replace re-validates the reference.
    #[serde(rename = "$ownerId")]
    OwnerId,
    /// `"$creatorId"`: the identity that created the document, the writer
    /// of its create and the stored creator id after that. Only a document
    /// type that records creator ids (a transferable or tradeable type of a
    /// format-1 contract) may declare it, checked at contract registration.
    /// The creator never changes, so a replace re-validates the reference
    /// when the key id changed.
    #[serde(rename = "$creatorId")]
    CreatorId,
    /// An identifier property of the same document type (a dotted path when
    /// nested) whose value is the identity; it must exist, be an identifier
    /// and not carry an `identityPublicKey` reference of its own, checked at
    /// contract registration. The identity is read from the document, so a
    /// replace re-validates the reference when the key id or that property
    /// changed, and a key id set while the property is not is refused.
    Property(String),
}

impl KeyReferenceIdentityProperty {
    /// The system `identityProperty` values the schema admits, as spelled
    /// there; every other admitted value is a property path.
    pub const SYSTEM_WIRE_NAMES: [&'static str; 2] = [OWNER_ID, CREATOR_ID];

    /// The value for its schema spelling: a system name, or a property path
    /// of 1 to 256 characters that does not start with `$`. `None` for a
    /// spelling the schema does not admit.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            OWNER_ID => Some(KeyReferenceIdentityProperty::OwnerId),
            CREATOR_ID => Some(KeyReferenceIdentityProperty::CreatorId),
            path if path.starts_with('$') || path.is_empty() || path.len() > 256 => None,
            path => Some(KeyReferenceIdentityProperty::Property(path.to_string())),
        }
    }

    /// The schema spelling.
    pub fn as_str(&self) -> &str {
        match self {
            KeyReferenceIdentityProperty::OwnerId => OWNER_ID,
            KeyReferenceIdentityProperty::CreatorId => CREATOR_ID,
            KeyReferenceIdentityProperty::Property(path) => path,
        }
    }
}

impl std::fmt::Display for KeyReferenceIdentityProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `refersTo: identityPublicKey` declaration on a KEY ID property: whose
/// key the value is, and what that key must be beyond existing and not
/// being disabled, the same [`IdentityKeyReferenceRequirements`] the
/// identifier form takes, checked the same way.
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct KeyIdReference {
    pub identity_property: KeyReferenceIdentityProperty,
    #[serde(
        default,
        skip_serializing_if = "IdentityKeyReferenceRequirements::is_empty"
    )]
    pub key_requirements: IdentityKeyReferenceRequirements,
}

impl KeyIdReference {
    /// A declaration requiring nothing of the key beyond existing.
    pub fn new(identity_property: KeyReferenceIdentityProperty) -> Self {
        KeyIdReference {
            identity_property,
            key_requirements: IdentityKeyReferenceRequirements::default(),
        }
    }
}

/// How the two document reference targets read: the kind, the contract, the
/// document type and, for a lookup, the unique index the document is found
/// through.
fn write_document_reference(
    f: &mut std::fmt::Formatter<'_>,
    kind: &str,
    contract_id: Option<Identifier>,
    document_type_name: &str,
    lookup: Option<&DocumentReferenceLookup>,
) -> std::fmt::Result {
    match contract_id {
        Some(contract_id) => write!(
            f,
            "{kind} document (contract {contract_id}, document type {document_type_name}"
        )?,
        None => write!(
            f,
            "{kind} document (own contract, document type {document_type_name}"
        )?,
    }
    if let Some(lookup) = lookup {
        write!(f, ", found through unique index {}", lookup.index)?;
    }
    write!(f, ")")
}

// @append_only
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum DocumentPropertyType {
    U128,
    I128,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
    F64,
    String(StringPropertySizes),
    ByteArray(ByteArrayPropertySizes),
    Identifier,
    Boolean,
    Date,
    Object(IndexMap<String, DocumentProperty>),
    /// A list of elements of one type with no element count bounds. The
    /// schema parser never produces it: a typed array property parses to
    /// [`DocumentPropertyType::TypedArray`], which shares its encoding.
    Array(ArrayItemType),
    VariableTypeArray(Vec<ArrayItemType>),
    IdentifierWithReference(DocumentPropertyReferenceTarget),
    /// A typed array property (`type: "array"` with an `items` element
    /// schema), from protocol version 14: the element type with the
    /// `minItems` / `maxItems` element count bounds and `uniqueItems`. Stored
    /// inline like [`DocumentPropertyType::Array`], a varint element count
    /// followed by the elements.
    TypedArray(TypedArrayProperty),
    /// A `u32` key id carrying a `refersTo: identityPublicKey` declaration
    /// with `identityProperty`: the value is the id of a key of the named
    /// identity, which must exist, not be disabled and meet the declared
    /// requirements when the document is written. Sized, encoded and queried
    /// exactly as [`Self::U32`].
    KeyIdWithReference(KeyIdReference),
}

impl DocumentPropertyType {
    #[deprecated = "this method is missing required information to create a type. Use TryFrom<&Value> instead."]
    pub fn try_from_name(name: &str) -> Result<Self, DataContractError> {
        match name {
            "u128" => Ok(DocumentPropertyType::U128),
            "i128" => Ok(DocumentPropertyType::I128),
            "u64" => Ok(DocumentPropertyType::U64),
            "i64" | "integer" => Ok(DocumentPropertyType::I64),
            "u32" => Ok(DocumentPropertyType::U32),
            "i32" => Ok(DocumentPropertyType::I32),
            "u16" => Ok(DocumentPropertyType::U16),
            "i16" => Ok(DocumentPropertyType::I16),
            "u8" => Ok(DocumentPropertyType::U8),
            "i8" => Ok(DocumentPropertyType::I8),
            "f64" | "number" => Ok(DocumentPropertyType::F64),
            "boolean" => Ok(DocumentPropertyType::Boolean),
            "date" => Ok(DocumentPropertyType::Date),
            "identifier" => Ok(DocumentPropertyType::Identifier),
            "string" => Ok(DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
                max_bytes: None,
            })),
            "byteArray" => Ok(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: None,
                max_size: None,
            })),
            "object" => Ok(DocumentPropertyType::Object(IndexMap::new())),
            "array" => Err(DataContractError::ValueWrongType(
                "array type needs to specify the inner type".to_string(),
            )),
            "variableTypeArray" => Ok(DocumentPropertyType::VariableTypeArray(Vec::new())),
            name => Err(DataContractError::ValueWrongType(format!(
                "invalid type {}",
                name
            ))),
        }
    }

    /// The kind of value this type holds, for the rules that compare a value of
    /// one property with a value of another (`propertyAgreement` pairs,
    /// `lookup` key parts): two types of the same kind can hold equal values.
    /// Sizes and other constraints do not count, and an identifier, or a `u32`
    /// key id, is one kind whether or not it carries its own reference.
    pub fn value_kind(&self) -> std::mem::Discriminant<DocumentPropertyType> {
        match self {
            DocumentPropertyType::IdentifierWithReference(_) => {
                std::mem::discriminant(&DocumentPropertyType::Identifier)
            }
            DocumentPropertyType::KeyIdWithReference(_) => {
                std::mem::discriminant(&DocumentPropertyType::U32)
            }
            other => std::mem::discriminant(other),
        }
    }

    pub fn name(&self) -> String {
        match self {
            DocumentPropertyType::U128 => "u128".to_string(),
            DocumentPropertyType::I128 => "i128".to_string(),
            DocumentPropertyType::U64 => "u64".to_string(),
            DocumentPropertyType::I64 => "i64".to_string(),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                "u32".to_string()
            }
            DocumentPropertyType::I32 => "i32".to_string(),
            DocumentPropertyType::U16 => "u16".to_string(),
            DocumentPropertyType::I16 => "i16".to_string(),
            DocumentPropertyType::U8 => "u8".to_string(),
            DocumentPropertyType::I8 => "i8".to_string(),
            DocumentPropertyType::F64 => "f64".to_string(),
            DocumentPropertyType::String(_) => "string".to_string(),
            DocumentPropertyType::ByteArray(_) => "byteArray".to_string(),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                "identifier".to_string()
            }
            DocumentPropertyType::Boolean => "boolean".to_string(),
            DocumentPropertyType::Date => "date".to_string(),
            DocumentPropertyType::Object(_) => "object".to_string(),
            DocumentPropertyType::Array(_) | DocumentPropertyType::TypedArray(_) => {
                "array".to_string()
            }
            DocumentPropertyType::VariableTypeArray(_) => "variableTypeArray".to_string(),
        }
    }

    /// The `refersTo` declaration this property carries, on its own value
    /// (an identifier property), on every element (a typed array whose
    /// `items` declare it) or on the key id (a key reference naming whose
    /// key it is); `None` for a property without one.
    pub fn reference(&self) -> Option<PropertyReference<'_>> {
        match self {
            DocumentPropertyType::IdentifierWithReference(target) => {
                Some(PropertyReference::Value(target))
            }
            DocumentPropertyType::TypedArray(typed_array) => match typed_array.item_type.as_ref() {
                DocumentPropertyType::IdentifierWithReference(target) => {
                    Some(PropertyReference::Elements {
                        target,
                        max_items: typed_array.max_items,
                    })
                }
                _ => None,
            },
            DocumentPropertyType::KeyIdWithReference(reference) => {
                Some(PropertyReference::KeyId(reference))
            }
            _ => None,
        }
    }

    /// How a value of this scalar type is laid out in a stored document, in
    /// the words a contract update error uses. The layout of two scalar types
    /// is the same exactly when they give the same answer. The schema chooses
    /// it in two places: an integer is stored at the width and signedness of
    /// its type, which its `minimum`, `maximum` or `enum` pick, and a byte
    /// array is stored raw when its `minItems` and `maxItems` pin one size and
    /// length-prefixed otherwise. Every other scalar is laid out the same
    /// whatever its schema says, so it answers with its name.
    pub(crate) fn stored_encoding(&self) -> String {
        match self {
            DocumentPropertyType::ByteArray(sizes) => match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max => format!("a fixed {min}-byte array"),
                _ => "a length-prefixed byte array".to_string(),
            },
            other => other.name(),
        }
    }

    pub fn min_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::U128 => Some(16),
            DocumentPropertyType::I128 => Some(16),
            DocumentPropertyType::U64 => Some(8),
            DocumentPropertyType::I64 => Some(8),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => Some(4),
            DocumentPropertyType::I32 => Some(4),
            DocumentPropertyType::U16 => Some(2),
            DocumentPropertyType::I16 => Some(2),
            DocumentPropertyType::U8 => Some(1),
            DocumentPropertyType::I8 => Some(1),
            DocumentPropertyType::F64 => Some(8),
            DocumentPropertyType::String(sizes) => match sizes.min_length {
                None => Some(0),
                Some(size) => Some(size),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.min_size {
                None => Some(0),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.min_size())
                .sum(),
            DocumentPropertyType::Array(_) | DocumentPropertyType::TypedArray(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
        }
    }

    pub fn min_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        match self {
            DocumentPropertyType::U128 => Ok(Some(16)),
            DocumentPropertyType::I128 => Ok(Some(16)),
            DocumentPropertyType::U64 => Ok(Some(8)),
            DocumentPropertyType::I64 => Ok(Some(8)),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => Ok(Some(4)),
            DocumentPropertyType::I32 => Ok(Some(4)),
            DocumentPropertyType::U16 => Ok(Some(2)),
            DocumentPropertyType::I16 => Ok(Some(2)),
            DocumentPropertyType::U8 => Ok(Some(1)),
            DocumentPropertyType::I8 => Ok(Some(1)),
            DocumentPropertyType::F64 => Ok(Some(8)),
            DocumentPropertyType::String(sizes) => match sizes.min_length {
                None => Ok(Some(0)),
                Some(size) => {
                    if platform_version.protocol_version > 8 {
                        match size.checked_mul(4) {
                            Some(mul) => Ok(Some(mul)),
                            None => Err(ProtocolError::Overflow("min_byte_size overflow")),
                        }
                    } else {
                        Ok(Some(size.wrapping_mul(4)))
                    }
                }
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.min_size {
                None => Ok(Some(0)),
                Some(size) => Ok(Some(size)),
            },
            DocumentPropertyType::Boolean => Ok(Some(1)),
            DocumentPropertyType::Date => Ok(Some(8)),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.min_byte_size(platform_version))
                .sum(),
            DocumentPropertyType::Array(_) => Ok(None),
            DocumentPropertyType::VariableTypeArray(_) => Ok(None),
            DocumentPropertyType::TypedArray(typed_array) => {
                typed_array.min_encoded_size(platform_version).map(Some)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Some(32))
            }
        }
    }

    pub fn max_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        match self {
            DocumentPropertyType::U128 => Ok(Some(16)),
            DocumentPropertyType::I128 => Ok(Some(16)),
            DocumentPropertyType::U64 => Ok(Some(8)),
            DocumentPropertyType::I64 => Ok(Some(8)),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => Ok(Some(4)),
            DocumentPropertyType::I32 => Ok(Some(4)),
            DocumentPropertyType::U16 => Ok(Some(2)),
            DocumentPropertyType::I16 => Ok(Some(2)),
            DocumentPropertyType::U8 => Ok(Some(1)),
            DocumentPropertyType::I8 => Ok(Some(1)),
            DocumentPropertyType::F64 => Ok(Some(8)),
            // A declared `maxBytes` bounds the value directly, below the four bytes a
            // character may take; only strings parsed from protocol version 14 carry one
            DocumentPropertyType::String(StringPropertySizes {
                max_length,
                max_bytes: Some(max_bytes),
                ..
            }) => Ok(Some(max_length.map_or(*max_bytes, |length| {
                length.saturating_mul(4).min(*max_bytes)
            }))),
            DocumentPropertyType::String(sizes) => match sizes.max_length {
                None => Ok(Some(u16::MAX)),
                Some(size) => {
                    if platform_version.protocol_version > 8 {
                        match size.checked_mul(4) {
                            Some(mul) => Ok(Some(mul)),
                            None => Err(ProtocolError::Overflow("max_byte_size overflow")),
                        }
                    } else {
                        Ok(Some(size.wrapping_mul(4)))
                    }
                }
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.max_size {
                None => Ok(Some(u16::MAX)),
                Some(size) => Ok(Some(size)),
            },
            DocumentPropertyType::Boolean => Ok(Some(1)),
            DocumentPropertyType::Date => Ok(Some(8)),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.max_byte_size(platform_version))
                .sum(),
            DocumentPropertyType::Array(_) => Ok(None),
            DocumentPropertyType::VariableTypeArray(_) => Ok(None),
            DocumentPropertyType::TypedArray(typed_array) => {
                typed_array.max_encoded_size(platform_version).map(Some)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Some(32))
            }
        }
    }

    pub fn max_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::U128 => Some(16),
            DocumentPropertyType::I128 => Some(16),
            DocumentPropertyType::U64 => Some(8),
            DocumentPropertyType::I64 => Some(8),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => Some(4),
            DocumentPropertyType::I32 => Some(4),
            DocumentPropertyType::U16 => Some(2),
            DocumentPropertyType::I16 => Some(2),
            DocumentPropertyType::U8 => Some(1),
            DocumentPropertyType::I8 => Some(1),
            DocumentPropertyType::F64 => Some(8),
            // No more characters than `maxBytes` fit, since each takes at least a byte
            DocumentPropertyType::String(sizes) => match (sizes.max_length, sizes.max_bytes) {
                (None, None) => Some(16383),
                (Some(size), None) => Some(size),
                (None, Some(max_bytes)) => Some(max_bytes.min(16383)),
                (Some(size), Some(max_bytes)) => Some(size.min(max_bytes)),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.max_size {
                None => Some(u16::MAX),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.max_size())
                .sum(),
            DocumentPropertyType::Array(_) | DocumentPropertyType::TypedArray(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
        }
    }

    /// The width every value of this type encodes to as a tree key
    /// ([`Self::encode_value_for_tree_keys`]), when that width is fixed:
    /// the integer, float, boolean, date and identifier encodings, and a
    /// byte array whose bounds pin one size. `None` for strings (their
    /// bound counts characters), unbounded or variable-size byte arrays,
    /// objects and arrays. The one width the composite indexOnly terminal
    /// rules, the walkers and synthesis all split member keys by.
    pub fn fixed_tree_key_width(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::U128 | DocumentPropertyType::I128 => Some(16),
            DocumentPropertyType::U64
            | DocumentPropertyType::I64
            | DocumentPropertyType::F64
            | DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::U32
            | DocumentPropertyType::KeyIdWithReference(_)
            | DocumentPropertyType::I32 => Some(4),
            DocumentPropertyType::U16 | DocumentPropertyType::I16 => Some(2),
            DocumentPropertyType::U8 | DocumentPropertyType::I8 | DocumentPropertyType::Boolean => {
                Some(1)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
            DocumentPropertyType::ByteArray(sizes) => match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max && min > 0 => Some(min),
                _ => None,
            },
            DocumentPropertyType::String(_)
            | DocumentPropertyType::Object(_)
            | DocumentPropertyType::Array(_)
            | DocumentPropertyType::VariableTypeArray(_)
            | DocumentPropertyType::TypedArray(_) => None,
        }
    }

    /// The middle size rounded down halfway between min and max size
    pub fn middle_size(&self, platform_version: &PlatformVersion) -> Option<u16> {
        let min_size = self.min_size()?;
        let max_size = self.max_size()?;
        if platform_version.protocol_version > 8 {
            Some(((min_size as u32 + max_size as u32) / 2) as u16)
        } else {
            Some(min_size.wrapping_add(max_size) / 2)
        }
    }

    /// The middle size rounded up halfway between min and max size
    pub fn middle_size_ceil(&self, platform_version: &PlatformVersion) -> Option<u16> {
        let min_size = self.min_size()?;
        let max_size = self.max_size()?;
        if platform_version.protocol_version > 8 {
            Some(((min_size as u32 + max_size as u32).div_ceil(2)) as u16)
        } else {
            Some(min_size.wrapping_add(max_size).wrapping_add(1) / 2)
        }
    }

    /// The middle size rounded down halfway between min and max byte size
    pub fn middle_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        let Some(min_size) = self.min_byte_size(platform_version)? else {
            return Ok(None);
        };
        let Some(max_size) = self.max_byte_size(platform_version)? else {
            return Ok(None);
        };
        if platform_version.protocol_version > 8 {
            Ok(Some(((min_size as u32 + max_size as u32) / 2) as u16))
        } else {
            Ok(Some(min_size.wrapping_add(max_size) / 2))
        }
    }

    /// The middle size rounded up halfway between min and max byte size
    pub fn middle_byte_size_ceil(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        let Some(min_size) = self.min_byte_size(platform_version)? else {
            return Ok(None);
        };
        let Some(max_size) = self.max_byte_size(platform_version)? else {
            return Ok(None);
        };
        if platform_version.protocol_version > 8 {
            Ok(Some(
                ((min_size as u32 + max_size as u32).div_ceil(2)) as u16,
            ))
        } else {
            Ok(Some(min_size.wrapping_add(max_size).wrapping_add(1) / 2))
        }
    }

    pub fn random_size(&self, rng: &mut StdRng) -> u16 {
        let min_size = self.min_size().unwrap_or_default();
        let max_size = self.max_size().unwrap_or_default();
        rng.gen_range(min_size..=max_size)
    }

    pub fn random_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                Value::U32(rng.gen::<u32>())
            }
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.random_size(rng);
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.random_size(rng);
                if self.min_size() == self.max_size() {
                    match size {
                        20 => Value::Bytes20(rng.gen()),
                        32 => Value::Bytes32(rng.gen()),
                        36 => Value::Bytes36(
                            rng.sample_iter(Standard)
                                .take(size as usize)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap(),
                        ),
                        _ => Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect()),
                    }
                } else {
                    Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
                }
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .filter_map(|(string, field_type)| {
                        if field_type.required {
                            Some((
                                Value::Text(string.clone()),
                                field_type.property_type.random_value(rng),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
            DocumentPropertyType::TypedArray(typed_array) => typed_array.random_value(rng),
        }
    }

    pub fn random_sub_filled_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                Value::U32(rng.gen::<u32>())
            }
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.min_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.min_size().unwrap();
                Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.property_type.random_sub_filled_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
            DocumentPropertyType::TypedArray(typed_array) => {
                typed_array.random_sub_filled_value(rng)
            }
        }
    }

    pub fn random_filled_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                Value::U32(rng.gen::<u32>())
            }
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.max_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.max_size().unwrap();
                Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.property_type.random_filled_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
            DocumentPropertyType::TypedArray(typed_array) => typed_array.random_filled_value(rng),
        }
    }

    /// Reads exactly `len` bytes. The length comes from the (possibly
    /// untrusted) serialized document itself, so it must never size an
    /// allocation: the buffer grows only as bytes actually arrive, and a
    /// prefix that claims more than the document holds is rejected once the
    /// input runs out.
    fn read_exact_bounded(
        buf: &mut BufReader<&[u8]>,
        len: usize,
        what: &str,
    ) -> Result<Vec<u8>, DataContractError> {
        let mut value = Vec::new();
        buf.by_ref()
            .take(len as u64)
            .read_to_end(&mut value)
            .map_err(|_| {
                DataContractError::CorruptedSerialization(format!(
                    "error reading {what} of length {len} from serialized document"
                ))
            })?;
        if value.len() != len {
            return Err(DataContractError::CorruptedSerialization(format!(
                "{what} declares {len} bytes but only {} remain in the serialized document",
                value.len()
            )));
        }
        Ok(value)
    }

    fn read_varint_value(buf: &mut BufReader<&[u8]>) -> Result<Vec<u8>, DataContractError> {
        let bytes: usize = buf.read_varint().map_err(|_| {
            DataContractError::CorruptedSerialization(
                "error reading varint length from serialized document".to_string(),
            )
        })?;
        Self::read_exact_bounded(buf, bytes, "varint value")
    }

    /// Reads an optional value from the buffer
    /// Returns an optional value, as well as a boolean to indicate if we have finished the buffer
    pub fn read_optionally_from(
        &self,
        buf: &mut BufReader<&[u8]>,
        required: bool,
    ) -> Result<(Option<Value>, bool), DataContractError> {
        if !required {
            let marker = buf.read_u8().ok();
            match marker {
                None => return Ok((None, true)), // we have no more data
                Some(0) => return Ok((None, false)),
                _ => {}
            }
        }
        match self {
            DocumentPropertyType::U128 => {
                let value = buf.read_u128::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u128 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U128(value)), false))
            }
            DocumentPropertyType::I128 => {
                let value = buf.read_i128::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i128 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I128(value)), false))
            }
            DocumentPropertyType::U64 => {
                let value = buf.read_u64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u64 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U64(value)), false))
            }
            DocumentPropertyType::I64 => {
                let value = buf.read_i64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i64 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I64(value)), false))
            }
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                let value = buf.read_u32::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u32 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U32(value)), false))
            }
            DocumentPropertyType::I32 => {
                let value = buf.read_i32::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i32 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I32(value)), false))
            }
            DocumentPropertyType::U16 => {
                let value = buf.read_u16::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u16 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U16(value)), false))
            }
            DocumentPropertyType::I16 => {
                let value = buf.read_i16::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i16 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I16(value)), false))
            }
            DocumentPropertyType::U8 => {
                let value = buf.read_u8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u8 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U8(value)), false))
            }
            DocumentPropertyType::I8 => {
                let value = buf.read_i8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i8 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I8(value)), false))
            }
            DocumentPropertyType::String(_) => {
                let bytes = Self::read_varint_value(buf)?;
                let string = String::from_utf8(bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Text(string)), false))
            }
            DocumentPropertyType::Date | DocumentPropertyType::F64 => {
                let date = buf.read_f64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading date/number from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Float(date)), false))
            }
            DocumentPropertyType::Boolean => {
                let value = buf.read_u8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading bool from serialized document".to_string(),
                    )
                })?;
                match value {
                    0 => Ok((Some(Value::Bool(false)), false)),
                    _ => Ok((Some(Value::Bool(true)), false)),
                }
            }
            DocumentPropertyType::ByteArray(sizes) => {
                match (sizes.min_size, sizes.max_size) {
                    (Some(min), Some(max)) if min == max => {
                        // if min == max, then we don't need a varint for the length
                        let len = min as usize;
                        // Schema-bounded (u16), so never an allocation hazard; routed
                        // through the bounded reader for uniformity while keeping the
                        // error variant this arm has always produced.
                        let bytes = Self::read_exact_bounded(buf, len, "fixed-size byte array")
                            .map_err(|_| {
                                DataContractError::DecodingContractError(DecodingError::new(
                                    format!(
                                        "expected to read {} bytes (min size for byte array)",
                                        len
                                    ),
                                ))
                            })?;
                        // To save space we use predefined types for most popular blob sizes
                        // so we don't need to store the size of the blob
                        match bytes.len() {
                            32 => Ok((Some(Value::Bytes32(bytes.try_into().unwrap())), false)),
                            20 => Ok((Some(Value::Bytes20(bytes.try_into().unwrap())), false)),
                            36 => Ok((Some(Value::Bytes36(bytes.try_into().unwrap())), false)),
                            _ => Ok((Some(Value::Bytes(bytes)), false)),
                        }
                    }
                    _ => {
                        let bytes = Self::read_varint_value(buf)?;

                        Ok((Some(Value::Bytes(bytes)), false))
                    }
                }
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let mut id = [0; 32];
                buf.read_exact(&mut id).map_err(|_| {
                    DataContractError::DecodingContractError(DecodingError::new(
                        "expected to read 32 bytes (identifier)".to_string(),
                    ))
                })?;
                //dbg!(hex::encode(&id));
                Ok((Some(Value::Identifier(id)), false))
            }

            DocumentPropertyType::Object(inner_fields) => {
                let object_byte_len: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint of object length".to_string(),
                    )
                })?;
                let object_bytes = Self::read_exact_bounded(buf, object_byte_len, "object")?;
                // Wrap the bytes in a BufReader
                let mut object_buf_reader = BufReader::new(&object_bytes[..]);
                let mut finished_buffer = false;
                let values = inner_fields
                    .iter()
                    .filter_map(|(key, field)| {
                        if finished_buffer {
                            return if field.required {
                                Some(Err(DataContractError::CorruptedSerialization(
                                    "required field after finished buffer in object".to_string(),
                                )))
                            } else {
                                None
                            };
                        }

                        let read_value = field
                            .property_type
                            .read_optionally_from(&mut object_buf_reader, field.required);

                        match read_value {
                            Ok(read_value) => {
                                finished_buffer |= read_value.1;
                                read_value
                                    .0
                                    .map(|read_value| Ok((Value::Text(key.clone()), read_value)))
                            }
                            Err(e) => Some(Err(e)),
                        }
                    })
                    .collect::<Result<Vec<(Value, Value)>, DataContractError>>()?;
                if values.is_empty() {
                    Ok((None, false))
                } else {
                    Ok((Some(Value::Map(values)), false))
                }
            }
            DocumentPropertyType::TypedArray(typed_array) => {
                Ok((Some(typed_array.read_from(buf)?), false))
            }
            DocumentPropertyType::Array(item_type) => {
                // Mirrors the encoding: a varint element count, then the
                // elements. The count comes from the serialized document, so
                // it never sizes an allocation; every element takes at least
                // one byte, so a count the document cannot hold fails once
                // the input runs out.
                let count: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint of array element count".to_string(),
                    )
                })?;
                let mut items = Vec::new();
                for _ in 0..count {
                    items.push(item_type.read_from(buf)?);
                }
                Ok((Some(Value::Array(items)), false))
            }
            DocumentPropertyType::VariableTypeArray(_) => Err(DataContractError::Unsupported(
                "serialization of variable type arrays not yet supported".to_string(),
            )),
        }
    }

    pub fn encode_value_with_size(
        &self,
        value: Value,
        required: bool,
    ) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::Date | DocumentPropertyType::F64 => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_f64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    // if the value wasn't required we need to add a byte to prove it existed
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U128 => {
                let value_as_u128: u128 =
                    value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u128.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I128 => {
                let value_as_i128: i128 =
                    value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i128.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U64 => {
                let value_as_u64: u64 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I64 => {
                let value_as_i64: i64 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                let value_as_u32: u32 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u32.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I32 => {
                let value_as_i32: i32 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i32.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U16 => {
                let value_as_u16: u16 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u16.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I16 => {
                let value_as_i16: i16 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i16.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U8 => {
                let value_as_u8: u8 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u8.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I8 => {
                let value_as_i8: i8 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i8.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::ByteArray(_) => {
                let mut bytes = value.into_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let mut bytes = value.into_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(&value))?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![2]) // 2 is false
                }
            }
            DocumentPropertyType::Object(inner_fields) => {
                if let Value::Map(map) = value {
                    let mut value_map =
                        Value::map_into_btree_string_map(map).map_err(ProtocolError::ValueError)?;
                    let mut r_vec = vec![];
                    inner_fields.iter().try_for_each(|(key, field)| {
                        if let Some(value) = value_map.remove(key) {
                            let mut serialized_value = field
                                .property_type
                                .encode_value_with_size(value, field.required)?;
                            r_vec.append(&mut serialized_value);
                            Ok(())
                        } else if field.required {
                            Err(ProtocolError::DataContractError(
                                DataContractError::MissingRequiredKey(
                                    "a required field is not present".to_string(),
                                ),
                            ))
                        } else {
                            // We don't have something that wasn't required
                            r_vec.push(0);
                            Ok(())
                        }
                    })?;
                    let mut len_prepended_vec = r_vec.len().encode_var_vec();
                    len_prepended_vec.append(&mut r_vec);
                    Ok(len_prepended_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::TypedArray(typed_array) => typed_array.encode_value_ref(&value),
            DocumentPropertyType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array.into_iter().try_for_each(|value| {
                        let mut serialized_value =
                            array_field_type.encode_value_with_size(value)?;
                        r_vec.append(&mut serialized_value);
                        Ok::<(), ProtocolError>(())
                    })?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::VariableTypeArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::Unsupported(
                    "serialization of variable type arrays not yet supported".to_string(),
                ),
            )),
        }
    }

    pub fn encode_value_ref_with_size(
        &self,
        value: &Value,
        required: bool,
    ) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                let value_as_text = value
                    .as_text()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            // TODO: Make the same as in https://github.com/dashpay/platform/blob/8d2a9e54d62b77581c44a15a09a2c61864af37d3/packages/rs-dpp/src/document/v0/serialize.rs#L161
            //  it must be u64 BE. Markers are wrong here as well
            DocumentPropertyType::Date => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_f64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    // if the value wasn't required we need to add a byte to prove it existed
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U128 => {
                let value_as_u128: u128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u128.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I128 => {
                let value_as_i128: i128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i128.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U64 => {
                let value_as_u64: u64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I64 => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                let value_as_u32: u32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u32.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I32 => {
                let value_as_i32: i32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i32.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U16 => {
                let value_as_u16: u16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u16.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I16 => {
                let value_as_i16: i16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i16.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U8 => {
                let value_as_u8: u8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u8.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I8 => {
                let value_as_i8: i8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i8.to_be_bytes().to_vec())
            }
            DocumentPropertyType::F64 => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                Ok(value_as_f64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::ByteArray(sizes) => match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max => Ok(value.to_binary_bytes()?),
                _ => {
                    let mut bytes = value.to_binary_bytes()?;

                    let mut r_vec = bytes.len().encode_var_vec();
                    r_vec.append(&mut bytes);
                    Ok(r_vec)
                }
            },
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(value.to_identifier_bytes()?)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 0 is false
                }
            }
            DocumentPropertyType::Object(inner_fields) => {
                let Some(value_map) = value.as_map() else {
                    return Err(get_field_type_matching_error(value).into());
                };
                let value_map = Value::map_ref_into_btree_string_map(value_map)?;
                let mut r_vec = vec![];
                inner_fields.iter().try_for_each(|(key, field)| {
                    if let Some(value) = value_map.get(key) {
                        if !field.required {
                            r_vec.push(1);
                        }
                        let value = field
                            .property_type
                            .encode_value_ref_with_size(value, field.required)?;
                        r_vec.extend(value.as_slice());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "a required field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // We don't have something that wasn't required
                        r_vec.push(0);
                        Ok(())
                    }
                })?;
                let mut len_prepended_vec = r_vec.len().encode_var_vec();
                len_prepended_vec.append(&mut r_vec);
                Ok(len_prepended_vec)
            }
            DocumentPropertyType::TypedArray(typed_array) => typed_array.encode_value_ref(value),
            DocumentPropertyType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array.iter().try_for_each(|value| {
                        let mut serialized_value =
                            array_field_type.encode_value_ref_with_size(value)?;
                        r_vec.append(&mut serialized_value);
                        Ok::<(), ProtocolError>(())
                    })?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(value).into())
                }
            }

            DocumentPropertyType::VariableTypeArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::Unsupported(
                    "serialization of arrays not yet supported".to_string(),
                ),
            )),
        }
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn encode_value_for_tree_keys(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                let value_as_text = value
                    .as_text()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                let vec = value_as_text.as_bytes().to_vec();
                if vec.is_empty() {
                    // we don't want to collide with the definition of an empty string
                    Ok(vec![0])
                } else {
                    Ok(vec)
                }
            }
            DocumentPropertyType::Date => Ok(DocumentPropertyType::encode_date_timestamp(
                value.to_integer().map_err(ProtocolError::ValueError)?,
            )),
            DocumentPropertyType::U128 => {
                let value_as_u128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u128(value_as_u128))
            }
            DocumentPropertyType::I128 => {
                let value_as_i128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i128(value_as_i128))
            }
            DocumentPropertyType::U64 => {
                let value_as_u64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u64(value_as_u64))
            }
            DocumentPropertyType::I64 => {
                let value_as_i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i64(value_as_i64))
            }
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                let value_as_u32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u32(value_as_u32))
            }
            DocumentPropertyType::I32 => {
                let value_as_i32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i32(value_as_i32))
            }
            DocumentPropertyType::U16 => {
                let value_as_u16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u16(value_as_u16))
            }
            DocumentPropertyType::I16 => {
                let value_as_i16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i16(value_as_i16))
            }
            DocumentPropertyType::U8 => {
                let value_as_u8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u8(value_as_u8))
            }
            DocumentPropertyType::I8 => {
                let value_as_i8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i8(value_as_i8))
            }
            DocumentPropertyType::F64 => Ok(Self::encode_float(
                value.to_float().map_err(ProtocolError::ValueError)?,
            )),
            DocumentPropertyType::ByteArray(_) => {
                value.to_binary_bytes().map_err(ProtocolError::ValueError)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                value
                    .to_identifier_bytes()
                    .map_err(ProtocolError::ValueError)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                if value_as_boolean {
                    Ok(vec![1])
                } else {
                    Ok(vec![0])
                }
            }
            DocumentPropertyType::Object(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object".to_string(),
                ),
            )),
            // Arrays are never index keys: the parser refuses an index on one
            DocumentPropertyType::Array(_)
            | DocumentPropertyType::VariableTypeArray(_)
            | DocumentPropertyType::TypedArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an array".to_string(),
                ),
            )),
        }
    }

    // Given a field type and a Vec<u8> this function chooses and executes the right decoding method
    pub fn decode_value_for_tree_keys(&self, value: &[u8]) -> Result<Value, ProtocolError> {
        if value.is_empty() {
            return Ok(Value::Null);
        }
        match self {
            DocumentPropertyType::String(_) => {
                if value == [0] {
                    // we don't want to collide with the definition of an empty string
                    Ok(Value::Text("".to_string()))
                } else {
                    Ok(Value::Text(String::from_utf8(value.to_vec()).map_err(
                        |_| {
                            ProtocolError::DecodingError(
                                "could not decode utf8 bytes into string".to_string(),
                            )
                        },
                    )?))
                }
            }
            DocumentPropertyType::Date => {
                let timestamp = DocumentPropertyType::decode_date_timestamp(value).ok_or(
                    ProtocolError::DecodingError("could not decode data timestamp".to_string()),
                )?;
                Ok(Value::U64(timestamp))
            }
            DocumentPropertyType::U128 => {
                let integer = DocumentPropertyType::decode_u128(value).ok_or(
                    ProtocolError::DecodingError("could not decode u128".to_string()),
                )?;
                Ok(Value::U128(integer))
            }
            DocumentPropertyType::I128 => {
                let integer = DocumentPropertyType::decode_i128(value).ok_or(
                    ProtocolError::DecodingError("could not decode i128".to_string()),
                )?;
                Ok(Value::I128(integer))
            }
            DocumentPropertyType::U64 => {
                let integer = DocumentPropertyType::decode_u64(value).ok_or(
                    ProtocolError::DecodingError("could not decode u64".to_string()),
                )?;
                Ok(Value::U64(integer))
            }
            DocumentPropertyType::I64 => {
                let integer = DocumentPropertyType::decode_i64(value).ok_or(
                    ProtocolError::DecodingError("could not decode i64".to_string()),
                )?;
                Ok(Value::I64(integer))
            }
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                let integer = DocumentPropertyType::decode_u32(value).ok_or(
                    ProtocolError::DecodingError("could not decode u32".to_string()),
                )?;
                Ok(Value::U32(integer))
            }
            DocumentPropertyType::I32 => {
                let integer = DocumentPropertyType::decode_i32(value).ok_or(
                    ProtocolError::DecodingError("could not decode i32".to_string()),
                )?;
                Ok(Value::I32(integer))
            }
            DocumentPropertyType::U16 => {
                let integer = DocumentPropertyType::decode_u16(value).ok_or(
                    ProtocolError::DecodingError("could not decode u16".to_string()),
                )?;
                Ok(Value::U16(integer))
            }
            DocumentPropertyType::I16 => {
                let integer = DocumentPropertyType::decode_i16(value).ok_or(
                    ProtocolError::DecodingError("could not decode i16".to_string()),
                )?;
                Ok(Value::I16(integer))
            }
            DocumentPropertyType::U8 => {
                let integer = DocumentPropertyType::decode_u8(value).ok_or(
                    ProtocolError::DecodingError("could not decode u8".to_string()),
                )?;
                Ok(Value::U8(integer))
            }
            DocumentPropertyType::I8 => {
                let integer = DocumentPropertyType::decode_i8(value).ok_or(
                    ProtocolError::DecodingError("could not decode i8".to_string()),
                )?;
                Ok(Value::I8(integer))
            }
            DocumentPropertyType::F64 => {
                let float = DocumentPropertyType::decode_float(value).ok_or(
                    ProtocolError::DecodingError("could not decode float".to_string()),
                )?;
                Ok(Value::Float(float))
            }
            DocumentPropertyType::ByteArray(_) => Ok(Value::Bytes(value.to_vec())),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let identifier = Identifier::from_bytes(value)?;
                Ok(identifier.into())
            }
            DocumentPropertyType::Boolean => {
                if value == [0] {
                    Ok(Value::Bool(false))
                } else if value == [1] {
                    Ok(Value::Bool(true))
                } else {
                    Err(ProtocolError::DecodingError(
                        "could not decode bool".to_string(),
                    ))
                }
            }
            DocumentPropertyType::Object(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try decoding an object".to_string(),
                ),
            )),
            DocumentPropertyType::Array(_)
            | DocumentPropertyType::VariableTypeArray(_)
            | DocumentPropertyType::TypedArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try decoding an array".to_string(),
                ),
            )),
        }
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn value_from_string(&self, str: &str) -> Result<Value, DataContractError> {
        match self {
            DocumentPropertyType::String(sizes) => {
                if let Some(min) = sizes.min_length {
                    if str.len() < min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = sizes.max_length {
                    if str.len() > max as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too big".to_string(),
                        ));
                    }
                }
                Ok(Value::Text(str.to_string()))
            }
            DocumentPropertyType::U128 => str.parse::<u128>().map(Value::U128).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u128 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I128 => str.parse::<i128>().map(Value::I128).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i128 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U64 => str.parse::<u64>().map(Value::U64).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u64 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I64 => str.parse::<i64>().map(Value::I64).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i64 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
                str.parse::<u32>().map(Value::U32).map_err(|_| {
                    DataContractError::ValueWrongType(
                        "value is not a u32 integer from string".to_string(),
                    )
                })
            }
            DocumentPropertyType::I32 => str.parse::<i32>().map(Value::I32).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i32 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U16 => str.parse::<u16>().map(Value::U16).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u16 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I16 => str.parse::<i16>().map(Value::I16).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i16 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U8 => str.parse::<u8>().map(Value::U8).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u8 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I8 => str.parse::<i8>().map(Value::I8).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i8 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::F64 | DocumentPropertyType::Date => {
                str.parse::<f64>().map(Value::Float).map_err(|_| {
                    DataContractError::ValueWrongType(
                        "value is not a float from string".to_string(),
                    )
                })
            }
            DocumentPropertyType::ByteArray(sizes) => {
                if let Some(min) = sizes.min_size {
                    if str.len() / 2 < min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "byte array is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = sizes.max_size {
                    if str.len() / 2 > max as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "byte array is too big".to_string(),
                        ));
                    }
                }
                Ok(Value::Bytes(hex::decode(str).map_err(|_| {
                    DataContractError::ValueDecodingError("could not parse hex bytes".to_string())
                })?))
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Value::Identifier(
                    Value::Text(str.to_owned())
                        .to_identifier()
                        .map_err(|e| DataContractError::ValueDecodingError(format!("{:?}", e)))?
                        .into_buffer(),
                ))
            }
            DocumentPropertyType::Boolean => {
                if str.to_lowercase().as_str() == "true" {
                    Ok(Value::Bool(true))
                } else if str.to_lowercase().as_str() == "false" {
                    Ok(Value::Bool(false))
                } else {
                    Err(DataContractError::ValueDecodingError(
                        "could not parse a boolean to a value".to_string(),
                    ))
                }
            }
            DocumentPropertyType::Object(_) => {
                Err(DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object".to_string(),
                ))
            }
            // A string names one value, never a list of them
            DocumentPropertyType::Array(_)
            | DocumentPropertyType::VariableTypeArray(_)
            | DocumentPropertyType::TypedArray(_) => {
                Err(DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an array".to_string(),
                ))
            }
        }
    }

    pub fn encode_date_timestamp(val: TimestampMillis) -> Vec<u8> {
        Self::encode_u64(val)
    }

    pub fn decode_date_timestamp(val: &[u8]) -> Option<TimestampMillis> {
        Self::decode_u64(val)
    }

    pub fn encode_u128(val: u128) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u128::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 128 bits.
    pub fn decode_u128(val: &[u8]) -> Option<u128> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u128::<BigEndian>().ok()
    }

    pub fn encode_i128(val: i128) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i128::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i128(val: &[u8]) -> Option<i128> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i128::<BigEndian>().ok()
    }

    pub fn encode_u64(val: u64) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u64::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 64 bits.
    pub fn decode_u64(val: &[u8]) -> Option<u64> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u64::<BigEndian>().ok()
    }

    pub fn encode_i64(val: i64) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i64::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i64(val: &[u8]) -> Option<i64> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i64::<BigEndian>().ok()
    }

    pub fn encode_u32(val: u32) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u32::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 32 bits.
    pub fn decode_u32(val: &[u8]) -> Option<u32> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u32::<BigEndian>().ok()
    }

    pub fn encode_i32(val: i32) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i32::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i32(val: &[u8]) -> Option<i32> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i32::<BigEndian>().ok()
    }

    pub fn encode_u16(val: u16) -> Vec<u8> {
        //todo this should just be to_be_bytes (and for all unsigned integers)
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u16::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 16 bits.
    pub fn decode_u16(val: &[u8]) -> Option<u16> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u16::<BigEndian>().ok()
    }

    pub fn encode_i16(val: i16) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i16::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i16(val: &[u8]) -> Option<i16> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i16::<BigEndian>().ok()
    }

    pub fn encode_u8(val: u8) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u8(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 8 bits.
    pub fn decode_u8(val: &[u8]) -> Option<u8> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u8().ok()
    }

    pub fn encode_i8(val: i8) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i8(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i8(val: &[u8]) -> Option<i8> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i8().ok()
    }

    pub fn encode_float(val: f64) -> Vec<u8> {
        // Floats are represented based on the  IEEE 754-2008 standard
        // [sign bit] [biased exponent] [mantissa]

        // when comparing floats, the sign bit has the greatest impact
        // any positive number is greater than all negative numbers
        // if the numbers come from the same domain then the exponent is the next factor to consider
        // the exponent gives a sense of how many digits are in the non fractional part of the number
        // for example in base 10, 10 has an exponent of 1 (1.0 * 10^1)
        // while 5000 (5.0 * 10^3) has an exponent of 3
        // for the positive domain, the bigger the exponent the larger the number i.e 5000 > 10
        // for the negative domain, the bigger the exponent the smaller the number i.e -10 > -5000
        // if the exponents are the same, then the mantissa is used to determine the greater number
        // the inverse relationship still holds
        // i.e bigger mantissa (bigger number in positive domain but smaller number in negative domain)

        // There are two things to fix to achieve total sort order
        // 1. Place positive domain above negative domain (i.e flip the sign bit)
        // 2. Exponent and mantissa for a smaller number like -5000 is greater than that of -10
        //    so bit level comparison would say -5000 is greater than -10
        //    we fix this by flipping the exponent and mantissa values, which has the effect of reversing
        //    the order (0000 [smallest] -> 1111 [largest])

        // Encode in big endian form, so most significant bits are compared first
        let mut wtr = vec![];
        wtr.write_f64::<BigEndian>(val).unwrap();

        // Check if the value is negative, if it is
        // flip all the bits i.e sign, exponent and mantissa
        if val < 0.0 {
            wtr = wtr.iter().map(|byte| !byte).collect();
        } else {
            // for positive values, just flip the sign bit
            wtr[0] ^= 0b1000_0000;
        }

        wtr
    }

    /// Decodes a float on 64 bits.
    pub fn decode_float(encoded: &[u8]) -> Option<f64> {
        // Check if the value is negative by looking at the original sign bit
        let is_negative = (encoded[0] & 0b1000_0000) == 0;

        // Create a mutable copy of the encoded vector to apply transformations
        let mut wtr = encoded.to_vec();

        if is_negative {
            // For originally negative values, flip all the bits back
            wtr = wtr.iter().map(|byte| !byte).collect();
        } else {
            // For originally positive values, just flip the sign bit back
            wtr[0] ^= 0b1000_0000;
        }

        // Read the float value from the transformed vector
        let mut cursor = Cursor::new(wtr);
        cursor.read_f64::<BigEndian>().ok()
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            DocumentPropertyType::I8
                | DocumentPropertyType::I16
                | DocumentPropertyType::I32
                | DocumentPropertyType::I64
                | DocumentPropertyType::U8
                | DocumentPropertyType::U16
                | DocumentPropertyType::U32
                | DocumentPropertyType::KeyIdWithReference(_)
                | DocumentPropertyType::U64
        )
    }

    pub fn sanitize_value_mut(&self, value: &mut Value) {
        match (self, value.clone()) {
            // Convert hex or base64 strings to byte arrays for ByteArray fields
            (DocumentPropertyType::ByteArray(property_sizes), Value::Text(str_value)) => {
                // Try to decode the string
                let decoded_bytes = if let Ok(bytes) = hex::decode(&str_value) {
                    Some(bytes)
                } else {
                    // If hex fails, try base64 decoding
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::STANDARD.decode(str_value).ok()
                };

                if let Some(bytes) = decoded_bytes {
                    let byte_len = bytes.len();

                    // Check if the decoded bytes meet the size constraints
                    let size_ok = match (property_sizes.min_size, property_sizes.max_size) {
                        (Some(min), Some(max)) => {
                            byte_len >= min as usize && byte_len <= max as usize
                        }
                        (Some(min), None) => byte_len >= min as usize,
                        (None, Some(max)) => byte_len <= max as usize,
                        (None, None) => true,
                    };

                    if size_ok {
                        // Use specific byte array types for exact sizes
                        match bytes.len() {
                            20 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes20(arr);
                                }
                            }
                            32 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes32(arr);
                                }
                            }
                            36 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes36(arr);
                                }
                            }
                            _ => {
                                *value = Value::Bytes(bytes);
                            }
                        }
                    }
                    // If size constraints are not met, leave the value as is
                }
                // If decoding fails, leave the value as is (validation will catch it later)
            }

            // Normalize an array of integers to bytes for ByteArray fields. A
            // binary property re-hydrated through a schemaless JSON layer (e.g. an
            // edited-and-replaced cached document) arrives as a plain array of
            // numbers rather than Value::Bytes; convert it here, on the client
            // build path, so the strict binary serializer receives Value::Bytes.
            // (The block-processing serialize path never sanitizes, so this does
            // not change which state transitions are accepted.)
            (DocumentPropertyType::ByteArray(property_sizes), Value::Array(array)) => {
                let decoded: Result<Vec<u8>, _> =
                    array.iter().map(|byte| byte.to_integer::<u8>()).collect();

                if let Ok(bytes) = decoded {
                    let byte_len = bytes.len();

                    let size_ok = match (property_sizes.min_size, property_sizes.max_size) {
                        (Some(min), Some(max)) => {
                            byte_len >= min as usize && byte_len <= max as usize
                        }
                        (Some(min), None) => byte_len >= min as usize,
                        (None, Some(max)) => byte_len <= max as usize,
                        (None, None) => true,
                    };

                    if size_ok {
                        match bytes.len() {
                            20 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes20(arr);
                                }
                            }
                            32 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes32(arr);
                                }
                            }
                            36 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes36(arr);
                                }
                            }
                            _ => {
                                *value = Value::Bytes(bytes);
                            }
                        }
                    }
                    // If size constraints are not met, leave the value as is.
                }
                // If any element is not a 0..=255 integer, leave the value as is
                // (validation will reject it later).
            }

            // Convert hex or base58 strings to identifiers for Identifier fields
            (
                DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_),
                Value::Text(str_value),
            ) => {
                // First try base58 decoding (most common for identifiers)
                if let Ok(id) = Identifier::from_string_unknown_encoding(&str_value) {
                    *value = Value::Identifier(id.into_buffer());
                }
                // If both conversions fail, leave the value as is (validation will catch it later)
            }

            // Ensure integers are in the correct range for their type
            (DocumentPropertyType::U8, Value::U8(_)) => {} // Already correct
            (DocumentPropertyType::U8, Value::U16(n)) if n <= u8::MAX as u16 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U32(n)) if n <= u8::MAX as u32 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U64(n)) if n <= u8::MAX as u64 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U128(n)) if n <= u8::MAX as u128 => {
                *value = Value::U8(n as u8);
            }

            (DocumentPropertyType::U16, Value::U16(_)) => {} // Already correct
            (DocumentPropertyType::U16, Value::U8(n)) => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U32(n)) if n <= u16::MAX as u32 => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U64(n)) if n <= u16::MAX as u64 => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U128(n)) if n <= u16::MAX as u128 => {
                *value = Value::U16(n as u16);
            }

            (
                DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_),
                Value::U32(_),
            ) => {} // Already correct
            (
                DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_),
                Value::U8(n),
            ) => {
                *value = Value::U32(n as u32);
            }
            (
                DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_),
                Value::U16(n),
            ) => {
                *value = Value::U32(n as u32);
            }
            (
                DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_),
                Value::U64(n),
            ) if n <= u32::MAX as u64 => {
                *value = Value::U32(n as u32);
            }
            (
                DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_),
                Value::U128(n),
            ) if n <= u32::MAX as u128 => {
                *value = Value::U32(n as u32);
            }

            (DocumentPropertyType::U64, Value::U64(_)) => {} // Already correct
            (DocumentPropertyType::U64, Value::U8(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U16(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U32(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U128(n)) if n <= u64::MAX as u128 => {
                *value = Value::U64(n as u64);
            }

            (DocumentPropertyType::U128, Value::U128(_)) => {} // Already correct
            (DocumentPropertyType::U128, Value::U8(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U16(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U32(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U64(n)) => {
                *value = Value::U128(n as u128);
            }

            // Handle signed integers similarly
            (DocumentPropertyType::I8, Value::I8(_)) => {} // Already correct
            (DocumentPropertyType::I8, Value::I16(n))
                if n >= i8::MIN as i16 && n <= i8::MAX as i16 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I32(n))
                if n >= i8::MIN as i32 && n <= i8::MAX as i32 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I64(n))
                if n >= i8::MIN as i64 && n <= i8::MAX as i64 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I128(n))
                if n >= i8::MIN as i128 && n <= i8::MAX as i128 =>
            {
                *value = Value::I8(n as i8);
            }

            (DocumentPropertyType::I16, Value::I16(_)) => {} // Already correct
            (DocumentPropertyType::I16, Value::I8(n)) => {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I32(n))
                if n >= i16::MIN as i32 && n <= i16::MAX as i32 =>
            {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I64(n))
                if n >= i16::MIN as i64 && n <= i16::MAX as i64 =>
            {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I128(n))
                if n >= i16::MIN as i128 && n <= i16::MAX as i128 =>
            {
                *value = Value::I16(n as i16);
            }

            (DocumentPropertyType::I32, Value::I32(_)) => {} // Already correct
            (DocumentPropertyType::I32, Value::I8(n)) => {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I16(n)) => {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I64(n))
                if n >= i32::MIN as i64 && n <= i32::MAX as i64 =>
            {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I128(n))
                if n >= i32::MIN as i128 && n <= i32::MAX as i128 =>
            {
                *value = Value::I32(n as i32);
            }

            (DocumentPropertyType::I64, Value::I64(_)) => {} // Already correct
            (DocumentPropertyType::I64, Value::I8(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I16(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I32(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I128(n))
                if n >= i64::MIN as i128 && n <= i64::MAX as i128 =>
            {
                *value = Value::I64(n as i64);
            }

            (DocumentPropertyType::I128, Value::I128(_)) => {} // Already correct
            (DocumentPropertyType::I128, Value::I8(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I16(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I32(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I64(n)) => {
                *value = Value::I128(n as i128);
            }

            // Handle Date type - convert integers to date
            (DocumentPropertyType::Date, Value::U64(_)) => {
                // Timestamp is already in the right format (milliseconds since epoch)
                // But we might want to validate it's a reasonable date
                // For now, just leave it as is
            }
            (DocumentPropertyType::Date, Value::I64(timestamp)) if timestamp >= 0 => {
                *value = Value::U64(timestamp as u64);
            }

            // Handle Object type - recursively sanitize nested fields
            (DocumentPropertyType::Object(schema), Value::Map(_)) => {
                if let Value::Map(map) = value {
                    for (key, nested_value) in map.iter_mut() {
                        if let Value::Text(field_name) = key {
                            if let Some(field_property) = schema.get(field_name) {
                                field_property
                                    .property_type
                                    .sanitize_value_mut(nested_value);
                            }
                        }
                    }
                }
            }

            // Handle Array type - sanitize all elements
            (DocumentPropertyType::Array(item_type), Value::Array(_)) => {
                if let Value::Array(items) = value {
                    for item in items.iter_mut() {
                        item_type.sanitize_value_mut(item);
                    }
                }
            }

            // A typed array's elements sanitize as scalars of its element type
            (DocumentPropertyType::TypedArray(typed_array), Value::Array(_)) => {
                if let Value::Array(items) = value {
                    for item in items.iter_mut() {
                        typed_array.item_type.sanitize_value_mut(item);
                    }
                }
            }

            // Handle VariableTypeArray - each item can have a different type
            (DocumentPropertyType::VariableTypeArray(item_types), Value::Array(_)) => {
                if let Value::Array(items) = value {
                    for (item, item_type) in items.iter_mut().zip(item_types.iter().cycle()) {
                        item_type.sanitize_value_mut(item);
                    }
                }
            }

            // For all other cases, leave the value as is
            _ => {}
        }
    }

    pub fn try_from_value_map(
        value_map: &BTreeMap<String, &Value>,
        options: &DocumentPropertyTypeParsingOptions,
    ) -> Result<Self, DataContractError> {
        let type_value = value_map.get_str(property_names::TYPE)?;

        let property_type = match type_value {
            "integer" => {
                if options.sized_integer_types {
                    find_integer_type_for_subschema_value(value_map)?
                } else {
                    DocumentPropertyType::I64
                }
            }
            "string" => DocumentPropertyType::String(StringPropertySizes {
                min_length: value_map.get_optional_integer(property_names::MIN_LENGTH)?,
                max_length: value_map.get_optional_integer(property_names::MAX_LENGTH)?,
                max_bytes: None,
            }),
            "array" => {
                // Only handling bytearrays for v1
                // Return an error if it is not a byte array
                let Some(is_byte_array) =
                    value_map.get_optional_bool(property_names::BYTE_ARRAY)?
                else {
                    return Err(DataContractError::InvalidContractStructure(
                        "only byte arrays are supported now".to_string(),
                    ));
                };

                if !is_byte_array {
                    return Err(DataContractError::InvalidContractStructure(
                        "byteArray should always be true if defined".to_string(),
                    ));
                }

                match value_map.get_optional_str(property_names::CONTENT_MEDIA_TYPE)? {
                    Some("application/x.dash.dpp.identifier") => DocumentPropertyType::Identifier,
                    Some(_) | None => DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                        min_size: value_map.get_optional_integer(property_names::MIN_ITEMS)?,
                        max_size: value_map.get_optional_integer(property_names::MAX_ITEMS)?,
                    }),
                }
            }
            "object" => Self::Object(Default::default()),
            "boolean" => DocumentPropertyType::Boolean,
            "number" => DocumentPropertyType::F64,
            _ => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "unsupported property type: {}",
                    type_value
                )));
            }
        };

        Ok(property_type)
    }
}

#[derive(Debug, Clone)]
pub struct DocumentPropertyTypeParsingOptions {
    pub sized_integer_types: bool,
}

impl Default for DocumentPropertyTypeParsingOptions {
    fn default() -> Self {
        Self {
            sized_integer_types: true,
        }
    }
}

impl From<&DataContractConfig> for DocumentPropertyTypeParsingOptions {
    fn from(config: &DataContractConfig) -> Self {
        Self {
            sized_integer_types: config.sized_integer_types(),
        }
    }
}

fn get_field_type_matching_error(value: &Value) -> DataContractError {
    DataContractError::ValueWrongType(format!(
        "document field type doesn't match \"{}\" document value",
        value
    ))
}

fn find_integer_type_for_subschema_value(
    value: &BTreeMap<String, &Value>,
) -> Result<DocumentPropertyType, DataContractError> {
    let minimum = value.get_optional_integer::<i64>(property_names::MINIMUM)?;
    let maximum = value.get_optional_integer::<i64>(property_names::MAXIMUM)?;

    let property_type = match (minimum, maximum) {
        (Some(min), Some(max)) => find_integer_type_for_min_and_max_values(min, max),
        (Some(min), None) => {
            if min >= 0 {
                DocumentPropertyType::U64
            } else {
                DocumentPropertyType::I64
            }
        }
        (None, Some(max)) => find_unsigned_integer_type_for_max_value(max),
        (None, None) => {
            // If enum is defined, we can try to figure out type based on minimal and maximal values
            let enum_type = if let Some(enum_values) =
                value.get_optional_inner_value_array::<Vec<_>>(property_names::ENUM)?
            {
                match enum_values
                    .into_iter()
                    .filter_map(|v| v.as_integer())
                    .minmax()
                {
                    itertools::MinMaxResult::MinMax(min, max) => {
                        Some(find_integer_type_for_min_and_max_values(min, max))
                    }
                    itertools::MinMaxResult::OneElement(val) => {
                        Some(find_unsigned_integer_type_for_max_value(val))
                    }
                    _ => None,
                }
            } else {
                None
            };

            if let Some(enum_type) = enum_type {
                enum_type
            } else {
                DocumentPropertyType::I64
            }
        }
    };

    Ok(property_type)
}

fn find_unsigned_integer_type_for_max_value(max_value: i64) -> DocumentPropertyType {
    if max_value <= u8::MAX as i64 {
        DocumentPropertyType::U8
    } else if max_value <= u16::MAX as i64 {
        DocumentPropertyType::U16
    } else if max_value <= u32::MAX as i64 {
        DocumentPropertyType::U32
    } else {
        DocumentPropertyType::U64
    }
}

fn find_integer_type_for_min_and_max_values(min: i64, max: i64) -> DocumentPropertyType {
    if min >= 0 {
        find_unsigned_integer_type_for_max_value(max)
    } else if min >= i8::MIN as i64 && max <= i8::MAX as i64 {
        DocumentPropertyType::I8
    } else if min >= i16::MIN as i64 && max <= i16::MAX as i64 {
        DocumentPropertyType::I16
    } else if min >= i32::MIN as i64 && max <= i32::MAX as i64 {
        DocumentPropertyType::I32
    } else {
        DocumentPropertyType::I64
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, SecurityLevel};
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;

    // -----------------------------------------------------------------------
    // name() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_name_returns_correct_string_for_all_variants() {
        let cases: Vec<(DocumentPropertyType, &str)> = vec![
            (DocumentPropertyType::U128, "u128"),
            (DocumentPropertyType::I128, "i128"),
            (DocumentPropertyType::U64, "u64"),
            (DocumentPropertyType::I64, "i64"),
            (DocumentPropertyType::U32, "u32"),
            (DocumentPropertyType::I32, "i32"),
            (DocumentPropertyType::U16, "u16"),
            (DocumentPropertyType::I16, "i16"),
            (DocumentPropertyType::U8, "u8"),
            (DocumentPropertyType::I8, "i8"),
            (DocumentPropertyType::F64, "f64"),
            (
                DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: None,
                    max_bytes: None,
                }),
                "string",
            ),
            (
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: None,
                    max_size: None,
                }),
                "byteArray",
            ),
            (DocumentPropertyType::Identifier, "identifier"),
            (DocumentPropertyType::Boolean, "boolean"),
            (DocumentPropertyType::Date, "date"),
            (DocumentPropertyType::Object(IndexMap::new()), "object"),
            (DocumentPropertyType::Array(ArrayItemType::Integer), "array"),
            (
                DocumentPropertyType::VariableTypeArray(vec![]),
                "variableTypeArray",
            ),
        ];
        for (prop_type, expected) in cases {
            assert_eq!(
                prop_type.name(),
                expected,
                "name() mismatch for {:?}",
                prop_type
            );
        }
    }

    // -----------------------------------------------------------------------
    // try_from_name() tests
    // -----------------------------------------------------------------------

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_known_types() {
        assert_eq!(
            DocumentPropertyType::try_from_name("u128").unwrap(),
            DocumentPropertyType::U128
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i128").unwrap(),
            DocumentPropertyType::I128
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u64").unwrap(),
            DocumentPropertyType::U64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i64").unwrap(),
            DocumentPropertyType::I64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("integer").unwrap(),
            DocumentPropertyType::I64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u32").unwrap(),
            DocumentPropertyType::U32
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i32").unwrap(),
            DocumentPropertyType::I32
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u16").unwrap(),
            DocumentPropertyType::U16
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i16").unwrap(),
            DocumentPropertyType::I16
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u8").unwrap(),
            DocumentPropertyType::U8
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i8").unwrap(),
            DocumentPropertyType::I8
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("f64").unwrap(),
            DocumentPropertyType::F64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("number").unwrap(),
            DocumentPropertyType::F64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("boolean").unwrap(),
            DocumentPropertyType::Boolean
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("date").unwrap(),
            DocumentPropertyType::Date
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("identifier").unwrap(),
            DocumentPropertyType::Identifier
        );
        assert!(DocumentPropertyType::try_from_name("string").is_ok());
        assert!(DocumentPropertyType::try_from_name("byteArray").is_ok());
        assert!(DocumentPropertyType::try_from_name("object").is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_array_returns_error() {
        assert!(DocumentPropertyType::try_from_name("array").is_err());
    }

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_unknown_returns_error() {
        assert!(DocumentPropertyType::try_from_name("unknown_type").is_err());
    }

    // -----------------------------------------------------------------------
    // min_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_size_fixed_width_types() {
        assert_eq!(DocumentPropertyType::U128.min_size(), Some(16));
        assert_eq!(DocumentPropertyType::I128.min_size(), Some(16));
        assert_eq!(DocumentPropertyType::U64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::I64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::U32.min_size(), Some(4));
        assert_eq!(DocumentPropertyType::I32.min_size(), Some(4));
        assert_eq!(DocumentPropertyType::U16.min_size(), Some(2));
        assert_eq!(DocumentPropertyType::I16.min_size(), Some(2));
        assert_eq!(DocumentPropertyType::U8.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::I8.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::F64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::Boolean.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::Date.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::Identifier.min_size(), Some(32));
    }

    #[test]
    fn test_min_size_string_with_and_without_min_length() {
        let no_min = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        assert_eq!(no_min.min_size(), Some(0));

        let with_min = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(5),
            max_length: None,
            max_bytes: None,
        });
        assert_eq!(with_min.min_size(), Some(5));
    }

    #[test]
    fn test_min_size_byte_array_with_and_without_min() {
        let no_min = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(no_min.min_size(), Some(0));

        let with_min = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: None,
        });
        assert_eq!(with_min.min_size(), Some(10));
    }

    #[test]
    fn test_min_size_array_and_variable_type_array_return_none() {
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.min_size(), None);

        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.min_size(), None);
    }

    #[test]
    fn test_min_size_object_sums_sub_field_sizes() {
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "field1".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "field2".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        assert_eq!(obj.min_size(), Some(12)); // 4 + 8
    }

    // -----------------------------------------------------------------------
    // max_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_max_size_fixed_width_types() {
        assert_eq!(DocumentPropertyType::U128.max_size(), Some(16));
        assert_eq!(DocumentPropertyType::I128.max_size(), Some(16));
        assert_eq!(DocumentPropertyType::U64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::I64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::U32.max_size(), Some(4));
        assert_eq!(DocumentPropertyType::I32.max_size(), Some(4));
        assert_eq!(DocumentPropertyType::U16.max_size(), Some(2));
        assert_eq!(DocumentPropertyType::I16.max_size(), Some(2));
        assert_eq!(DocumentPropertyType::U8.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::I8.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::F64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::Boolean.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::Date.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::Identifier.max_size(), Some(32));
    }

    #[test]
    fn test_max_size_string_defaults_and_explicit() {
        let no_max = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        assert_eq!(no_max.max_size(), Some(16383));

        let with_max = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
            max_bytes: None,
        });
        assert_eq!(with_max.max_size(), Some(100));
    }

    #[test]
    fn test_max_size_byte_array_defaults_and_explicit() {
        let no_max = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(no_max.max_size(), Some(u16::MAX));

        let with_max = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(256),
        });
        assert_eq!(with_max.max_size(), Some(256));
    }

    #[test]
    fn test_max_size_array_and_variable_type_array_return_none() {
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.max_size(), None);
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.max_size(), None);
    }

    // -----------------------------------------------------------------------
    // min_byte_size() / max_byte_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_byte_size_fixed_width() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::U128.min_byte_size(pv).unwrap(),
            Some(16)
        );
        assert_eq!(
            DocumentPropertyType::I128.min_byte_size(pv).unwrap(),
            Some(16)
        );
        assert_eq!(
            DocumentPropertyType::U64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::I64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::U32.min_byte_size(pv).unwrap(),
            Some(4)
        );
        assert_eq!(
            DocumentPropertyType::I32.min_byte_size(pv).unwrap(),
            Some(4)
        );
        assert_eq!(
            DocumentPropertyType::U16.min_byte_size(pv).unwrap(),
            Some(2)
        );
        assert_eq!(
            DocumentPropertyType::I16.min_byte_size(pv).unwrap(),
            Some(2)
        );
        assert_eq!(DocumentPropertyType::U8.min_byte_size(pv).unwrap(), Some(1));
        assert_eq!(DocumentPropertyType::I8.min_byte_size(pv).unwrap(), Some(1));
        assert_eq!(
            DocumentPropertyType::F64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::Boolean.min_byte_size(pv).unwrap(),
            Some(1)
        );
        assert_eq!(
            DocumentPropertyType::Date.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::Identifier.min_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    #[test]
    fn test_min_byte_size_string_multiplied_by_4() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(10),
            max_length: None,
            max_bytes: None,
        });
        // protocol version > 8 => checked_mul(4)
        assert_eq!(s.min_byte_size(pv).unwrap(), Some(40));
    }

    #[test]
    fn test_min_byte_size_string_no_min() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        assert_eq!(s.min_byte_size(pv).unwrap(), Some(0));
    }

    #[test]
    fn test_max_byte_size_string_multiplied_by_4() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
            max_bytes: None,
        });
        assert_eq!(s.max_byte_size(pv).unwrap(), Some(400));
    }

    #[test]
    fn test_max_byte_size_string_no_max() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        assert_eq!(s.max_byte_size(pv).unwrap(), Some(u16::MAX));
    }

    #[test]
    fn test_min_byte_size_byte_array_with_min() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(100),
        });
        assert_eq!(ba.min_byte_size(pv).unwrap(), Some(20));
    }

    #[test]
    fn test_max_byte_size_byte_array_with_max() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(200),
        });
        assert_eq!(ba.max_byte_size(pv).unwrap(), Some(200));
    }

    #[test]
    fn test_max_byte_size_byte_array_no_max() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(ba.max_byte_size(pv).unwrap(), Some(u16::MAX));
    }

    #[test]
    fn test_min_byte_size_array_returns_none() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.min_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_max_byte_size_array_returns_none() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.max_byte_size(pv).unwrap(), None);
    }

    // -----------------------------------------------------------------------
    // middle_size() / middle_size_ceil() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_middle_size_fixed_type() {
        let pv = PlatformVersion::latest();
        // U32: min=4, max=4 => middle = (4+4)/2 = 4
        assert_eq!(DocumentPropertyType::U32.middle_size(pv), Some(4));
    }

    #[test]
    fn test_middle_size_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(0),
            max_length: Some(100),
            max_bytes: None,
        });
        // min_size=0, max_size=100 => (0+100)/2 = 50
        assert_eq!(s.middle_size(pv), Some(50));
    }

    #[test]
    fn test_middle_size_ceil_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(0),
            max_length: Some(101),
            max_bytes: None,
        });
        // min_size=0, max_size=101 => ceil((0+101)/2) = 51
        assert_eq!(s.middle_size_ceil(pv), Some(51));
    }

    #[test]
    fn test_middle_size_returns_none_for_array() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.middle_size(pv), None);
    }

    // -----------------------------------------------------------------------
    // middle_byte_size() / middle_byte_size_ceil() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_middle_byte_size_fixed_type() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::U64.middle_byte_size(pv).unwrap(),
            Some(8)
        );
    }

    #[test]
    fn test_middle_byte_size_returns_none_for_array() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.middle_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_middle_byte_size_ceil_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(1),
            max_length: Some(10),
            max_bytes: None,
        });
        // min_byte_size = 1*4 = 4, max_byte_size = 10*4 = 40
        // ceil((4+40)/2) = 22
        assert_eq!(s.middle_byte_size_ceil(pv).unwrap(), Some(22));
    }

    // -----------------------------------------------------------------------
    // is_integer() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_integer_returns_true_for_integer_types() {
        assert!(DocumentPropertyType::I8.is_integer());
        assert!(DocumentPropertyType::I16.is_integer());
        assert!(DocumentPropertyType::I32.is_integer());
        assert!(DocumentPropertyType::I64.is_integer());
        assert!(DocumentPropertyType::U8.is_integer());
        assert!(DocumentPropertyType::U16.is_integer());
        assert!(DocumentPropertyType::U32.is_integer());
        assert!(DocumentPropertyType::U64.is_integer());
    }

    #[test]
    fn test_is_integer_returns_false_for_non_integer_types() {
        assert!(!DocumentPropertyType::F64.is_integer());
        assert!(!DocumentPropertyType::Boolean.is_integer());
        assert!(!DocumentPropertyType::Date.is_integer());
        assert!(!DocumentPropertyType::Identifier.is_integer());
        assert!(!DocumentPropertyType::U128.is_integer());
        assert!(!DocumentPropertyType::I128.is_integer());
        assert!(!DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        })
        .is_integer());
    }

    // -----------------------------------------------------------------------
    // encode / decode roundtrip tests for tree keys
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_decode_u128_roundtrip() {
        let values: Vec<u128> = vec![0, 1, u128::MAX / 2, u128::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u128(val);
            let decoded = DocumentPropertyType::decode_u128(&encoded).unwrap();
            assert_eq!(val, decoded, "u128 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i128_roundtrip() {
        let values: Vec<i128> = vec![i128::MIN, -1, 0, 1, i128::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i128(val);
            let decoded = DocumentPropertyType::decode_i128(&encoded).unwrap();
            assert_eq!(val, decoded, "i128 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u64_roundtrip() {
        let values: Vec<u64> = vec![0, 1, 42, u64::MAX / 2, u64::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u64(val);
            let decoded = DocumentPropertyType::decode_u64(&encoded).unwrap();
            assert_eq!(val, decoded, "u64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i64_roundtrip() {
        let values: Vec<i64> = vec![i64::MIN, -1, 0, 1, i64::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i64(val);
            let decoded = DocumentPropertyType::decode_i64(&encoded).unwrap();
            assert_eq!(val, decoded, "i64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u32_roundtrip() {
        let values: Vec<u32> = vec![0, 1, u32::MAX / 2, u32::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u32(val);
            let decoded = DocumentPropertyType::decode_u32(&encoded).unwrap();
            assert_eq!(val, decoded, "u32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i32_roundtrip() {
        let values: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i32(val);
            let decoded = DocumentPropertyType::decode_i32(&encoded).unwrap();
            assert_eq!(val, decoded, "i32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u16_roundtrip() {
        let values: Vec<u16> = vec![0, 1, u16::MAX / 2, u16::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u16(val);
            let decoded = DocumentPropertyType::decode_u16(&encoded).unwrap();
            assert_eq!(val, decoded, "u16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i16_roundtrip() {
        let values: Vec<i16> = vec![i16::MIN, -1, 0, 1, i16::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i16(val);
            let decoded = DocumentPropertyType::decode_i16(&encoded).unwrap();
            assert_eq!(val, decoded, "i16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u8_roundtrip() {
        let values: Vec<u8> = vec![0, 1, 127, 255];
        for val in values {
            let encoded = DocumentPropertyType::encode_u8(val);
            let decoded = DocumentPropertyType::decode_u8(&encoded).unwrap();
            assert_eq!(val, decoded, "u8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i8_roundtrip() {
        let values: Vec<i8> = vec![i8::MIN, -1, 0, 1, i8::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i8(val);
            let decoded = DocumentPropertyType::decode_i8(&encoded).unwrap();
            assert_eq!(val, decoded, "i8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_float_roundtrip() {
        let values: Vec<f64> = vec![-1000.5, -1.0, 0.0, 1.0, 42.42, 1000.5];
        for val in values {
            let encoded = DocumentPropertyType::encode_float(val);
            let decoded = DocumentPropertyType::decode_float(&encoded).unwrap();
            assert!(
                (val - decoded).abs() < f64::EPSILON,
                "float roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_encode_decode_date_timestamp_roundtrip() {
        let timestamps: Vec<u64> = vec![0, 1648910575000, u64::MAX];
        for ts in timestamps {
            let encoded = DocumentPropertyType::encode_date_timestamp(ts);
            let decoded = DocumentPropertyType::decode_date_timestamp(&encoded).unwrap();
            assert_eq!(ts, decoded, "date timestamp roundtrip failed for {}", ts);
        }
    }

    // -----------------------------------------------------------------------
    // encode sort order preservation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_u64_preserves_sort_order_in_lower_half() {
        // The encoding flips the sign bit, so sort order is preserved for
        // values in the lower half of the u64 range (0..2^63-1).
        let values: Vec<u64> = vec![0, 1, 100, 1000, i64::MAX as u64];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u64(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u64");
        }
    }

    #[test]
    fn test_encode_i64_preserves_sort_order() {
        let values: Vec<i64> = vec![i64::MIN, -100, -1, 0, 1, 100, i64::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i64(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for i64");
        }
    }

    #[test]
    fn test_encode_float_preserves_sort_order() {
        let values: Vec<f64> = vec![-1000.0, -1.0, -0.5, 0.0, 0.5, 1.0, 1000.0];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_float(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for float");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_for_tree_keys_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_for_tree_keys(&Value::Null).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_for_tree_keys_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop
            .encode_value_for_tree_keys(&Value::Text("".to_string()))
            .unwrap();
        assert_eq!(result, vec![0]); // empty string marker
    }

    #[test]
    fn test_encode_value_for_tree_keys_string_nonempty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop
            .encode_value_for_tree_keys(&Value::Text("hello".to_string()))
            .unwrap();
        assert_eq!(result, b"hello".to_vec());
    }

    #[test]
    fn test_encode_value_for_tree_keys_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let true_enc = prop.encode_value_for_tree_keys(&Value::Bool(true)).unwrap();
        assert_eq!(true_enc, vec![1]);
        let false_enc = prop
            .encode_value_for_tree_keys(&Value::Bool(false))
            .unwrap();
        assert_eq!(false_enc, vec![0]);
    }

    #[test]
    fn test_encode_value_for_tree_keys_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![1u8, 2, 3, 4];
        let result = prop
            .encode_value_for_tree_keys(&Value::Bytes(bytes.clone()))
            .unwrap();
        assert_eq!(result, bytes);
    }

    #[test]
    fn test_encode_value_for_tree_keys_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_for_tree_keys(&Value::Map(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_for_tree_keys_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.encode_value_for_tree_keys(&Value::Array(vec![]));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // decode_value_for_tree_keys() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_value_for_tree_keys_empty_returns_null() {
        let prop = DocumentPropertyType::U64;
        let result = prop.decode_value_for_tree_keys(&[]).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_decode_value_for_tree_keys_string_empty_marker() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop.decode_value_for_tree_keys(&[0]).unwrap();
        assert_eq!(result, Value::Text("".to_string()));
    }

    #[test]
    fn test_decode_value_for_tree_keys_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop.decode_value_for_tree_keys(b"hello").unwrap();
        assert_eq!(result, Value::Text("hello".to_string()));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_true() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[1]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_false() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[0]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_invalid() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_value_for_tree_keys_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![10, 20, 30];
        let result = prop.decode_value_for_tree_keys(&bytes).unwrap();
        assert_eq!(result, Value::Bytes(bytes));
    }

    #[test]
    fn test_decode_value_for_tree_keys_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_value_for_tree_keys_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() / decode_value_for_tree_keys() roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn test_tree_keys_roundtrip_all_integer_types() {
        // U64
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(12345);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I64
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-42);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U32
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I32
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-100);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U16
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(500);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I16
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-200);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U8
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(42);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I8
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-5);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U128
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(99999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I128
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-99999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);
    }

    #[test]
    fn test_tree_keys_roundtrip_float() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(3.14);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        if let Value::Float(f) = dec {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float value");
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_date() {
        let prop = DocumentPropertyType::Date;
        let val = Value::U64(1648910575000);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);
    }

    #[test]
    fn test_tree_keys_roundtrip_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes: [u8; 32] = [42u8; 32];
        let val = Value::Identifier(id_bytes);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        // Identifier decodes via Identifier::from_bytes, which gives Identifier variant
        if let Value::Identifier(decoded_id) = dec {
            assert_eq!(decoded_id, id_bytes);
        } else {
            panic!("expected identifier value");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::Null, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_with_size_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop
            .encode_value_with_size(Value::Text("hi".to_string()), true)
            .unwrap();
        // varint(2) + b"hi" = [2, 104, 105]
        assert_eq!(result.len(), 3);
        assert_eq!(&result[1..], b"hi");
    }

    #[test]
    fn test_encode_value_with_size_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let true_result = prop
            .encode_value_with_size(Value::Bool(true), true)
            .unwrap();
        assert_eq!(true_result, vec![1]);
        let false_result = prop
            .encode_value_with_size(Value::Bool(false), true)
            .unwrap();
        assert_eq!(false_result, vec![2]);
    }

    #[test]
    fn test_encode_value_with_size_u64_required() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::U64(42), true).unwrap();
        assert_eq!(result.len(), 8); // 8 bytes for u64
        assert_eq!(result, 42u64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_u64_not_required() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::U64(42), false).unwrap();
        assert_eq!(result.len(), 9); // 1 byte marker + 8 bytes
        assert_eq!(result[0], 255u8); // marker byte
        assert_eq!(&result[1..], 42u64.to_be_bytes().as_slice());
    }

    #[test]
    fn test_encode_value_with_size_i64_required() {
        let prop = DocumentPropertyType::I64;
        let result = prop.encode_value_with_size(Value::I64(-42), true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_f64_required() {
        let prop = DocumentPropertyType::F64;
        let result = prop
            .encode_value_with_size(Value::Float(3.14), true)
            .unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, 3.14f64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_f64_not_required() {
        let prop = DocumentPropertyType::F64;
        let result = prop
            .encode_value_with_size(Value::Float(3.14), false)
            .unwrap();
        assert_eq!(result.len(), 9);
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_with_size_date_required() {
        let prop = DocumentPropertyType::Date;
        let result = prop
            .encode_value_with_size(Value::Float(1648910575.0), true)
            .unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![1u8, 2, 3];
        let result = prop
            .encode_value_with_size(Value::Bytes(bytes), true)
            .unwrap();
        // varint(3) + [1,2,3]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_with_size_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [1u8; 32];
        let result = prop
            .encode_value_with_size(Value::Identifier(id_bytes), true)
            .unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_with_size_u128_required() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .encode_value_with_size(Value::U128(1000), true)
            .unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_with_size_u128_not_required() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .encode_value_with_size(Value::U128(1000), false)
            .unwrap();
        assert_eq!(result.len(), 17); // 1 marker + 16
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_with_size_i128_required() {
        let prop = DocumentPropertyType::I128;
        let result = prop
            .encode_value_with_size(Value::I128(-500), true)
            .unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_with_size_u32_required() {
        let prop = DocumentPropertyType::U32;
        let result = prop.encode_value_with_size(Value::U32(100), true).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result, 100u32.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_i32_required() {
        let prop = DocumentPropertyType::I32;
        let result = prop.encode_value_with_size(Value::I32(-50), true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_with_size_u16_required() {
        let prop = DocumentPropertyType::U16;
        let result = prop.encode_value_with_size(Value::U16(300), true).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result, 300u16.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_i16_required() {
        let prop = DocumentPropertyType::I16;
        let result = prop.encode_value_with_size(Value::I16(-100), true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_with_size_u8_required() {
        let prop = DocumentPropertyType::U8;
        let result = prop.encode_value_with_size(Value::U8(42), true).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_encode_value_with_size_i8_required() {
        let prop = DocumentPropertyType::I8;
        let result = prop.encode_value_with_size(Value::I8(-10), true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_with_size_variable_type_array_returns_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.encode_value_with_size(Value::Array(vec![]), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_string_type_mismatch() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop.encode_value_with_size(Value::U64(42), true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_ref_with_size(&Value::Null, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_ref_with_size_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let val = Value::Text("test".to_string());
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 5); // varint(4) + "test"
    }

    #[test]
    fn test_encode_value_ref_with_size_date_required() {
        let prop = DocumentPropertyType::Date;
        let val = Value::Float(1648910575.0);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_date_not_required() {
        let prop = DocumentPropertyType::Date;
        let val = Value::Float(1648910575.0);
        let result = prop.encode_value_ref_with_size(&val, false).unwrap();
        assert_eq!(result.len(), 9); // marker + 8 bytes
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_ref_with_size_u128() {
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(42);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_ref_with_size_i128() {
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-42);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_ref_with_size_u64() {
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(100);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_i64() {
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-100);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_u32() {
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(50);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_i32() {
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-50);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_u16() {
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(300);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_ref_with_size_i16() {
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-300);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_ref_with_size_u8() {
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(255);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_ref_with_size_i8() {
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-128);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_ref_with_size_f64() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(2.718);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let result_true = prop
            .encode_value_ref_with_size(&Value::Bool(true), true)
            .unwrap();
        assert_eq!(result_true, vec![1]);
        let result_false = prop
            .encode_value_ref_with_size(&Value::Bool(false), true)
            .unwrap();
        assert_eq!(result_false, vec![0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array_fixed_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(4),
        });
        let val = Value::Bytes(vec![1, 2, 3, 4]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // fixed size: no varint prefix
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array_variable_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        let val = Value::Bytes(vec![10, 20, 30]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // varint(3) + [10,20,30]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [5u8; 32];
        let val = Value::Identifier(id_bytes);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_encode_value_ref_with_size_variable_type_array_returns_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let val = Value::Array(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // value_from_string() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_from_string_string_type() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop.value_from_string("hello").unwrap();
        assert_eq!(result, Value::Text("hello".to_string()));
    }

    #[test]
    fn test_value_from_string_string_too_small() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(10),
            max_length: None,
            max_bytes: None,
        });
        let result = prop.value_from_string("hi");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_string_too_big() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(3),
            max_bytes: None,
        });
        let result = prop.value_from_string("hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_u128() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .value_from_string("340282366920938463463374607431768211455")
            .unwrap();
        assert_eq!(result, Value::U128(u128::MAX));
    }

    #[test]
    fn test_value_from_string_u128_invalid() {
        let prop = DocumentPropertyType::U128;
        assert!(prop.value_from_string("not_a_number").is_err());
    }

    #[test]
    fn test_value_from_string_i128() {
        let prop = DocumentPropertyType::I128;
        let result = prop.value_from_string("-1").unwrap();
        assert_eq!(result, Value::I128(-1));
    }

    #[test]
    fn test_value_from_string_i128_invalid() {
        let prop = DocumentPropertyType::I128;
        assert!(prop.value_from_string("abc").is_err());
    }

    #[test]
    fn test_value_from_string_u64() {
        let prop = DocumentPropertyType::U64;
        let result = prop.value_from_string("12345").unwrap();
        assert_eq!(result, Value::U64(12345));
    }

    #[test]
    fn test_value_from_string_u64_invalid() {
        let prop = DocumentPropertyType::U64;
        assert!(prop.value_from_string("-1").is_err());
    }

    #[test]
    fn test_value_from_string_i64() {
        let prop = DocumentPropertyType::I64;
        let result = prop.value_from_string("-42").unwrap();
        assert_eq!(result, Value::I64(-42));
    }

    #[test]
    fn test_value_from_string_u32() {
        let prop = DocumentPropertyType::U32;
        let result = prop.value_from_string("1000").unwrap();
        assert_eq!(result, Value::U32(1000));
    }

    #[test]
    fn test_value_from_string_i32() {
        let prop = DocumentPropertyType::I32;
        let result = prop.value_from_string("-1000").unwrap();
        assert_eq!(result, Value::I32(-1000));
    }

    #[test]
    fn test_value_from_string_u16() {
        let prop = DocumentPropertyType::U16;
        let result = prop.value_from_string("65535").unwrap();
        assert_eq!(result, Value::U16(65535));
    }

    #[test]
    fn test_value_from_string_i16() {
        let prop = DocumentPropertyType::I16;
        let result = prop.value_from_string("-32768").unwrap();
        assert_eq!(result, Value::I16(-32768));
    }

    #[test]
    fn test_value_from_string_u8() {
        let prop = DocumentPropertyType::U8;
        let result = prop.value_from_string("255").unwrap();
        assert_eq!(result, Value::U8(255));
    }

    #[test]
    fn test_value_from_string_u8_invalid() {
        let prop = DocumentPropertyType::U8;
        assert!(prop.value_from_string("256").is_err());
    }

    #[test]
    fn test_value_from_string_i8() {
        let prop = DocumentPropertyType::I8;
        let result = prop.value_from_string("-128").unwrap();
        assert_eq!(result, Value::I8(-128));
    }

    #[test]
    fn test_value_from_string_f64() {
        let prop = DocumentPropertyType::F64;
        let result = prop.value_from_string("3.14").unwrap();
        if let Value::Float(f) = result {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_value_from_string_date() {
        let prop = DocumentPropertyType::Date;
        let result = prop.value_from_string("1648910575.0").unwrap();
        assert!(matches!(result, Value::Float(_)));
    }

    #[test]
    fn test_value_from_string_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let result = prop.value_from_string("deadbeef").unwrap();
        assert_eq!(result, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_value_from_string_byte_array_too_small() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: None,
        });
        let result = prop.value_from_string("aabb");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_byte_array_too_big() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(2),
        });
        let result = prop.value_from_string("aabbccddee");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_byte_array_invalid_hex() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let result = prop.value_from_string("not_hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_boolean_true() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.value_from_string("true").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_value_from_string_boolean_false() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.value_from_string("false").unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_value_from_string_boolean_case_insensitive() {
        let prop = DocumentPropertyType::Boolean;
        assert_eq!(prop.value_from_string("TRUE").unwrap(), Value::Bool(true));
        assert_eq!(prop.value_from_string("False").unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_value_from_string_boolean_invalid() {
        let prop = DocumentPropertyType::Boolean;
        assert!(prop.value_from_string("yes").is_err());
    }

    #[test]
    fn test_value_from_string_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        assert!(prop.value_from_string("{}").is_err());
    }

    #[test]
    fn test_value_from_string_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert!(prop.value_from_string("[]").is_err());
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() tests
    // -----------------------------------------------------------------------

    /// A serialized document is untrusted input: a length prefix must never
    /// size an allocation before it has been checked against the bytes that
    /// are actually present. Before this check a two-byte string field
    /// declaring a multi-gigabyte length aborted the process on allocation.
    #[test]
    fn test_read_optionally_from_rejects_length_prefix_longer_than_input() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        // varint 2^62 followed by two bytes of payload
        let mut data = vec![0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x3f];
        data.extend_from_slice(b"ab");
        let mut reader = BufReader::new(data.as_slice());
        let err = prop
            .read_optionally_from(&mut reader, true)
            .expect_err("oversized length prefix must be rejected, not allocated");
        assert!(
            err.to_string()
                .contains("remain in the serialized document"),
            "unexpected error: {err}"
        );

        let object = DocumentPropertyType::Object(IndexMap::new());
        let mut reader = BufReader::new(data.as_slice());
        let err = object
            .read_optionally_from(&mut reader, true)
            .expect_err("oversized object length must be rejected, not allocated");
        assert!(
            err.to_string()
                .contains("remain in the serialized document"),
            "unexpected error: {err}"
        );
    }

    /// The guard must not reject a prefix that exactly consumes the rest of
    /// the document: that is the normal shape of a document's last field.
    #[test]
    fn test_read_optionally_from_accepts_length_prefix_equal_to_remaining_input() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let mut data = vec![2u8];
        data.extend_from_slice(b"ab");
        let mut reader = BufReader::new(data.as_slice());
        let (value, finished) = prop
            .read_optionally_from(&mut reader, true)
            .expect("exact-length prefix must decode");
        assert_eq!(value, Some(Value::Text("ab".to_string())));
        assert!(!finished);

        // One byte short of the declared length is still a rejection.
        let mut reader = BufReader::new(&data[..2]);
        let err = prop
            .read_optionally_from(&mut reader, true)
            .expect_err("short input must be rejected");
        assert!(
            err.to_string().contains("only 1 remain"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_read_optionally_from_optional_marker_none() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        // byte 0 means "not present"
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert!(value.is_none());
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_optional_marker_eof() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert!(value.is_none());
        assert!(finished); // EOF = finished
    }

    #[test]
    fn test_read_optionally_from_u64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        let data = 42u64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, finished) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U64(42)));
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_boolean_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::Boolean;
        // 0 = false
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bool(false)));
        assert!(!finished);

        // non-zero = true
        let data: &[u8] = &[1];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bool(true)));
    }

    #[test]
    fn test_read_optionally_from_string_required() {
        use integer_encoding::VarInt;
        use std::io::BufReader;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let text = b"hello";
        let mut data = text.len().encode_var_vec();
        data.extend_from_slice(text);
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Text("hello".to_string())));
    }

    #[test]
    fn test_read_optionally_from_identifier_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [7u8; 32];
        let mut reader = BufReader::new(id_bytes.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Identifier(id_bytes)));
    }

    #[test]
    fn test_read_optionally_from_f64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::F64;
        let data = 3.14f64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        if let Some(Value::Float(f)) = value {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_read_optionally_from_i128_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I128;
        let data = (-999i128).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I128(-999)));
    }

    #[test]
    fn test_read_optionally_from_u128_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U128;
        let data = 999u128.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U128(999)));
    }

    #[test]
    fn test_read_optionally_from_i64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I64;
        let data = (-42i64).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I64(-42)));
    }

    #[test]
    fn test_read_optionally_from_u32_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U32;
        let data = 100u32.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U32(100)));
    }

    #[test]
    fn test_read_optionally_from_i32_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I32;
        let data = (-100i32).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I32(-100)));
    }

    #[test]
    fn test_read_optionally_from_u16_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U16;
        let data = 300u16.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U16(300)));
    }

    #[test]
    fn test_read_optionally_from_i16_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I16;
        let data = (-300i16).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I16(-300)));
    }

    #[test]
    fn test_read_optionally_from_u8_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U8;
        let data: &[u8] = &[42];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U8(42)));
    }

    #[test]
    fn test_read_optionally_from_i8_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I8;
        let data: &[u8] = &[(-10i8) as u8];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I8(-10)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_size() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(4),
        });
        let data: &[u8] = &[1, 2, 3, 4];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![1, 2, 3, 4])));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_32() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        let data = [99u8; 32];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes32(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_variable() {
        use integer_encoding::VarInt;
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        let bytes = vec![10, 20, 30];
        let mut data = bytes.len().encode_var_vec();
        data.extend_from_slice(&bytes);
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![10, 20, 30])));
    }

    fn typed_array(item_type: DocumentPropertyType) -> DocumentPropertyType {
        DocumentPropertyType::TypedArray(TypedArrayProperty {
            item_type: Box::new(item_type),
            item_constraints: Default::default(),
            min_items: None,
            max_items: 8,
            unique_items: false,
        })
    }

    fn encode_and_read_back(property_type: &DocumentPropertyType, value: &Value) -> Vec<u8> {
        use std::io::BufReader;
        // The document serializer writes the presence flag of a property that
        // is not required itself
        let encoded = property_type
            .encode_value_ref_with_size(value, true)
            .expect("encodes");
        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, finished) = property_type
            .read_optionally_from(&mut reader, true)
            .expect("decodes");
        assert_eq!(decoded.as_ref(), Some(value), "{property_type:?}");
        assert!(!finished);
        assert!(reader.buffer().is_empty(), "{property_type:?} left bytes");

        let mut with_marker = vec![1];
        with_marker.extend(&encoded);
        let mut reader = BufReader::new(with_marker.as_slice());
        let (decoded, _) = property_type
            .read_optionally_from(&mut reader, false)
            .expect("decodes behind a presence flag");
        assert_eq!(
            decoded.as_ref(),
            Some(value),
            "{property_type:?} behind a presence flag"
        );
        encoded
    }

    #[test]
    fn should_round_trip_every_typed_array_element_type_through_encode_and_read_optionally_from() {
        for (item_type, items) in [
            (
                DocumentPropertyType::I64,
                vec![Value::I64(i64::MIN), Value::I64(-1), Value::I64(i64::MAX)],
            ),
            (
                DocumentPropertyType::U8,
                vec![Value::U8(0), Value::U8(u8::MAX)],
            ),
            (
                DocumentPropertyType::I16,
                vec![Value::I16(i16::MIN), Value::I16(1000)],
            ),
            (DocumentPropertyType::U32, vec![Value::U32(u32::MAX)]),
            (DocumentPropertyType::U128, vec![Value::U128(u128::MAX)]),
            (
                DocumentPropertyType::F64,
                vec![Value::Float(-0.5), Value::Float(1e300)],
            ),
            (
                DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(20),
                    max_bytes: None,
                }),
                vec![Value::Text("".to_string()), Value::Text("über".to_string())],
            ),
            (
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: Some(1),
                    max_size: Some(40),
                }),
                vec![Value::Bytes(vec![0xFF]), Value::Bytes(vec![7; 40])],
            ),
            // Fixed-size elements read back as the fixed-size value kinds
            (
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: Some(32),
                    max_size: Some(32),
                }),
                vec![Value::Bytes32([0x80; 32])],
            ),
            (
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: Some(3),
                    max_size: Some(3),
                }),
                vec![Value::Bytes(vec![1, 2, 3]), Value::Bytes(vec![4, 5, 6])],
            ),
            (
                DocumentPropertyType::Identifier,
                vec![Value::Identifier([1; 32]), Value::Identifier([2; 32])],
            ),
            (
                DocumentPropertyType::Boolean,
                vec![Value::Bool(true), Value::Bool(false)],
            ),
        ] {
            let property_type = typed_array(item_type);
            for items in [items.clone(), vec![]] {
                encode_and_read_back(&property_type, &Value::Array(items));
            }
        }
    }

    /// Each element is written exactly as a required scalar property of its
    /// type is written: after the varint count, an identifier is its 32 raw
    /// bytes, an integer takes its width, a fixed-size byte array is raw, and
    /// only strings and variable-size byte arrays carry a length.
    #[test]
    fn should_encode_each_typed_array_element_as_a_required_scalar_property_of_its_type() {
        let identifiers = encode_and_read_back(
            &typed_array(DocumentPropertyType::Identifier),
            &Value::Array(vec![
                Value::Identifier([0xAA; 32]),
                Value::Identifier([0xBB; 32]),
            ]),
        );
        let mut expected = vec![2];
        expected.extend([0xAA; 32]);
        expected.extend([0xBB; 32]);
        assert_eq!(identifiers, expected);

        let small_integers = encode_and_read_back(
            &typed_array(DocumentPropertyType::U8),
            &Value::Array(vec![Value::U8(7), Value::U8(200)]),
        );
        assert_eq!(small_integers, vec![2, 7, 200]);

        let wide_integers = encode_and_read_back(
            &typed_array(DocumentPropertyType::I64),
            &Value::Array(vec![Value::I64(1)]),
        );
        assert_eq!(wide_integers, vec![1, 0, 0, 0, 0, 0, 0, 0, 1]);

        let hashes = encode_and_read_back(
            &typed_array(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(3),
                max_size: Some(3),
            })),
            &Value::Array(vec![Value::Bytes(vec![1, 2, 3])]),
        );
        assert_eq!(hashes, vec![1, 1, 2, 3]);

        let blobs = encode_and_read_back(
            &typed_array(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: None,
                max_size: Some(3),
            })),
            &Value::Array(vec![Value::Bytes(vec![1, 2])]),
        );
        assert_eq!(blobs, vec![1, 2, 1, 2]);

        let strings = encode_and_read_back(
            &typed_array(DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: Some(8),
                max_bytes: None,
            })),
            &Value::Array(vec![Value::Text("ab".to_string())]),
        );
        assert_eq!(strings, vec![1, 2, b'a', b'b']);

        let flags = encode_and_read_back(
            &typed_array(DocumentPropertyType::Boolean),
            &Value::Array(vec![Value::Bool(true), Value::Bool(false)]),
        );
        assert_eq!(flags, vec![2, 1, 0]);
    }

    #[test]
    fn should_refuse_to_encode_a_typed_array_value_that_is_not_a_list_of_its_elements() {
        let identifiers = typed_array(DocumentPropertyType::Identifier);
        for value in [
            Value::Identifier([1; 32]),
            Value::Array(vec![Value::Null]),
            Value::Array(vec![Value::Text("not an identifier".to_string())]),
            Value::Array(vec![Value::Bytes(vec![1; 31])]),
        ] {
            assert!(
                identifiers
                    .encode_value_ref_with_size(&value, true)
                    .is_err(),
                "{value:?}"
            );
        }
        // An element out of a fixed size's bounds is refused, not written raw
        let hashes = typed_array(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(3),
            max_size: Some(3),
        }));
        assert!(hashes
            .encode_value_ref_with_size(&Value::Array(vec![Value::Bytes(vec![1, 2])]), true)
            .is_err());
    }

    #[test]
    fn should_refuse_a_typed_array_whose_elements_run_past_the_serialized_document() {
        use std::io::BufReader;
        // One element claimed, two of its eight bytes present
        let data: &[u8] = &[1, 2, 3];
        let mut reader = BufReader::new(data);
        let result = typed_array(DocumentPropertyType::I64).read_optionally_from(&mut reader, true);
        assert!(matches!(
            result,
            Err(DataContractError::CorruptedSerialization(_))
        ));

        // One identifier claimed, 31 of its 32 bytes present: refused as a
        // scalar identifier cut short is
        let mut data = vec![1];
        data.extend([5; 31]);
        let mut reader = BufReader::new(data.as_slice());
        assert!(typed_array(DocumentPropertyType::Identifier)
            .read_optionally_from(&mut reader, true)
            .is_err());
    }

    /// The count comes from the serialized document, so one above `maxItems`
    /// is refused before any element is read. Without that, elements of zero
    /// width (a byte array pinned to zero bytes) would let a few bytes claim
    /// a list of any length.
    #[test]
    fn should_refuse_a_serialized_typed_array_counting_more_elements_than_its_max_items() {
        use std::io::BufReader;
        for item_type in [
            DocumentPropertyType::Boolean,
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(0),
                max_size: Some(0),
            }),
        ] {
            let property_type = typed_array(item_type);
            for count in [9u64, u64::MAX] {
                let mut data = count.encode_var_vec();
                data.extend([1; 16]);
                let mut reader = BufReader::new(data.as_slice());
                let error = property_type
                    .read_optionally_from(&mut reader, true)
                    .expect_err("more elements than maxItems");
                assert!(
                    matches!(error, DataContractError::CorruptedSerialization(ref message)
                        if message.contains("more than its maxItems of 8")),
                    "{error}"
                );
            }
        }

        // maxItems zero-width elements read back as that many empty byte arrays
        let empties = typed_array(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(0),
        }));
        let data: &[u8] = &[8];
        let mut reader = BufReader::new(data);
        let (value, _) = empties
            .read_optionally_from(&mut reader, true)
            .expect("maxItems elements decode");
        assert_eq!(value, Some(Value::Array(vec![Value::Bytes(vec![]); 8])));
    }

    #[test]
    fn should_bound_a_typed_array_by_its_item_counts_times_its_element_bounds() {
        let pv = PlatformVersion::latest();
        let bounded = |item_type, min_items, max_items| {
            DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type: Box::new(item_type),
                item_constraints: Default::default(),
                min_items,
                max_items,
                unique_items: true,
            })
        };

        // Identifiers are 32 raw bytes each
        let identifiers = bounded(DocumentPropertyType::Identifier, Some(2), 64);
        assert_eq!(identifiers.min_byte_size(pv).unwrap(), Some(1 + 2 * 32));
        assert_eq!(identifiers.max_byte_size(pv).unwrap(), Some(1 + 64 * 32));

        // An integer element takes the width its bounds give it
        let small_integers = bounded(DocumentPropertyType::U8, Some(1), 10);
        assert_eq!(small_integers.min_byte_size(pv).unwrap(), Some(2));
        assert_eq!(small_integers.max_byte_size(pv).unwrap(), Some(11));

        // A fixed-size byte array carries no length; a variable one does
        let hashes = bounded(
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(20),
                max_size: Some(20),
            }),
            None,
            4,
        );
        assert_eq!(hashes.max_byte_size(pv).unwrap(), Some(1 + 4 * 20));
        let blobs = bounded(
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: None,
                max_size: Some(200),
            }),
            None,
            4,
        );
        assert_eq!(blobs.max_byte_size(pv).unwrap(), Some(1 + 4 * (2 + 200)));

        // A string element is sized as a string property is: four bytes per
        // character of its length bounds, plus its varint length
        let strings = bounded(
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(3),
                max_length: Some(40),
                max_bytes: None,
            }),
            None,
            200,
        );
        assert_eq!(strings.min_byte_size(pv).unwrap(), Some(1));
        assert_eq!(
            strings.max_byte_size(pv).unwrap(),
            Some(2 + 200 * (2 + 160))
        );

        // Unbounded, or past what a u16 holds, reports u16::MAX
        let unbounded_elements = bounded(
            DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
                max_bytes: None,
            }),
            None,
            4,
        );
        assert_eq!(
            unbounded_elements.max_byte_size(pv).unwrap(),
            Some(u16::MAX)
        );
        let saturated = bounded(
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(1),
                max_length: Some(5000),
                max_bytes: None,
            }),
            Some(1024),
            1024,
        );
        assert_eq!(saturated.max_byte_size(pv).unwrap(), Some(u16::MAX));
        assert_eq!(
            saturated.min_byte_size(pv).unwrap(),
            Some(2 + 1024 * (1 + 4))
        );
    }

    #[test]
    fn test_read_optionally_from_variable_type_array_returns_error() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let data: &[u8] = &[1, 2, 3];
        let mut reader = BufReader::new(data);
        let result = prop.read_optionally_from(&mut reader, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_value_mut_u8_from_u16() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U16(200);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(200));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u32() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U32(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(100));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u64() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U64(50);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(50));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u128() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U128(10);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(10));
    }

    #[test]
    fn test_sanitize_value_mut_u16_from_u8() {
        let prop = DocumentPropertyType::U16;
        let mut val = Value::U8(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U16(100));
    }

    #[test]
    fn test_sanitize_value_mut_u32_from_u8() {
        let prop = DocumentPropertyType::U32;
        let mut val = Value::U8(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U32(100));
    }

    #[test]
    fn test_sanitize_value_mut_u64_from_u32() {
        let prop = DocumentPropertyType::U64;
        let mut val = Value::U32(1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1000));
    }

    #[test]
    fn test_sanitize_value_mut_u128_from_u64() {
        let prop = DocumentPropertyType::U128;
        let mut val = Value::U64(1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U128(1000));
    }

    #[test]
    fn test_sanitize_value_mut_i8_from_i16() {
        let prop = DocumentPropertyType::I8;
        let mut val = Value::I16(-50);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I8(-50));
    }

    #[test]
    fn test_sanitize_value_mut_i16_from_i8() {
        let prop = DocumentPropertyType::I16;
        let mut val = Value::I8(-10);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I16(-10));
    }

    #[test]
    fn test_sanitize_value_mut_i32_from_i16() {
        let prop = DocumentPropertyType::I32;
        let mut val = Value::I16(-100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I32(-100));
    }

    #[test]
    fn test_sanitize_value_mut_i64_from_i32() {
        let prop = DocumentPropertyType::I64;
        let mut val = Value::I32(-1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(-1000));
    }

    #[test]
    fn test_sanitize_value_mut_i128_from_i64() {
        let prop = DocumentPropertyType::I128;
        let mut val = Value::I64(-50000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I128(-50000));
    }

    #[test]
    fn test_sanitize_value_mut_date_from_i64() {
        let prop = DocumentPropertyType::Date;
        let mut val = Value::I64(1648910575000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1648910575000));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_from_hex_string() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let mut val = Value::Text("deadbeef".to_string());
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_sanitize_value_mut_leaves_unrelated_type_unchanged() {
        let prop = DocumentPropertyType::U64;
        let mut val = Value::Text("hello".to_string());
        prop.sanitize_value_mut(&mut val);
        // Should not change since String doesn't match U64 sanitization
        assert_eq!(val, Value::Text("hello".to_string()));
    }

    // -----------------------------------------------------------------------
    // try_from_value_map() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_try_from_value_map_string_type() {
        let type_val = Value::Text("string".to_string());
        let min_val = Value::U64(5);
        let max_val = Value::U64(100);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minLength".to_string(), &min_val);
        map.insert("maxLength".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(5),
                max_length: Some(100),
                max_bytes: None,
            })
        );
    }

    #[test]
    fn test_try_from_value_map_boolean_type() {
        let type_val = Value::Text("boolean".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::Boolean);
    }

    #[test]
    fn test_try_from_value_map_number_type() {
        let type_val = Value::Text("number".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::F64);
    }

    #[test]
    fn test_try_from_value_map_integer_with_min_max() {
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(0);
        let max_val = Value::I64(255);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        map.insert("maximum".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_no_sized() {
        let type_val = Value::Text("integer".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: false,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_unsupported_type() {
        let type_val = Value::Text("map".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_identifier() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let media_type_val = Value::Text("application/x.dash.dpp.identifier".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("contentMediaType".to_string(), &media_type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::Identifier);
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_plain() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let min_items_val = Value::U64(10);
        let max_items_val = Value::U64(50);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("minItems".to_string(), &min_items_val);
        map.insert("maxItems".to_string(), &max_items_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(10),
                max_size: Some(50),
            })
        );
    }

    #[test]
    fn test_try_from_value_map_array_not_byte_array_errors() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(false);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_value_map_array_no_byte_array_flag_errors() {
        let type_val = Value::Text("array".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // find_integer_type helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_unsigned_integer_type_for_max_value() {
        assert_eq!(
            find_unsigned_integer_type_for_max_value(100),
            DocumentPropertyType::U8
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(255),
            DocumentPropertyType::U8
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(256),
            DocumentPropertyType::U16
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(65535),
            DocumentPropertyType::U16
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(65536),
            DocumentPropertyType::U32
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(u32::MAX as i64),
            DocumentPropertyType::U32
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(u32::MAX as i64 + 1),
            DocumentPropertyType::U64
        );
    }

    #[test]
    fn test_find_integer_type_for_min_and_max_values() {
        // positive range -> unsigned
        assert_eq!(
            find_integer_type_for_min_and_max_values(0, 255),
            DocumentPropertyType::U8
        );
        // signed ranges
        assert_eq!(
            find_integer_type_for_min_and_max_values(-128, 127),
            DocumentPropertyType::I8
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(-32768, 32767),
            DocumentPropertyType::I16
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(i32::MIN as i64, i32::MAX as i64),
            DocumentPropertyType::I32
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(i64::MIN, i64::MAX),
            DocumentPropertyType::I64
        );
    }

    // -----------------------------------------------------------------------
    // DocumentPropertyTypeParsingOptions tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parsing_options_default() {
        let opts = DocumentPropertyTypeParsingOptions::default();
        assert!(opts.sized_integer_types);
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() round-trip with read_optionally_from()
    // -----------------------------------------------------------------------

    /// Helper: encode a value with `encode_value_with_size`, then decode it
    /// with `read_optionally_from` and return the decoded value.
    fn roundtrip_encode_read(prop: &DocumentPropertyType, value: Value, required: bool) -> Value {
        let encoded = prop
            .encode_value_with_size(value, required)
            .expect("encode should succeed");
        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _finished) = prop
            .read_optionally_from(&mut reader, required)
            .expect("read should succeed");
        decoded.expect("decoded value should be Some")
    }

    #[test]
    fn test_roundtrip_u8_required() {
        let prop = DocumentPropertyType::U8;
        for val in [0u8, 1, 127, 255] {
            let decoded = roundtrip_encode_read(&prop, Value::U8(val), true);
            assert_eq!(decoded, Value::U8(val), "u8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u16_required() {
        let prop = DocumentPropertyType::U16;
        for val in [0u16, 1, 300, u16::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U16(val), true);
            assert_eq!(decoded, Value::U16(val), "u16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u32_required() {
        let prop = DocumentPropertyType::U32;
        for val in [0u32, 1, 100_000, u32::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U32(val), true);
            assert_eq!(decoded, Value::U32(val), "u32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u64_required() {
        let prop = DocumentPropertyType::U64;
        for val in [0u64, 1, 1_000_000, u64::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U64(val), true);
            assert_eq!(decoded, Value::U64(val), "u64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u128_required() {
        let prop = DocumentPropertyType::U128;
        for val in [0u128, 1, u128::MAX / 2, u128::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U128(val), true);
            assert_eq!(
                decoded,
                Value::U128(val),
                "u128 roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_roundtrip_i8_required() {
        let prop = DocumentPropertyType::I8;
        for val in [i8::MIN, -1, 0, 1, i8::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I8(val), true);
            assert_eq!(decoded, Value::I8(val), "i8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i16_required() {
        let prop = DocumentPropertyType::I16;
        for val in [i16::MIN, -1, 0, 1, i16::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I16(val), true);
            assert_eq!(decoded, Value::I16(val), "i16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i32_required() {
        let prop = DocumentPropertyType::I32;
        for val in [i32::MIN, -1, 0, 1, i32::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I32(val), true);
            assert_eq!(decoded, Value::I32(val), "i32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i64_required() {
        let prop = DocumentPropertyType::I64;
        for val in [i64::MIN, -1, 0, 1, i64::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I64(val), true);
            assert_eq!(decoded, Value::I64(val), "i64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i128_required() {
        let prop = DocumentPropertyType::I128;
        for val in [i128::MIN, -1, 0, 1, i128::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I128(val), true);
            assert_eq!(
                decoded,
                Value::I128(val),
                "i128 roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_roundtrip_f64_required() {
        let prop = DocumentPropertyType::F64;
        for val in [-1000.5f64, -1.0, 0.0, 1.0, 3.14, 1000.5] {
            let decoded = roundtrip_encode_read(&prop, Value::Float(val), true);
            if let Value::Float(f) = decoded {
                assert!(
                    (f - val).abs() < f64::EPSILON,
                    "f64 roundtrip failed for {}",
                    val
                );
            } else {
                panic!("expected float, got {:?}", decoded);
            }
        }
    }

    #[test]
    fn test_roundtrip_date_required() {
        let prop = DocumentPropertyType::Date;
        let val = 1648910575.0f64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(val), true);
        if let Value::Float(f) = decoded {
            assert!((f - val).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    #[test]
    fn test_roundtrip_boolean_true_required() {
        let prop = DocumentPropertyType::Boolean;
        // encode_value_with_size encodes true as [1], read_optionally_from
        // interprets non-zero as true
        let decoded = roundtrip_encode_read(&prop, Value::Bool(true), true);
        assert_eq!(decoded, Value::Bool(true));
    }

    #[test]
    fn test_roundtrip_boolean_false_required() {
        let prop = DocumentPropertyType::Boolean;
        // encode_value_with_size encodes false as [2], read_optionally_from
        // interprets non-zero as true -- this is the actual behavior
        let decoded = roundtrip_encode_read(&prop, Value::Bool(false), true);
        // Note: encode uses 2 for false, but read interprets any non-zero as true.
        // This documents the actual (asymmetric) behavior of the production code.
        assert_eq!(decoded, Value::Bool(true));
    }

    #[test]
    fn test_roundtrip_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let decoded = roundtrip_encode_read(&prop, Value::Text("".to_string()), true);
        assert_eq!(decoded, Value::Text("".to_string()));
    }

    #[test]
    fn test_roundtrip_string_short() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
            max_bytes: None,
        });
        let decoded = roundtrip_encode_read(&prop, Value::Text("hello world".to_string()), true);
        assert_eq!(decoded, Value::Text("hello world".to_string()));
    }

    #[test]
    fn test_roundtrip_string_long() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(1000),
            max_bytes: None,
        });
        let long_string = "a".repeat(500);
        let decoded = roundtrip_encode_read(&prop, Value::Text(long_string.clone()), true);
        assert_eq!(decoded, Value::Text(long_string));
    }

    #[test]
    fn test_roundtrip_byte_array_variable_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(100),
        });
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let decoded = roundtrip_encode_read(&prop, Value::Bytes(bytes.clone()), true);
        assert_eq!(decoded, Value::Bytes(bytes));
    }

    #[test]
    fn test_roundtrip_byte_array_empty_variable() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(100),
        });
        let decoded = roundtrip_encode_read(&prop, Value::Bytes(vec![]), true);
        assert_eq!(decoded, Value::Bytes(vec![]));
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() optional (non-required) round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_u64_optional_present() {
        let prop = DocumentPropertyType::U64;
        let decoded = roundtrip_encode_read(&prop, Value::U64(42), false);
        assert_eq!(decoded, Value::U64(42));
    }

    #[test]
    fn test_roundtrip_i64_optional_present() {
        let prop = DocumentPropertyType::I64;
        let decoded = roundtrip_encode_read(&prop, Value::I64(-999), false);
        assert_eq!(decoded, Value::I64(-999));
    }

    #[test]
    fn test_roundtrip_u32_optional_present() {
        let prop = DocumentPropertyType::U32;
        let decoded = roundtrip_encode_read(&prop, Value::U32(12345), false);
        assert_eq!(decoded, Value::U32(12345));
    }

    #[test]
    fn test_roundtrip_i32_optional_present() {
        let prop = DocumentPropertyType::I32;
        let decoded = roundtrip_encode_read(&prop, Value::I32(-12345), false);
        assert_eq!(decoded, Value::I32(-12345));
    }

    #[test]
    fn test_roundtrip_u16_optional_present() {
        let prop = DocumentPropertyType::U16;
        let decoded = roundtrip_encode_read(&prop, Value::U16(500), false);
        assert_eq!(decoded, Value::U16(500));
    }

    #[test]
    fn test_roundtrip_i16_optional_present() {
        let prop = DocumentPropertyType::I16;
        let decoded = roundtrip_encode_read(&prop, Value::I16(-500), false);
        assert_eq!(decoded, Value::I16(-500));
    }

    #[test]
    fn test_roundtrip_u8_optional_present() {
        let prop = DocumentPropertyType::U8;
        let decoded = roundtrip_encode_read(&prop, Value::U8(200), false);
        assert_eq!(decoded, Value::U8(200));
    }

    #[test]
    fn test_roundtrip_i8_optional_present() {
        let prop = DocumentPropertyType::I8;
        let decoded = roundtrip_encode_read(&prop, Value::I8(-100), false);
        assert_eq!(decoded, Value::I8(-100));
    }

    #[test]
    fn test_roundtrip_u128_optional_present() {
        let prop = DocumentPropertyType::U128;
        let decoded = roundtrip_encode_read(&prop, Value::U128(99999), false);
        assert_eq!(decoded, Value::U128(99999));
    }

    #[test]
    fn test_roundtrip_i128_optional_present() {
        let prop = DocumentPropertyType::I128;
        let decoded = roundtrip_encode_read(&prop, Value::I128(-99999), false);
        assert_eq!(decoded, Value::I128(-99999));
    }

    #[test]
    fn test_roundtrip_f64_optional_present() {
        let prop = DocumentPropertyType::F64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(2.718), false);
        if let Value::Float(f) = decoded {
            assert!((f - 2.718).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_roundtrip_date_optional_present() {
        let prop = DocumentPropertyType::Date;
        let val = 1648910575.0f64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(val), false);
        if let Value::Float(f) = decoded {
            assert!((f - val).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() for Object with nested fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_object_with_nested_fields() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                    max_bytes: None,
                }),
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        inner_fields.insert(
            "age".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        let value = Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Alice".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U32(30)),
        ]);

        let encoded = prop
            .encode_value_with_size(value, true)
            .expect("encode object should succeed");

        // Decode it back
        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _) = prop
            .read_optionally_from(&mut reader, true)
            .expect("read object should succeed");

        let decoded = decoded.expect("decoded should be Some");
        if let Value::Map(map) = decoded {
            assert_eq!(map.len(), 2);
            assert_eq!(
                map[0],
                (
                    Value::Text("name".to_string()),
                    Value::Text("Alice".to_string())
                )
            );
            assert_eq!(map[1], (Value::Text("age".to_string()), Value::U32(30)));
        } else {
            panic!("expected Map value, got {:?}", decoded);
        }
    }

    #[test]
    fn test_encode_value_with_size_object_missing_required_field() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                    max_bytes: None,
                }),
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Empty map -- missing required "name" field
        let value = Value::Map(vec![]);
        let result = prop.encode_value_with_size(value, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_object_with_optional_field_absent() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "required_field".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        inner_fields.insert(
            "optional_field".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Only provide the required field
        let value = Value::Map(vec![(
            Value::Text("required_field".to_string()),
            Value::U32(42),
        )]);

        let encoded = prop
            .encode_value_with_size(value, true)
            .expect("encode should succeed");

        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _) = prop
            .read_optionally_from(&mut reader, true)
            .expect("read should succeed");

        let decoded = decoded.expect("should decode to Some");
        if let Value::Map(map) = decoded {
            // Only the required field should be present
            assert_eq!(map.len(), 1);
            assert_eq!(
                map[0],
                (Value::Text("required_field".to_string()), Value::U32(42))
            );
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_for_tree_keys_u128() {
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result.len(), 16);
        // Should match the static encode_u128
        assert_eq!(result, DocumentPropertyType::encode_u128(42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i128() {
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result.len(), 16);
        assert_eq!(result, DocumentPropertyType::encode_i128(-42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u64() {
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(12345);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u64(12345));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i64() {
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-12345);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i64(-12345));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u32() {
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(999);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u32(999));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i32() {
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-999);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i32(-999));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u16() {
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(500);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u16(500));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i16() {
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-500);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i16(-500));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u8() {
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u8(42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i8() {
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i8(-42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_f64() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(3.14);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_float(3.14));
    }

    #[test]
    fn test_encode_value_for_tree_keys_date_timestamp() {
        let prop = DocumentPropertyType::Date;
        let val = Value::U64(1648910575000);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::encode_date_timestamp(1648910575000)
        );
    }

    #[test]
    fn test_encode_value_for_tree_keys_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id = [7u8; 32];
        let result = prop
            .encode_value_for_tree_keys(&Value::Identifier(id))
            .unwrap();
        assert_eq!(result, id.to_vec());
    }

    #[test]
    fn test_encode_value_for_tree_keys_variable_type_array_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.encode_value_for_tree_keys(&Value::Array(vec![]));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // decode_value_for_tree_keys() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_value_for_tree_keys_u128() {
        let prop = DocumentPropertyType::U128;
        let encoded = DocumentPropertyType::encode_u128(42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U128(42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i128() {
        let prop = DocumentPropertyType::I128;
        let encoded = DocumentPropertyType::encode_i128(-42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I128(-42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u32() {
        let prop = DocumentPropertyType::U32;
        let encoded = DocumentPropertyType::encode_u32(999);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U32(999));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i32() {
        let prop = DocumentPropertyType::I32;
        let encoded = DocumentPropertyType::encode_i32(-999);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I32(-999));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u16() {
        let prop = DocumentPropertyType::U16;
        let encoded = DocumentPropertyType::encode_u16(500);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U16(500));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i16() {
        let prop = DocumentPropertyType::I16;
        let encoded = DocumentPropertyType::encode_i16(-500);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I16(-500));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u8() {
        let prop = DocumentPropertyType::U8;
        let encoded = DocumentPropertyType::encode_u8(42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U8(42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i8() {
        let prop = DocumentPropertyType::I8;
        let encoded = DocumentPropertyType::encode_i8(-42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I8(-42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_f64() {
        let prop = DocumentPropertyType::F64;
        let encoded = DocumentPropertyType::encode_float(3.14);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        if let Value::Float(f) = decoded {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_decode_value_for_tree_keys_date() {
        let prop = DocumentPropertyType::Date;
        let encoded = DocumentPropertyType::encode_date_timestamp(1648910575000);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U64(1648910575000));
    }

    #[test]
    fn test_decode_value_for_tree_keys_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id = [7u8; 32];
        let decoded = prop.decode_value_for_tree_keys(&id).unwrap();
        if let Value::Identifier(decoded_id) = decoded {
            assert_eq!(decoded_id, id);
        } else {
            panic!("expected identifier");
        }
    }

    #[test]
    fn test_decode_value_for_tree_keys_variable_type_array_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // tree keys roundtrip at boundary values
    // -----------------------------------------------------------------------

    #[test]
    fn test_tree_keys_roundtrip_u64_boundary_values() {
        let prop = DocumentPropertyType::U64;
        for val in [0u64, 1, u64::MAX / 2, u64::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::U64(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::U64(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_i64_boundary_values() {
        let prop = DocumentPropertyType::I64;
        for val in [i64::MIN, -1, 0, 1, i64::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::I64(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::I64(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_u128_boundary_values() {
        let prop = DocumentPropertyType::U128;
        for val in [0u128, 1, u128::MAX / 2, u128::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::U128(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::U128(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_i128_boundary_values() {
        let prop = DocumentPropertyType::I128;
        for val in [i128::MIN, -1, 0, 1, i128::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::I128(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::I128(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let enc = prop
            .encode_value_for_tree_keys(&Value::Text("".to_string()))
            .unwrap();
        // Empty string should produce sentinel [0]
        assert_eq!(enc, vec![0]);
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, Value::Text("".to_string()));
    }

    #[test]
    fn test_tree_keys_roundtrip_string_nonempty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let enc = prop
            .encode_value_for_tree_keys(&Value::Text("test".to_string()))
            .unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, Value::Text("test".to_string()));
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_optionally_from_optional_u64_present() {
        // When required=false and marker byte is non-zero, the value follows
        let prop = DocumentPropertyType::U64;
        let mut data = vec![0xFF]; // marker: present
        data.extend_from_slice(&42u64.to_be_bytes());
        let mut reader = BufReader::new(data.as_slice());
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert_eq!(value, Some(Value::U64(42)));
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_optional_i32_present() {
        let prop = DocumentPropertyType::I32;
        let mut data = vec![0xFF]; // marker: present
        data.extend_from_slice(&(-100i32).to_be_bytes());
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert_eq!(value, Some(Value::I32(-100)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_20() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        let data = [42u8; 20];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes20(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_36() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        let data = [99u8; 36];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes36(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_non_special_size() {
        // A fixed-size byte array that is not 20, 32, or 36 should use Value::Bytes
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: Some(10),
        });
        let data = [1u8; 10];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(data.to_vec())));
    }

    #[test]
    fn test_read_optionally_from_byte_array_variable_empty() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(100),
        });
        // varint 0 means zero-length byte array
        let data = 0usize.encode_var_vec();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![])));
    }

    #[test]
    fn test_read_optionally_from_date_required() {
        let prop = DocumentPropertyType::Date;
        let data = 1648910575.0f64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        if let Some(Value::Float(f)) = value {
            assert!((f - 1648910575.0).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    #[test]
    fn test_read_optionally_from_object_with_inner_fields() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "count".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Build the serialized object: varint(object_byte_len) + object_bytes
        let object_bytes = 100u32.to_be_bytes();
        let mut data = object_bytes.len().encode_var_vec();
        data.extend_from_slice(&object_bytes);

        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        let value = value.expect("should decode object");
        if let Value::Map(map) = value {
            assert_eq!(map.len(), 1);
            assert_eq!(map[0], (Value::Text("count".to_string()), Value::U32(100)));
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // min_byte_size() / max_byte_size() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_byte_size_object_sums_sub_fields() {
        let pv = PlatformVersion::latest();
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        // 4 + 8 = 12
        assert_eq!(obj.min_byte_size(pv).unwrap(), Some(12));
    }

    #[test]
    fn test_max_byte_size_object_sums_sub_fields() {
        let pv = PlatformVersion::latest();
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U16,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::Boolean,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        // 2 + 1 = 3
        assert_eq!(obj.max_byte_size(pv).unwrap(), Some(3));
    }

    #[test]
    fn test_min_byte_size_variable_type_array_returns_none() {
        let pv = PlatformVersion::latest();
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.min_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_max_byte_size_variable_type_array_returns_none() {
        let pv = PlatformVersion::latest();
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.max_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_min_byte_size_identifier() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::Identifier.min_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    #[test]
    fn test_max_byte_size_identifier() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::Identifier.max_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() marker byte verification for optional types
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_u32_not_required_has_marker() {
        let prop = DocumentPropertyType::U32;
        let result = prop.encode_value_with_size(Value::U32(100), false).unwrap();
        assert_eq!(result.len(), 5); // 1 marker + 4 bytes
        assert_eq!(result[0], 0xFF);
        assert_eq!(&result[1..], 100u32.to_be_bytes().as_slice());
    }

    #[test]
    fn test_encode_value_with_size_i32_not_required_has_marker() {
        let prop = DocumentPropertyType::I32;
        let result = prop.encode_value_with_size(Value::I32(-50), false).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_u16_not_required_has_marker() {
        let prop = DocumentPropertyType::U16;
        let result = prop.encode_value_with_size(Value::U16(300), false).unwrap();
        assert_eq!(result.len(), 3); // 1 marker + 2 bytes
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_i16_not_required_has_marker() {
        let prop = DocumentPropertyType::I16;
        let result = prop
            .encode_value_with_size(Value::I16(-100), false)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_u8_not_required_has_marker() {
        let prop = DocumentPropertyType::U8;
        let result = prop.encode_value_with_size(Value::U8(42), false).unwrap();
        assert_eq!(result.len(), 2); // 1 marker + 1 byte
        assert_eq!(result[0], 0xFF);
        assert_eq!(result[1], 42);
    }

    #[test]
    fn test_encode_value_with_size_i8_not_required_has_marker() {
        let prop = DocumentPropertyType::I8;
        let result = prop.encode_value_with_size(Value::I8(-10), false).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_i128_not_required_has_marker() {
        let prop = DocumentPropertyType::I128;
        let result = prop
            .encode_value_with_size(Value::I128(-500), false)
            .unwrap();
        assert_eq!(result.len(), 17); // 1 marker + 16 bytes
        assert_eq!(result[0], 0xFF);
    }

    // -----------------------------------------------------------------------
    // random_value() tests - exercise branches not covered elsewhere
    // -----------------------------------------------------------------------

    use rand::SeedableRng;

    #[test]
    fn test_random_value_produces_expected_type_for_all_scalar_variants() {
        let mut rng = StdRng::seed_from_u64(1);
        assert!(matches!(
            DocumentPropertyType::U128.random_value(&mut rng),
            Value::U128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I128.random_value(&mut rng),
            Value::I128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U64.random_value(&mut rng),
            Value::U64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I64.random_value(&mut rng),
            Value::I64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U32.random_value(&mut rng),
            Value::U32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I32.random_value(&mut rng),
            Value::I32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U16.random_value(&mut rng),
            Value::U16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I16.random_value(&mut rng),
            Value::I16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U8.random_value(&mut rng),
            Value::U8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I8.random_value(&mut rng),
            Value::I8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::F64.random_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Boolean.random_value(&mut rng),
            Value::Bool(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Date.random_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Identifier.random_value(&mut rng),
            Value::Identifier(_)
        ));
    }

    #[test]
    fn test_random_value_string_respects_size_bounds() {
        let mut rng = StdRng::seed_from_u64(2);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(5),
            max_length: Some(10),
            max_bytes: None,
        });
        // Exercise several random draws
        for _ in 0..5 {
            if let Value::Text(s) = prop.random_value(&mut rng) {
                assert!(
                    s.len() >= 5 && s.len() <= 10,
                    "length out of range: {}",
                    s.len()
                );
                assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
            } else {
                panic!("expected Text variant");
            }
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_20() {
        let mut rng = StdRng::seed_from_u64(3);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        // min == max == 20 => Value::Bytes20 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes20(_) => {}
            v => panic!("expected Bytes20, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_32() {
        let mut rng = StdRng::seed_from_u64(4);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        // min == max == 32 => Value::Bytes32 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes32(_) => {}
            v => panic!("expected Bytes32, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_36() {
        let mut rng = StdRng::seed_from_u64(5);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        // min == max == 36 => Value::Bytes36 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes36(_) => {}
            v => panic!("expected Bytes36, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_other_uses_bytes() {
        let mut rng = StdRng::seed_from_u64(6);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(8),
            max_size: Some(8),
        });
        // min == max but not in the special {20, 32, 36} set => Value::Bytes
        match prop.random_value(&mut rng) {
            Value::Bytes(b) => assert_eq!(b.len(), 8),
            v => panic!("expected Bytes, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_variable_uses_bytes() {
        let mut rng = StdRng::seed_from_u64(7);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        // min != max => Value::Bytes (never Bytes20/32/36)
        for _ in 0..5 {
            match prop.random_value(&mut rng) {
                Value::Bytes(b) => {
                    assert!(!b.is_empty() && b.len() <= 10);
                }
                v => panic!("expected Bytes, got {:?}", v),
            }
        }
    }

    #[test]
    fn test_random_value_array_and_variable_type_array_return_null() {
        let mut rng = StdRng::seed_from_u64(8);
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_value_object_only_includes_required_fields() {
        let mut rng = StdRng::seed_from_u64(9);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "req".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        let val = prop.random_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(
                entries.len(),
                1,
                "only the required field should be present"
            );
            assert_eq!(entries[0].0, Value::Text("req".to_string()));
            assert!(matches!(entries[0].1, Value::U32(_)));
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // random_sub_filled_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_random_sub_filled_value_string_uses_min_size() {
        let mut rng = StdRng::seed_from_u64(10);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(7),
            max_length: Some(20),
            max_bytes: None,
        });
        if let Value::Text(s) = prop.random_sub_filled_value(&mut rng) {
            assert_eq!(s.len(), 7);
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn test_random_sub_filled_value_byte_array_uses_min_size() {
        let mut rng = StdRng::seed_from_u64(11);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(100),
        });
        if let Value::Bytes(b) = prop.random_sub_filled_value(&mut rng) {
            assert_eq!(b.len(), 4);
        } else {
            panic!("expected Bytes");
        }
    }

    #[test]
    fn test_random_sub_filled_value_object_includes_all_fields() {
        let mut rng = StdRng::seed_from_u64(12);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "req".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        // sub_filled_value includes ALL fields regardless of required flag
        let val = prop.random_sub_filled_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(entries.len(), 2);
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_random_sub_filled_value_array_returns_null() {
        let mut rng = StdRng::seed_from_u64(13);
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_sub_filled_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_sub_filled_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_sub_filled_value_date_returns_float() {
        let mut rng = StdRng::seed_from_u64(14);
        assert!(matches!(
            DocumentPropertyType::Date.random_sub_filled_value(&mut rng),
            Value::Float(_)
        ));
    }

    #[test]
    fn test_random_sub_filled_value_identifier() {
        let mut rng = StdRng::seed_from_u64(15);
        assert!(matches!(
            DocumentPropertyType::Identifier.random_sub_filled_value(&mut rng),
            Value::Identifier(_)
        ));
    }

    // -----------------------------------------------------------------------
    // random_filled_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_random_filled_value_string_uses_max_size() {
        let mut rng = StdRng::seed_from_u64(16);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(1),
            max_length: Some(12),
            max_bytes: None,
        });
        if let Value::Text(s) = prop.random_filled_value(&mut rng) {
            assert_eq!(s.len(), 12);
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn test_random_filled_value_byte_array_uses_max_size() {
        let mut rng = StdRng::seed_from_u64(17);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(9),
        });
        if let Value::Bytes(b) = prop.random_filled_value(&mut rng) {
            assert_eq!(b.len(), 9);
        } else {
            panic!("expected Bytes");
        }
    }

    #[test]
    fn test_random_filled_value_object_includes_all_fields() {
        let mut rng = StdRng::seed_from_u64(18);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U8,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::Boolean,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        let val = prop.random_filled_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(entries.len(), 2);
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_random_filled_value_scalars() {
        let mut rng = StdRng::seed_from_u64(19);
        // exhaustively exercise each scalar variant not already covered
        assert!(matches!(
            DocumentPropertyType::U128.random_filled_value(&mut rng),
            Value::U128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I128.random_filled_value(&mut rng),
            Value::I128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U64.random_filled_value(&mut rng),
            Value::U64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I64.random_filled_value(&mut rng),
            Value::I64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U32.random_filled_value(&mut rng),
            Value::U32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I32.random_filled_value(&mut rng),
            Value::I32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U16.random_filled_value(&mut rng),
            Value::U16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I16.random_filled_value(&mut rng),
            Value::I16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U8.random_filled_value(&mut rng),
            Value::U8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I8.random_filled_value(&mut rng),
            Value::I8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::F64.random_filled_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Boolean.random_filled_value(&mut rng),
            Value::Bool(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Date.random_filled_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Identifier.random_filled_value(&mut rng),
            Value::Identifier(_)
        ));
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_filled_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_filled_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_size_respects_range() {
        let mut rng = StdRng::seed_from_u64(20);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(3),
            max_length: Some(6),
            max_bytes: None,
        });
        for _ in 0..10 {
            let sz = prop.random_size(&mut rng);
            assert!((3..=6).contains(&sz));
        }
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() corrupted / truncated buffer error paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_optionally_from_u64_truncated_returns_error() {
        // Only 3 bytes but u64 needs 8
        let prop = DocumentPropertyType::U64;
        let data: &[u8] = &[0, 0, 0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i64_truncated_returns_error() {
        let prop = DocumentPropertyType::I64;
        let data: &[u8] = &[1, 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u128_truncated_returns_error() {
        let prop = DocumentPropertyType::U128;
        let data: &[u8] = &[0; 4];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i128_truncated_returns_error() {
        let prop = DocumentPropertyType::I128;
        let data: &[u8] = &[0; 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u32_truncated_returns_error() {
        let prop = DocumentPropertyType::U32;
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i32_truncated_returns_error() {
        let prop = DocumentPropertyType::I32;
        let data: &[u8] = &[0, 1];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u16_truncated_returns_error() {
        let prop = DocumentPropertyType::U16;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i16_truncated_returns_error() {
        let prop = DocumentPropertyType::I16;
        let data: &[u8] = &[7];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u8_eof_returns_error() {
        // required=true but empty buffer: u8 read must fail
        let prop = DocumentPropertyType::U8;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i8_eof_returns_error() {
        let prop = DocumentPropertyType::I8;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_f64_truncated_returns_error() {
        let prop = DocumentPropertyType::F64;
        let data: &[u8] = &[0, 0, 0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_date_truncated_returns_error() {
        let prop = DocumentPropertyType::Date;
        let data: &[u8] = &[1, 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_boolean_eof_returns_error() {
        let prop = DocumentPropertyType::Boolean;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_identifier_truncated_returns_error() {
        let prop = DocumentPropertyType::Identifier;
        // Only 16 bytes but identifier needs 32
        let data = [1u8; 16];
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_string_invalid_utf8_returns_error() {
        use integer_encoding::VarInt;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        // Valid varint length but invalid UTF-8 bytes
        let invalid_bytes = vec![0xFFu8, 0xFEu8, 0xFDu8];
        let mut data = invalid_bytes.len().encode_var_vec();
        data.extend_from_slice(&invalid_bytes);
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_string_truncated_returns_error() {
        use integer_encoding::VarInt;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        // varint says 10 bytes follow, but only provide 2
        let mut data = 10usize.encode_var_vec();
        data.push(b'a');
        data.push(b'b');
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_size_truncated_returns_error() {
        // min == max == 32, but provide only 10 bytes
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        let data = [1u8; 10];
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_object_truncated_length_returns_error() {
        use integer_encoding::VarInt;
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "x".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        // Claim 100 bytes follow but provide only 2
        let mut data = 100usize.encode_var_vec();
        data.push(0);
        data.push(0);
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_object_required_field_after_finished_buffer() {
        // Exercises the explicit "required field after finished buffer in object"
        // branch: the optional first field exhausts the inner buffer by reading
        // an absence marker from a zero-length buffer (which flips
        // `finished_buffer` to true), then the iterator sees a required field
        // with the buffer already finished and must produce a
        // CorruptedSerialization error.
        use integer_encoding::VarInt;
        let mut inner_fields = IndexMap::new();
        // First field is optional
        inner_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        // Second field is required
        inner_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Empty inner buffer: the optional-field read observes EOF on the
        // absence-marker byte, returns (None, finished = true). On the next
        // iteration, "b" is required and the buffer is finished => the
        // targeted error branch fires.
        let inner_bytes: Vec<u8> = vec![];
        let mut data = inner_bytes.len().encode_var_vec();
        data.extend_from_slice(&inner_bytes);
        let mut reader = BufReader::new(data.as_slice());
        let err = prop
            .read_optionally_from(&mut reader, true)
            .expect_err("required field with finished buffer must error");
        match err {
            DataContractError::CorruptedSerialization(msg) => {
                assert!(
                    msg.contains("required field after finished buffer in object"),
                    "expected the finished-buffer branch, got: {msg}"
                );
            }
            other => panic!("expected CorruptedSerialization, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() type-mismatch errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_boolean_type_mismatch() {
        let prop = DocumentPropertyType::Boolean;
        // U64 cannot be coerced to bool
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_object_type_mismatch() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_array_type_mismatch() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        // Not an Array value
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_f64_type_mismatch() {
        let prop = DocumentPropertyType::F64;
        // Text cannot be converted to a float
        let result = prop.encode_value_with_size(Value::Text("not a number".to_string()), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_u64_type_mismatch() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::Text("x".to_string()), true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() type-mismatch errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_string_type_mismatch() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_type_mismatch() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_type_mismatch() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_array_type_mismatch() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_missing_required_field_errors() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                    max_bytes: None,
                }),
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        let val = Value::Map(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_optional_absent_pushes_zero() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: false,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        // Missing optional field path encodes a 0 absence marker
        let val = Value::Map(vec![]);
        let encoded = prop.encode_value_ref_with_size(&val, true).unwrap();
        // The body is one byte (0), prefixed with varint length (1).
        assert_eq!(encoded, vec![1, 0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_variable_type_array_returns_error_specific() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let val = Value::Array(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Array encode roundtrip (covers array arm of encode_value_with_size and
    // encode_value_ref_with_size)
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_array_of_integers() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let val = Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        let result = prop.encode_value_with_size(val, true).unwrap();
        // varint(3) + 3 * 8 bytes
        assert_eq!(result.len(), 1 + 3 * 8);
        assert_eq!(result[0], 3);
    }

    #[test]
    fn test_encode_value_ref_with_size_array_of_integers() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let val = Value::Array(vec![Value::I64(1), Value::I64(2)]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // varint(2) + 2 * 8 bytes
        assert_eq!(result.len(), 1 + 2 * 8);
        assert_eq!(result[0], 2);
    }

    // -----------------------------------------------------------------------
    // try_from_value_map() - extra branches
    // -----------------------------------------------------------------------

    #[test]
    fn test_try_from_value_map_string_without_sizes() {
        let type_val = Value::Text("string".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
                max_bytes: None,
            })
        );
    }

    #[test]
    fn test_try_from_value_map_object_type() {
        let type_val = Value::Text("object".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert!(matches!(result, DocumentPropertyType::Object(_)));
    }

    #[test]
    fn test_try_from_value_map_integer_only_min_positive() {
        // sized, only min >= 0 => U64
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(10);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U64);
    }

    #[test]
    fn test_try_from_value_map_integer_only_min_negative() {
        // sized, only min < 0 => I64
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(-10);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_integer_only_max() {
        // sized, only max <= u8::MAX => U8
        let type_val = Value::Text("integer".to_string());
        let max_val = Value::I64(200);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("maximum".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_no_min_no_max_defaults_to_i64() {
        // sized, no min/max, no enum => I64
        let type_val = Value::Text("integer".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_integer_with_enum_min_max() {
        // sized, enum values drive the integer selection
        let type_val = Value::Text("integer".to_string());
        let enum_val = Value::Array(vec![Value::I64(0), Value::I64(1), Value::I64(255)]);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("enum".to_string(), &enum_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        // min=0, max=255 => U8
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_with_enum_single_value() {
        // A single-element enum picks the unsigned type for that max
        let type_val = Value::Text("integer".to_string());
        let enum_val = Value::Array(vec![Value::I64(300)]);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("enum".to_string(), &enum_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        // 300 => U16
        assert_eq!(result, DocumentPropertyType::U16);
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_non_identifier_media_type() {
        // Non-identifier content-media-type => falls through to ByteArray
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let media_type_val = Value::Text("application/octet-stream".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("contentMediaType".to_string(), &media_type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert!(matches!(result, DocumentPropertyType::ByteArray(_)));
    }

    // -----------------------------------------------------------------------
    // find_integer_type_for_min_and_max_values() - negative boundaries
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_integer_type_negative_small_range_is_i8() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-50, 50),
            DocumentPropertyType::I8
        );
    }

    #[test]
    fn test_find_integer_type_negative_medium_range_is_i16() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-1000, 1000),
            DocumentPropertyType::I16
        );
    }

    #[test]
    fn test_find_integer_type_negative_large_range_is_i32() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-100_000, 100_000),
            DocumentPropertyType::I32
        );
    }

    #[test]
    fn test_find_integer_type_very_large_negative_is_i64() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(i64::MIN, 0),
            DocumentPropertyType::I64
        );
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() - additional branches
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_value_mut_byte_array_from_base64_fallback() {
        // The hex decode should fail (contains +/= padding); base64 path should win
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        // "hello" in base64 is "aGVsbG8="
        let mut val = Value::Text("aGVsbG8=".to_string());
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(b"hello".to_vec()));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_size_constraint_rejects() {
        // hex of 10 bytes, but min_size is 100 => value is left unchanged
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(100),
            max_size: None,
        });
        let original = Value::Text("aabbccddee".to_string()); // 5 bytes
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original, "out-of-bounds byte array must remain text");
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_32() {
        // Fixed 32-byte hex string => Value::Bytes32 specialization
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        // 64 hex chars = 32 bytes
        let hex_str = "00".repeat(32);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes32(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_20() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        let hex_str = "ab".repeat(20);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes20(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_36() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        let hex_str = "cd".repeat(36);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes36(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_undecodable_leaves_unchanged() {
        // Neither valid hex (odd chars) nor valid base64 (has !@# chars)
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let original = Value::Text("!@#not valid!".to_string());
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original);
    }

    #[test]
    fn test_sanitize_value_mut_object_nested() {
        // Object sanitization should recurse into nested fields
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "small".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U8,
                required: true,
                transient: false,
                required_since: None,
                distinct_from: None,
                encrypted_for: None,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);

        let mut val = Value::Map(vec![(
            Value::Text("small".to_string()),
            Value::U32(200), // will be sanitized to U8
        )]);
        prop.sanitize_value_mut(&mut val);
        if let Value::Map(entries) = val {
            assert_eq!(entries[0].1, Value::U8(200));
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_sanitize_value_mut_array_elements() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        // Provide an array of Values
        let original_vals = vec![Value::I64(1), Value::I64(2)];
        let mut val = Value::Array(original_vals.clone());
        prop.sanitize_value_mut(&mut val);
        // Array path iterates every element; item_type.sanitize_value_mut is
        // defined in array.rs and shouldn't panic on well-formed input.
        if let Value::Array(items) = val {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_sanitize_value_mut_variable_type_array_elements() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![
            ArrayItemType::Integer,
            ArrayItemType::Integer,
        ]);
        let mut val = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        prop.sanitize_value_mut(&mut val);
        if let Value::Array(items) = val {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_sanitize_value_mut_u128_already_correct_unchanged() {
        let prop = DocumentPropertyType::U128;
        let mut val = Value::U128(42);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U128(42));
    }

    #[test]
    fn test_sanitize_value_mut_u8_out_of_range_unchanged() {
        let prop = DocumentPropertyType::U8;
        let original = Value::U16(300); // > u8::MAX
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        // Guard clause `n <= u8::MAX as u16` fails, so no conversion
        assert_eq!(val, original);
    }

    #[test]
    fn test_sanitize_value_mut_i8_out_of_range_unchanged() {
        let prop = DocumentPropertyType::I8;
        let original = Value::I16(500);
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original);
    }

    // -----------------------------------------------------------------------
    // Additional numeric encode/decode roundtrips at boundaries through
    // value_from_string()
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_from_string_i64_min_max() {
        let prop = DocumentPropertyType::I64;
        let min_str = i64::MIN.to_string();
        let max_str = i64::MAX.to_string();
        assert_eq!(
            prop.value_from_string(&min_str).unwrap(),
            Value::I64(i64::MIN)
        );
        assert_eq!(
            prop.value_from_string(&max_str).unwrap(),
            Value::I64(i64::MAX)
        );
    }

    #[test]
    fn test_value_from_string_u128_overflow_errors() {
        let prop = DocumentPropertyType::U128;
        // One larger than u128::MAX
        let out_of_range = "340282366920938463463374607431768211456";
        assert!(prop.value_from_string(out_of_range).is_err());
    }

    #[test]
    fn test_value_from_string_i128_overflow_errors() {
        let prop = DocumentPropertyType::I128;
        // Way too small
        let out_of_range = "-170141183460469231731687303715884105729";
        assert!(prop.value_from_string(out_of_range).is_err());
    }

    #[test]
    fn test_value_from_string_u8_negative_errors() {
        let prop = DocumentPropertyType::U8;
        assert!(prop.value_from_string("-1").is_err());
    }

    #[test]
    fn test_value_from_string_f64_invalid_errors() {
        let prop = DocumentPropertyType::F64;
        assert!(prop.value_from_string("not_a_float").is_err());
    }

    #[test]
    fn test_value_from_string_boolean_invalid_empty() {
        let prop = DocumentPropertyType::Boolean;
        assert!(prop.value_from_string("").is_err());
    }

    #[test]
    fn test_value_from_string_string_at_exact_max_len_ok() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(3),
            max_length: Some(5),
            max_bytes: None,
        });
        // Boundary: exactly min and exactly max
        assert!(prop.value_from_string("abc").is_ok());
        assert!(prop.value_from_string("abcde").is_ok());
    }

    #[test]
    fn test_value_from_string_byte_array_exact_boundaries() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(2),
            max_size: Some(4),
        });
        // 2 hex chars = 1 byte -> too small
        assert!(prop.value_from_string("ab").is_err());
        // 4 hex chars = 2 bytes -> ok
        assert!(prop.value_from_string("abcd").is_ok());
        // 8 hex chars = 4 bytes -> ok
        assert!(prop.value_from_string("aabbccdd").is_ok());
        // 10 hex chars = 5 bytes -> too big
        assert!(prop.value_from_string("aabbccddee").is_err());
    }

    // -----------------------------------------------------------------------
    // DocumentPropertyTypeParsingOptions::From<&DataContractConfig> test
    // -----------------------------------------------------------------------

    #[test]
    fn test_parsing_options_from_data_contract_config() {
        let config = DataContractConfig::default_for_version(PlatformVersion::latest())
            .expect("should create default config");
        let opts: DocumentPropertyTypeParsingOptions = (&config).into();
        // Just verify that the conversion yields the same sized_integer_types
        assert_eq!(opts.sized_integer_types, config.sized_integer_types());
    }

    // -----------------------------------------------------------------------
    // get_field_type_matching_error() - producer of ValueWrongType
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_field_type_matching_error_is_value_wrong_type() {
        let err = get_field_type_matching_error(&Value::U64(1));
        match err {
            DataContractError::ValueWrongType(msg) => {
                assert!(msg.contains("document field type"));
            }
            other => panic!("expected ValueWrongType, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Integer encode/decode roundtrips for u128 boundary values
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_i128_of_zero_roundtrip() {
        let enc = DocumentPropertyType::encode_i128(0);
        assert_eq!(DocumentPropertyType::decode_i128(&enc).unwrap(), 0);
    }

    #[test]
    fn test_encode_i128_preserves_sort_order() {
        let values: Vec<i128> = vec![i128::MIN, -100, -1, 0, 1, 100, i128::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i128(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for i128");
        }
    }

    #[test]
    fn test_encode_u128_preserves_sort_order() {
        // sort order holds in the lower half of the u128 range
        let values: Vec<u128> = vec![0, 1, 100, 1_000_000, i128::MAX as u128];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u128(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u128");
        }
    }

    #[test]
    fn test_encode_u32_preserves_sort_order() {
        let values: Vec<u32> = vec![0, 1, 100, 1_000, i32::MAX as u32];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u32(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u32");
        }
    }

    #[test]
    fn test_encode_i16_preserves_sort_order() {
        let values: Vec<i16> = vec![i16::MIN, -1000, -1, 0, 1, 1000, i16::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i16(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_encode_i8_preserves_sort_order() {
        let values: Vec<i8> = vec![i8::MIN, -100, -1, 0, 1, 100, i8::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i8(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn should_serialize_reference_metadata() {
        let property = DocumentProperty {
            property_type: DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Identity,
            ),
            required: false,
            transient: false,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
        };

        let value = serde_json::to_value(&property).expect("serialization should succeed");

        assert_eq!(
            value.get("property_type"),
            Some(&serde_json::json!({
                "IdentifierWithReference": "identity"
            }))
        );
    }

    #[test]
    fn should_serialize_key_id_reference_metadata() {
        let property = DocumentProperty {
            property_type: DocumentPropertyType::KeyIdWithReference(KeyIdReference::new(
                KeyReferenceIdentityProperty::OwnerId,
            )),
            required: false,
            transient: false,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
        };

        let value = serde_json::to_value(&property).expect("serialization should succeed");

        assert_eq!(
            value.get("property_type"),
            Some(&serde_json::json!({
                "KeyIdWithReference": { "identity_property": "$ownerId" }
            }))
        );
    }

    /// A key id with a reference is a `u32` to everything that sizes, encodes,
    /// decodes or names a property: the declaration changes what consensus
    /// checks, not the bytes.
    #[test]
    fn should_treat_a_key_id_with_reference_exactly_as_a_u32() {
        let platform_version = PlatformVersion::latest();
        let key_id = DocumentPropertyType::KeyIdWithReference(KeyIdReference::new(
            KeyReferenceIdentityProperty::OwnerId,
        ));
        let u32_type = DocumentPropertyType::U32;

        assert_eq!(key_id.name(), u32_type.name());
        assert!(key_id.is_integer());
        assert_eq!(key_id.min_size(), u32_type.min_size());
        assert_eq!(key_id.max_size(), u32_type.max_size());
        assert_eq!(
            key_id.middle_size(platform_version),
            u32_type.middle_size(platform_version)
        );
        assert_eq!(
            key_id.min_byte_size(platform_version).unwrap(),
            u32_type.min_byte_size(platform_version).unwrap()
        );
        assert_eq!(
            key_id.max_byte_size(platform_version).unwrap(),
            u32_type.max_byte_size(platform_version).unwrap()
        );

        let value = Value::U32(7);
        assert_eq!(
            key_id.encode_value_for_tree_keys(&value).unwrap(),
            u32_type.encode_value_for_tree_keys(&value).unwrap()
        );
        assert_eq!(
            key_id.encode_value_with_size(value.clone(), true).unwrap(),
            u32_type
                .encode_value_with_size(value.clone(), true)
                .unwrap()
        );
        assert_eq!(
            key_id.encode_value_ref_with_size(&value, false).unwrap(),
            u32_type.encode_value_ref_with_size(&value, false).unwrap()
        );
        let encoded = key_id.encode_value_for_tree_keys(&value).unwrap();
        assert_eq!(
            key_id.decode_value_for_tree_keys(&encoded).unwrap(),
            u32_type.decode_value_for_tree_keys(&encoded).unwrap()
        );
        assert_eq!(
            key_id.value_from_string("7").unwrap(),
            u32_type.value_from_string("7").unwrap()
        );

        let mut widened = Value::U64(7);
        key_id.sanitize_value_mut(&mut widened);
        assert_eq!(widened, Value::U32(7));
    }

    #[test]
    fn should_spell_the_key_reference_identity_property_as_the_schema_does() {
        assert_eq!(KeyReferenceIdentityProperty::OwnerId.as_str(), "$ownerId");
        assert_eq!(
            KeyReferenceIdentityProperty::OwnerId.to_string(),
            "$ownerId"
        );
        assert_eq!(
            KeyReferenceIdentityProperty::CreatorId.as_str(),
            "$creatorId"
        );
        assert_eq!(
            KeyReferenceIdentityProperty::Property("meta.toUserId".to_string()).as_str(),
            "meta.toUserId"
        );
        assert_eq!(
            KeyReferenceIdentityProperty::from_wire_name("$ownerId"),
            Some(KeyReferenceIdentityProperty::OwnerId)
        );
        assert_eq!(
            KeyReferenceIdentityProperty::from_wire_name("$creatorId"),
            Some(KeyReferenceIdentityProperty::CreatorId)
        );
        assert_eq!(
            KeyReferenceIdentityProperty::from_wire_name("toUserId"),
            Some(KeyReferenceIdentityProperty::Property(
                "toUserId".to_string()
            ))
        );
        // Other system names, the empty path and an overlong path are not admitted
        assert_eq!(KeyReferenceIdentityProperty::from_wire_name("$id"), None);
        assert_eq!(KeyReferenceIdentityProperty::from_wire_name(""), None);
        assert_eq!(
            KeyReferenceIdentityProperty::from_wire_name(&"a".repeat(257)),
            None
        );
        for name in KeyReferenceIdentityProperty::SYSTEM_WIRE_NAMES {
            assert_eq!(
                KeyReferenceIdentityProperty::from_wire_name(name).map(|p| p.as_str().to_string()),
                Some(name.to_string())
            );
        }
    }

    #[test]
    fn should_meet_a_minimum_age_from_the_recorded_creation_time_at_the_block_time() {
        let created_at: TimestampMillis = 1_700_000_000_000;
        let one_hour_ms: TimestampMillis = 3_600_000;
        // One millisecond short of the minimum is not old enough; the exact minimum is
        assert!(!ContractReferenceRequirement::minimum_age_is_met(
            Some(created_at),
            3600,
            created_at + one_hour_ms - 1
        ));
        assert!(ContractReferenceRequirement::minimum_age_is_met(
            Some(created_at),
            3600,
            created_at + one_hour_ms
        ));
        assert!(ContractReferenceRequirement::minimum_age_is_met(
            Some(created_at),
            3600,
            TimestampMillis::MAX
        ));
        // A contract that never recorded its creation time is of unknown age
        assert!(!ContractReferenceRequirement::minimum_age_is_met(
            None,
            1,
            TimestampMillis::MAX
        ));
        // The bound saturates rather than wrapping around into the past
        assert!(!ContractReferenceRequirement::minimum_age_is_met(
            Some(TimestampMillis::MAX - 1),
            u32::MAX,
            TimestampMillis::MAX - 1
        ));

        let requirements = ContractReferenceRequirements {
            moderation: None,
            minimum_age_seconds: Some(3600),
            minimum_seconds_since_update: Some(60),
            owner: None,
            readonly: None,
            keeps_history: None,
            owner_protected: None,
        };
        assert_eq!(
            requirements.requirements().collect::<Vec<_>>(),
            vec![
                ContractReferenceRequirement::MinimumAgeSeconds(3600),
                ContractReferenceRequirement::MinimumSecondsSinceUpdate(60)
            ]
        );
        assert_eq!(
            ContractReferenceRequirement::MinimumAgeSeconds(3600).field(),
            "minimumAgeSeconds"
        );
        assert_eq!(
            ContractReferenceRequirement::MinimumAgeSeconds(3600).required(),
            "3600"
        );
        assert_eq!(
            ContractReferenceRequirement::MinimumSecondsSinceUpdate(60).field(),
            "minimumSecondsSinceUpdate"
        );
        assert_eq!(
            ContractReferenceRequirement::MinimumSecondsSinceUpdate(60).required(),
            "60"
        );
    }

    #[test]
    fn should_meet_an_owner_requirement_from_the_referenced_contract_owner_and_the_writer() {
        use crate::tests::fixtures::get_dashpay_contract_fixture;

        let platform_version = PlatformVersion::latest();
        let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let owner_id = contract.owner_id();
        let someone_else = Identifier::from([0x42; 32]);
        assert_ne!(owner_id, someone_else);

        assert!(ContractReferenceOwner::Writer.is_met_by(&contract, &owner_id));
        assert!(!ContractReferenceOwner::Writer.is_met_by(&contract, &someone_else));
        assert!(!ContractReferenceOwner::Other.is_met_by(&contract, &owner_id));
        assert!(ContractReferenceOwner::Other.is_met_by(&contract, &someone_else));

        assert_eq!(ContractReferenceOwner::Writer.as_str(), "self");
        assert_eq!(ContractReferenceOwner::Other.as_str(), "other");
        assert_eq!(
            ContractReferenceOwner::from_wire_name("self"),
            Some(ContractReferenceOwner::Writer)
        );
        assert_eq!(
            ContractReferenceOwner::from_wire_name("other"),
            Some(ContractReferenceOwner::Other)
        );
        assert_eq!(ContractReferenceOwner::from_wire_name("owner"), None);

        let requirement = ContractReferenceRequirement::Owner(ContractReferenceOwner::Writer);
        assert_eq!(requirement.field(), "owner");
        assert_eq!(requirement.required(), "self");
        assert_eq!(
            ContractReferenceRequirement::Owner(ContractReferenceOwner::Other).required(),
            "other"
        );

        // The first unmet requirement is reported in declaration order: an owner requirement
        // is checked after the moderation and duration ones
        let requirements = ContractReferenceRequirements {
            owner: Some(ContractReferenceOwner::Other),
            ..Default::default()
        };
        assert_eq!(
            requirements.requirements().collect::<Vec<_>>(),
            vec![ContractReferenceRequirement::Owner(
                ContractReferenceOwner::Other
            )]
        );
        let by_owner = ReferringWrite {
            owner_id,
            block_time_ms: 0,
        };
        let by_someone_else = ReferringWrite {
            owner_id: someone_else,
            block_time_ms: 0,
        };
        assert_eq!(
            requirements.first_unmet_by(&contract, by_owner),
            Some(ContractReferenceRequirement::Owner(
                ContractReferenceOwner::Other
            ))
        );
        assert_eq!(
            requirements.first_unmet_by(&contract, by_someone_else),
            None
        );
        let both = ContractReferenceRequirements {
            minimum_age_seconds: Some(1),
            owner: Some(ContractReferenceOwner::Other),
            ..Default::default()
        };
        assert_eq!(
            both.first_unmet_by(&contract, by_owner),
            Some(ContractReferenceRequirement::MinimumAgeSeconds(1))
        );
    }

    #[test]
    fn should_meet_a_config_flag_requirement_from_the_referenced_contract_config() {
        use crate::data_contract::accessors::v0::DataContractV0Setters;
        use crate::data_contract::config::moderation::{
            ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
            ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
        };
        use crate::data_contract::config::v0::DataContractConfigSettersV0;
        use crate::tests::fixtures::get_dashpay_contract_fixture;
        use std::collections::BTreeSet;

        let platform_version = PlatformVersion::latest();
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let write = ReferringWrite {
            owner_id: Identifier::from([0x42; 32]),
            block_time_ms: 0,
        };
        let readonly = ContractReferenceRequirement::Readonly(true);
        let keeps_history = ContractReferenceRequirement::KeepsHistory(true);
        let protected = ContractReferenceRequirement::OwnerProtected(true);
        let unprotected = ContractReferenceRequirement::OwnerProtected(false);

        // The fixture is neither read-only nor keeping history, and declares no moderation:
        // it meets none of the flags, whichever value the owner protection requires
        assert!(!readonly.is_met_by(&contract, write));
        assert!(!keeps_history.is_met_by(&contract, write));
        assert!(!protected.is_met_by(&contract, write));
        assert!(!unprotected.is_met_by(&contract, write));

        let mut config = contract.config().clone();
        config.set_readonly(true);
        config.set_keeps_history(true);
        contract.set_config(config);
        assert!(readonly.is_met_by(&contract, write));
        assert!(keeps_history.is_met_by(&contract, write));

        // Appointed moderation still has no owner protection to read
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: true,
                suspensions: false,
                warnings: false,
                moderators: ContractModerators::AppointedModerators(BTreeSet::from([
                    Identifier::from([0x77; 32]),
                ])),
            },
        )));
        assert!(!protected.is_met_by(&contract, write));
        assert!(!unprotected.is_met_by(&contract, write));

        for owner_protected in [true, false] {
            contract.set_config(contract.config().clone().with_moderation(Some(
                ContractModerationConfig {
                    banlist: true,
                    suspensions: false,
                    warnings: false,
                    moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                        join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                        vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                        challenge_cool_down: Some(1_209_600),
                        election_delay: None,
                        max_added_moderators: 0,
                        moderated_document_types: BTreeMap::from([(
                            "profile".to_string(),
                            BTreeSet::from([ModerationAbility::Ban]),
                        )]),
                        interim: InterimModerators::ContractOwner,
                        owner_protected,
                    })),
                },
            )));
            assert_eq!(protected.is_met_by(&contract, write), owner_protected);
            assert_eq!(unprotected.is_met_by(&contract, write), !owner_protected);
        }

        assert_eq!(readonly.field(), "readonly");
        assert_eq!(readonly.required(), "true");
        assert_eq!(keeps_history.field(), "keepsHistory");
        assert_eq!(keeps_history.required(), "true");
        assert_eq!(protected.field(), "ownerProtected");
        assert_eq!(protected.required(), "true");
        assert_eq!(unprotected.required(), "false");

        // The contract is left read-only, keeping history and with its owner unprotected:
        // requiring the protection is the one requirement it does not meet
        let requirements = ContractReferenceRequirements {
            readonly: Some(true),
            keeps_history: Some(true),
            owner_protected: Some(true),
            ..Default::default()
        };
        assert!(!requirements.is_empty());
        assert_eq!(
            requirements.requirements().collect::<Vec<_>>(),
            vec![readonly, keeps_history, protected]
        );
        assert_eq!(
            requirements.first_unmet_by(&contract, write),
            Some(protected)
        );
        let met = ContractReferenceRequirements {
            owner_protected: Some(false),
            ..requirements
        };
        assert_eq!(met.first_unmet_by(&contract, write), None);
    }

    #[test]
    fn should_meet_election_open_when_the_contract_declares_no_delay_or_the_delay_passed() {
        use crate::data_contract::accessors::v0::DataContractV0Setters;
        use crate::data_contract::accessors::v1::DataContractV1Setters;
        use crate::data_contract::config::moderation::{
            ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
            ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
        };
        use crate::tests::fixtures::get_dashpay_contract_fixture;
        use std::collections::BTreeSet;

        let platform_version = PlatformVersion::latest();
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let created_at: TimestampMillis = 1_700_000_000_000;
        contract.set_created_at(Some(created_at));

        let elected = ContractReferenceModeration::Elected;
        let open = ContractReferenceModeration::ElectionOpen;

        // No moderation at all: neither is met
        assert!(!elected.is_met_by(&contract, created_at));
        assert!(!open.is_met_by(&contract, created_at));

        let declare = |contract: &mut DataContract, election_delay: Option<u32>| {
            let config =
                contract
                    .config()
                    .clone()
                    .with_moderation(Some(ContractModerationConfig {
                        banlist: true,
                        suspensions: false,
                        warnings: false,
                        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                            challenge_cool_down: Some(1_209_600),
                            election_delay,
                            max_added_moderators: 0,
                            moderated_document_types: BTreeMap::from([(
                                "profile".to_string(),
                                BTreeSet::from([ModerationAbility::Ban]),
                            )]),
                            interim: InterimModerators::ContractOwner,
                            owner_protected: false,
                        })),
                    }));
            contract.set_config(config);
        };

        // Elected without a delay: open at once
        declare(&mut contract, None);
        assert!(elected.is_met_by(&contract, created_at));
        assert!(open.is_met_by(&contract, created_at));

        // Elected with a delay: elected at once, open once the delay passed
        declare(&mut contract, Some(3600));
        assert!(elected.is_met_by(&contract, created_at));
        assert!(!open.is_met_by(&contract, created_at + 3_599_999));
        assert!(open.is_met_by(&contract, created_at + 3_600_000));

        // A delay on a contract of unknown age never opens
        contract.set_created_at(None);
        assert!(!open.is_met_by(&contract, TimestampMillis::MAX));

        assert_eq!(
            ContractReferenceModeration::from_wire_name("electionOpen"),
            Some(ContractReferenceModeration::ElectionOpen)
        );
        assert_eq!(
            ContractReferenceModeration::ElectionOpen.as_str(),
            "electionOpen"
        );
        assert_eq!(
            ContractReferenceRequirement::Moderation(ContractReferenceModeration::ElectionOpen)
                .required(),
            "electionOpen"
        );
    }

    #[test]
    fn should_take_the_last_change_time_from_the_later_of_creation_and_update() {
        use crate::data_contract::accessors::v1::DataContractV1Setters;
        use crate::tests::fixtures::get_dashpay_contract_fixture;

        let platform_version = PlatformVersion::latest();
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        contract.set_created_at(None);
        contract.set_updated_at(None);
        assert_eq!(
            ContractReferenceRequirement::last_change_time(&contract),
            None
        );

        contract.set_created_at(Some(1_000));
        assert_eq!(
            ContractReferenceRequirement::last_change_time(&contract),
            Some(1_000)
        );

        contract.set_updated_at(Some(5_000));
        assert_eq!(
            ContractReferenceRequirement::last_change_time(&contract),
            Some(5_000)
        );

        // A recorded update alone counts as the last change
        contract.set_created_at(None);
        assert_eq!(
            ContractReferenceRequirement::last_change_time(&contract),
            Some(5_000)
        );
    }

    #[test]
    fn should_display_reference_targets() {
        let contract_id = Identifier::from([7u8; 32]);

        assert_eq!(
            DocumentPropertyReferenceTarget::Identity.to_string(),
            "identity"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: Default::default()
            }
            .to_string(),
            "contract"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::Elected),
                    minimum_age_seconds: None,
                    minimum_seconds_since_update: None,
                    owner: None,
                    readonly: None,
                    keeps_history: None,
                    owner_protected: None,
                },
            }
            .to_string(),
            "contract with elected moderation"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::Elected),
                    minimum_age_seconds: Some(604_800),
                    minimum_seconds_since_update: Some(86_400),
                    owner: None,
                    readonly: None,
                    keeps_history: None,
                    owner_protected: None,
                },
            }
            .to_string(),
            "contract with elected moderation at least 604800 seconds old unchanged for at least 86400 seconds"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: None,
                    minimum_age_seconds: Some(1),
                    minimum_seconds_since_update: None,
                    owner: None,
                    readonly: None,
                    keeps_history: None,
                    owner_protected: None,
                },
            }
            .to_string(),
            "contract at least 1 seconds old"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    owner: Some(ContractReferenceOwner::Writer),
                    ..Default::default()
                },
            }
            .to_string(),
            "contract owned by the writer"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::Elected),
                    owner: Some(ContractReferenceOwner::Other),
                    ..Default::default()
                },
            }
            .to_string(),
            "contract with elected moderation not owned by the writer"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    readonly: Some(true),
                    keeps_history: Some(true),
                    owner_protected: Some(true),
                    ..Default::default()
                },
            }
            .to_string(),
            "contract read-only keeping history with the owner protected"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    owner: Some(ContractReferenceOwner::Other),
                    owner_protected: Some(false),
                    ..Default::default()
                },
            }
            .to_string(),
            "contract not owned by the writer with the owner unprotected"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::ElectionOpen),
                    minimum_age_seconds: None,
                    minimum_seconds_since_update: None,
                    owner: None,
                    readonly: None,
                    keeps_history: None,
                    owner_protected: None,
                },
            }
            .to_string(),
            "contract with its moderation election open"
        );
        assert_eq!(DocumentPropertyReferenceTarget::Token.to_string(), "token");
        assert_eq!(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: Some(contract_id),
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            }
            .to_string(),
            format!("permanent document (contract {contract_id}, document type note)")
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            }
            .to_string(),
            "permanent document (own contract, document type note)"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: Some(contract_id),
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            }
            .to_string(),
            format!("deletable document (contract {contract_id}, document type note)")
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            }
            .to_string(),
            "deletable document (own contract, document type note)"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: Default::default(),
                lookup: DocumentReferenceLookup {
                    index: "bySubmittedCharter".to_string(),
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                },
            }
            .to_string(),
            "permanent document (own contract, document type joinRequest, found through unique \
             index bySubmittedCharter)"
        );
    }

    #[test]
    fn should_expose_the_shared_declaration_of_both_document_references() {
        let agreement: BTreeMap<String, String> =
            [("hashtag".to_string(), "hashtag".to_string())].into();
        let permanent = DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: "note".to_string(),
            property_agreement: agreement.clone(),
        };
        let deletable = DocumentPropertyReferenceTarget::DeletableDocument {
            contract_id: None,
            document_type_name: "note".to_string(),
            property_agreement: agreement.clone(),
        };

        let permanent = permanent.as_document_reference().expect("a document");
        let deletable = deletable.as_document_reference().expect("a document");
        assert!(permanent.permanent);
        assert!(!deletable.permanent);
        for declaration in [permanent, deletable] {
            assert_eq!(declaration.contract_id, None);
            assert_eq!(declaration.document_type_name, "note");
            assert_eq!(declaration.property_agreement, &agreement);
        }
        assert!(DocumentPropertyReferenceTarget::Identity
            .as_document_reference()
            .is_none());
    }

    /// A lookup reference is a document reference, but its value is not a
    /// document id: only the accessor for every kind returns it, so code that
    /// treats the value as an id can not take it for one.
    #[test]
    fn should_return_a_lookup_reference_only_from_the_accessor_for_every_kind() {
        let lookup = DocumentReferenceLookup {
            index: "bySubmittedCharter".to_string(),
            keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
        };
        let target = DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            contract_id: None,
            document_type_name: "joinRequest".to_string(),
            property_agreement: Default::default(),
            lookup: lookup.clone(),
        };

        assert_eq!(target.as_document_reference(), None);
        let declaration = target
            .as_any_document_reference()
            .expect("a document reference");
        assert!(declaration.permanent);
        assert_eq!(declaration.document_type_name, "joinRequest");
        assert_eq!(declaration.lookup, Some(&lookup));

        // An id reference is returned by both, without a lookup
        let id_reference = DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: "joinRequest".to_string(),
            property_agreement: Default::default(),
        };
        assert_eq!(
            id_reference.as_document_reference(),
            id_reference.as_any_document_reference()
        );
        assert_eq!(
            id_reference
                .as_document_reference()
                .and_then(|declaration| declaration.lookup),
            None
        );
    }

    fn note() -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: "note".to_string(),
            property_agreement: Default::default(),
        }
    }

    fn identity_or_note() -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(vec![
            DocumentPropertyReferenceTarget::Identity,
            note(),
        ]))
    }

    /// `anyOf(note, allOf(identity, anyOf(note, identity)))`: depth 3, four
    /// leaves.
    fn nested_expression() -> DocumentPropertyReferenceTarget {
        DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(vec![
            note(),
            DocumentPropertyReferenceTarget::AllOf(ReferenceOperands::new(vec![
                DocumentPropertyReferenceTarget::Identity,
                DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(vec![
                    note(),
                    DocumentPropertyReferenceTarget::Identity,
                ])),
            ])),
        ]))
    }

    /// An expression is no document reference as a whole: code that needs one
    /// target sees none, and code that checks every declaration walks its
    /// leaves, depth first, each with where it sits; a single declaration is
    /// its own one leaf.
    #[test]
    fn should_walk_the_leaves_of_an_expression_and_expose_no_single_document_reference() {
        let expression = nested_expression();
        assert_eq!(expression.as_document_reference(), None);
        assert_eq!(expression.as_any_document_reference(), None);
        assert_eq!(expression.expression_depth(), 3);
        assert_eq!(
            expression
                .leaves_with_paths()
                .into_iter()
                .map(|(path, leaf)| (path, leaf.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("anyOf[0]".to_string(), note()),
                (
                    "anyOf[1].allOf[0]".to_string(),
                    DocumentPropertyReferenceTarget::Identity
                ),
                ("anyOf[1].allOf[1].anyOf[0]".to_string(), note()),
                (
                    "anyOf[1].allOf[1].anyOf[1]".to_string(),
                    DocumentPropertyReferenceTarget::Identity
                ),
            ]
        );
        assert_eq!(expression.leaves().len(), 4);
        assert_eq!(
            expression
                .combinator()
                .map(|(combinator, operands)| (combinator, operands.operands().len())),
            Some((ReferenceCombinator::AnyOf, 2))
        );

        let single = DocumentPropertyReferenceTarget::Identity;
        assert_eq!(single.leaves(), vec![&single]);
        assert_eq!(single.leaves_with_paths(), vec![(String::new(), &single)]);
        assert_eq!(single.expression_depth(), 0);
        assert_eq!(single.combinator(), None);
    }

    #[test]
    fn should_display_an_expression_in_declared_order() {
        assert_eq!(
            identity_or_note().to_string(),
            "any of (identity or permanent document (own contract, document type note))"
        );
        assert_eq!(
            nested_expression().to_string(),
            "any of (permanent document (own contract, document type note) or all of (identity \
             and any of (permanent document (own contract, document type note) or identity)))"
        );
    }

    /// Registration counts every leaf of an expression: each may be read for
    /// one value when the document is written.
    #[test]
    fn should_count_every_leaf_of_an_expression_as_a_reference() {
        let any_of = identity_or_note();
        assert_eq!(PropertyReference::Value(&any_of).max_references(), 2);
        assert_eq!(
            PropertyReference::Elements {
                target: &any_of,
                max_items: 15,
            }
            .max_references(),
            30
        );
        let nested = nested_expression();
        assert_eq!(PropertyReference::Value(&nested).max_references(), 4);
        let single = DocumentPropertyReferenceTarget::Identity;
        assert_eq!(PropertyReference::Value(&single).max_references(), 1);
        assert_eq!(
            PropertyReference::Elements {
                target: &single,
                max_items: 15,
            }
            .max_references(),
            15
        );
    }

    fn key_with(purpose: Purpose, contract_bounds: Option<ContractBounds>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(vec![0x74; 20]),
            read_only: false,
            disabled_at: None,
        })
    }

    #[test]
    fn should_report_the_first_unmet_key_requirement_and_what_the_key_has() {
        let contract_id = Identifier::new([3; 32]);
        let other_contract_id = Identifier::new([4; 32]);
        let requirements = IdentityKeyReferenceRequirements {
            purpose: Some(Purpose::DECRYPTION),
            bound_to: Some("submittedCharter".to_string()),
        };
        let bound_to_charter = |id: Identifier| ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "submittedCharter".to_string(),
        };

        assert!(requirements
            .first_unmet_by(
                &key_with(Purpose::DECRYPTION, Some(bound_to_charter(contract_id))),
                contract_id,
            )
            .is_none());

        // The purpose is checked first, whatever the bound
        let key = key_with(Purpose::ENCRYPTION, None);
        let unmet = requirements
            .first_unmet_by(&key, contract_id)
            .expect("the purpose is unmet");
        assert_eq!(
            unmet,
            IdentityKeyReferenceRequirement::Purpose(Purpose::DECRYPTION)
        );
        assert_eq!(unmet.field(), "purpose");
        assert_eq!(unmet.required(), "decryption");
        assert_eq!(unmet.actual_of(&key), "encryption");

        for (key, actual) in [
            (key_with(Purpose::DECRYPTION, None), "no contract bounds"),
            (
                key_with(
                    Purpose::DECRYPTION,
                    Some(ContractBounds::SingleContract { id: contract_id }),
                ),
                &format!("whole contract {contract_id}, not a document type"),
            ),
            (
                key_with(
                    Purpose::DECRYPTION,
                    Some(ContractBounds::SingleContractDocumentType {
                        id: contract_id,
                        document_type_name: "joinRequest".to_string(),
                    }),
                ),
                &format!("contract {contract_id} document type joinRequest"),
            ),
            (
                key_with(
                    Purpose::DECRYPTION,
                    Some(bound_to_charter(other_contract_id)),
                ),
                &format!("contract {other_contract_id} document type submittedCharter"),
            ),
            (
                key_with(
                    Purpose::DECRYPTION,
                    Some(ContractBounds::ContractGroup { id: contract_id }),
                ),
                &format!("contract group {contract_id}, which never meets a document type bound"),
            ),
        ] {
            let unmet = requirements
                .first_unmet_by(&key, contract_id)
                .expect("the bound is unmet");
            assert_eq!(
                unmet,
                IdentityKeyReferenceRequirement::BoundTo("submittedCharter")
            );
            assert_eq!(unmet.field(), "boundTo");
            assert_eq!(unmet.required(), "submittedCharter");
            assert_eq!(unmet.actual_of(&key), actual);
        }

        assert!(IdentityKeyReferenceRequirements::default().is_empty());
        assert!(IdentityKeyReferenceRequirements::default()
            .first_unmet_by(&key_with(Purpose::ENCRYPTION, None), contract_id)
            .is_none());
    }

    #[test]
    fn should_serialize_key_requirements_by_their_wire_names() {
        let requirements = IdentityKeyReferenceRequirements {
            purpose: Some(Purpose::DECRYPTION),
            bound_to: Some("submittedCharter".to_string()),
        };
        let json = serde_json::to_value(&requirements).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({ "purpose": "decryption", "boundTo": "submittedCharter" })
        );
        assert_eq!(
            serde_json::from_value::<IdentityKeyReferenceRequirements>(json).expect("parses"),
            requirements
        );
        assert_eq!(
            serde_json::to_value(IdentityKeyReferenceRequirements::default()).expect("serializes"),
            serde_json::json!({})
        );
        for name in ["signing", "system", "DECRYPTION"] {
            assert!(
                serde_json::from_value::<IdentityKeyReferenceRequirements>(
                    serde_json::json!({ "purpose": name })
                )
                .is_err(),
                "{name} should not deserialize as a key purpose requirement"
            );
        }
        assert_eq!(
            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property: "recipientKeyId".to_string(),
                key_requirements: requirements,
            }
            .to_string(),
            "identity public key (key id property recipientKeyId) with purpose decryption bound \
             to document type submittedCharter"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property: "recipientKeyId".to_string(),
                key_requirements: Default::default(),
            }
            .to_string(),
            "identity public key (key id property recipientKeyId)"
        );
    }

    /// A compile-time guard, not a behavioural test.
    ///
    /// `DocumentPropertyReferenceTarget` is mirrored outside this crate —
    /// notably by wasm-dpp2's `DocumentPropertyReference` TypeScript union
    /// and the conversion that builds it. Those live behind a `match` that
    /// a new variant would not break, because they can fall back to a
    /// catch-all. This exhaustive `match` has no catch-all, so adding a
    /// tenth variant fails to compile *here*, in the crate that owns the
    /// enum, where whoever adds it will see it.
    #[test]
    fn reference_targets_are_exhaustively_mirrored() {
        let targets = [
            DocumentPropertyReferenceTarget::Identity,
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: Default::default(),
            },
            DocumentPropertyReferenceTarget::Token,
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            },
            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property: "signerKeyId".to_string(),
                key_requirements: Default::default(),
            },
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
            },
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
                lookup: DocumentReferenceLookup {
                    index: "byOwner".to_string(),
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                },
            },
            DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(vec![
                DocumentPropertyReferenceTarget::Identity,
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "note".to_string(),
                    property_agreement: Default::default(),
                },
            ])),
            DocumentPropertyReferenceTarget::AllOf(ReferenceOperands::new(vec![
                DocumentPropertyReferenceTarget::Identity,
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "note".to_string(),
                    property_agreement: Default::default(),
                },
            ])),
            DocumentPropertyReferenceTarget::ListElement(ListElementReference {
                contract_id: None,
                document_type_name: "electedCharter".to_string(),
                property_agreement: [("electedCharterId".to_string(), "$id".to_string())].into(),
                in_list: "members".to_string(),
            }),
            DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: Default::default(),
                lookup: DocumentReferenceLookup {
                    index: "byOwner".to_string(),
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                },
            },
        ];

        for target in &targets {
            // No `_ =>` arm: a new variant is a compile error.
            let json_tag = match target {
                DocumentPropertyReferenceTarget::Identity => "identity",
                DocumentPropertyReferenceTarget::Contract { .. } => "contract",
                DocumentPropertyReferenceTarget::Token => "token",
                DocumentPropertyReferenceTarget::PermanentDocument { .. } => "permanentDocument",
                DocumentPropertyReferenceTarget::IdentityPublicKey { .. } => "identityPublicKey",
                DocumentPropertyReferenceTarget::DeletableDocument { .. } => "deletableDocument",
                DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. } => {
                    "permanentDocument"
                }
                DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. } => {
                    "deletableDocument"
                }
                // Not a `type`: the schema declares them under their own keys
                DocumentPropertyReferenceTarget::ListElement(_) => "listElement",
                DocumentPropertyReferenceTarget::AnyOf(_) => "anyOf",
                DocumentPropertyReferenceTarget::AllOf(_) => "allOf",
            };

            // The tag is the `refersTo` schema keyword's own `type` value
            // (or `anyOf` / `allOf`), which is what the JS surface reports
            // verbatim.
            assert!(!json_tag.is_empty());
            assert!(!target.to_string().is_empty());
        }
    }
}
