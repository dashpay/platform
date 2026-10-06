//! `findBy`, how a document reference finds the referenced document when the
//! value is not that document's id: properties of the referenced document type
//! mapped to where each value comes from on the referring side, the key of the
//! unique index whose properties are exactly those `findBy` names.
//!
//! Declared on a `permanentDocument` or `deletableDocument` `refersTo`
//! (meta-schema v3, protocol version 14):
//!
//! ```json
//! "refersTo": {
//!   "type": "permanentDocument",
//!   "documentType": "joinRequest",
//!   "findBy": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
//! }
//! ```
//!
//! reads: the value must be the owner of a `joinRequest` whose
//! `submittedCharterId` equals this document's `submittedCharterId`, found
//! through its unique index over (`submittedCharterId`, `$ownerId`). The rules
//! live here so the two places that check a declaration against its referenced
//! document type (the contract parse for a type of the same contract, the
//! registration state validation for a type of another contract) cannot drift.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV1Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property::lookup_preimage::LookupHashKey;
use crate::data_contract::document_type::property::{
    is_transient, top_level_property, DocumentPropertyReferenceTarget, DocumentPropertyType,
};

/// Why a key part moves when it is a field only the contract's moderators write
/// (`moderatorAbilities.changeFields`): no replace is needed to change it.
const MODERATORS_CHANGE: &str = "the contract's moderators change";

/// Why a key part moves when it is an immutable, optional `deletableDocument`
/// reference by id: a replace may clear it once its document is deleted.
const CLEARED_ONCE_DELETED: &str = "a replace can clear once its document is deleted";
use crate::data_contract::document_type::{DocumentTypeRef, Index};
use crate::data_contract::errors::DataContractError;
use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, CREATOR_ID, ID, OWNER_ID,
    TRANSFERRED_AT, TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::nft::TradeMode;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, Value};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::mem::Discriminant;

/// The wire name of [`LookupKeySource::ReferenceValue`].
pub const LOOKUP_REFERENCE_VALUE: &str = ".";

/// Where the value of one key part of a [`DocumentReferenceLookup`] comes from,
/// on the referring side.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub enum LookupKeySource {
    /// `"."`: the value of the property carrying the reference, or, for a
    /// reference on the elements of an array, the element. The one part of the
    /// key the reference's own value supplies.
    ReferenceValue,
    /// `"$ownerId"`: the `$ownerId` of the referring document, the writer.
    OwnerId,
    /// A property path of the referring document type. The property must be
    /// required, so the key is never missing a part.
    Property(String),
    /// A `findBy` function,
    /// `"<index property>": { "function": "sys.hash.sha256d", "params": [...] }`:
    /// the hash of a preimage the referring document reveals, see
    /// [`LookupHashKey`], the source of that index property.
    /// A lookup holding one is checked when the document is created only, and
    /// the document it finds is a commitment. Appended, so every earlier
    /// source keeps its consensus encoding.
    Hash(LookupHashKey),
}

/// The source as `findBy` spells it: `"."`, `"$ownerId"`, a property path, or
/// a computed key's `{ "function", "params" }` under the index property it
/// fills.
impl Serialize for LookupKeySource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LookupKeySource::Hash(key) => key.serialize(serializer),
            source => serializer.serialize_str(source.as_str()),
        }
    }
}

impl LookupKeySource {
    /// The source a wire name names: `"."`, `"$ownerId"` or a property path.
    /// Any other `$`-prefixed name is refused: the writer is the only system
    /// value a referring document carries that a key could need.
    pub fn from_wire_name(name: &str) -> Result<Self, DataContractError> {
        match name {
            LOOKUP_REFERENCE_VALUE => Ok(LookupKeySource::ReferenceValue),
            OWNER_ID => Ok(LookupKeySource::OwnerId),
            _ if name.starts_with('$') => {
                Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo findBy takes \".\", \"{OWNER_ID}\" or a property of the \
                     referring document type, not system property \"{name}\""
                )))
            }
            _ if name.is_empty() || name.len() > MAX_LOOKUP_PATH_LENGTH => {
                Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo findBy sources must be between 1 and {MAX_LOOKUP_PATH_LENGTH} \
                     characters"
                )))
            }
            _ => Ok(LookupKeySource::Property(name.to_string())),
        }
    }

    /// The wire name, as the schema spells it; for a computed key, which the
    /// schema spells as an object, the name of its hash.
    pub fn as_str(&self) -> &str {
        match self {
            LookupKeySource::ReferenceValue => LOOKUP_REFERENCE_VALUE,
            LookupKeySource::OwnerId => OWNER_ID,
            LookupKeySource::Property(path) => path.as_str(),
            LookupKeySource::Hash(key) => key.function.as_str(),
        }
    }
}

/// The longest index property name or source path a lookup may name, the bound
/// meta-schema v3 puts on index property names.
pub const MAX_LOOKUP_PATH_LENGTH: usize = 256;

/// The most key parts a lookup may map, the most properties an index can have.
pub const MAX_LOOKUP_KEYS: usize = 10;

/// How a document reference finds the referenced document when its value is not
/// that document's id, the reference's `findBy`: through the unique index of
/// the referenced document type whose properties are exactly the ones `keys`
/// fills ([`Self::resolve_index`]), with one key part per index property. With
/// a computed key it also holds what the reference declares, beside `findBy`,
/// of the document found: `minimumAgeBlocks` and `consume`. It serializes as
/// the parsed model, those two inside it.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct DocumentReferenceLookup {
    /// Every property of the index, by its name on the referenced side
    /// (`$ownerId` among the system ones), mapped to the referring-side source
    /// of its value, as `findBy` declares them: at most one is a computed key
    /// ([`LookupKeySource::Hash`]). The value
    /// carrying the reference fills a key part exactly once, as a `"."` source
    /// ([`LookupKeySource::ReferenceValue`]) or as a param of the computed key,
    /// except in the lookup of an `ownerRefersTo` or `creatorRefersTo` holding
    /// a computed key, where it may be left out.
    pub keys: BTreeMap<String, LookupKeySource>,
    /// With a computed key only: how many blocks before the create the
    /// document the key finds must have been created (the reference's
    /// `minimumAgeBlocks`, beside `findBy`), judged on its
    /// `$createdAtBlockHeight`, so that 1 keeps a commitment and its reveal out
    /// of the same block. The referenced document type must
    /// record `$createdAtBlockHeight`.
    #[serde(rename = "minimumAgeBlocks", skip_serializing_if = "Option::is_none")]
    pub minimum_age_blocks: Option<u32>,
    /// With a computed key only: whether the create deletes the document the
    /// key found (the reference's `consume`, beside `findBy`), in the same
    /// state transition. Only a `deletableDocument` reference whose `where`
    /// pairs the found document's `$ownerId` with the writer's may consume,
    /// into a
    /// document type of the declaring contract whose documents their owner
    /// may delete.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub consume: bool,
}

impl DocumentReferenceLookup {
    /// A lookup by `keys` with neither a minimum age nor consumption.
    pub fn new(keys: BTreeMap<String, LookupKeySource>) -> Self {
        DocumentReferenceLookup {
            keys,
            minimum_age_blocks: None,
            consume: false,
        }
    }

    /// The properties `findBy` names, joined with ", ", as errors show them.
    pub fn find_by_names(&self) -> String {
        self.keys
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The name of the unique index of `referenced` this lookup finds its
    /// document through: one whose properties are exactly the ones `keys`
    /// fills, in any order, and that does not bucket its first property
    /// (`timeRange`, `integerRange`), whose first key part is a window start no
    /// referring value names. Two unique indexes over the same properties find
    /// the same document, so the first by name is taken. A type's indexes never
    /// change once it is registered, so the answer holds for good. When none
    /// qualifies, why, naming the unique indexes the type has.
    pub fn resolve_index(&self, referenced: DocumentTypeRef) -> Result<String, String> {
        let find_by = self.find_by_names();
        let over_find_by: Vec<(&String, &Index)> = referenced
            .indexes()
            .iter()
            .filter(|(_, index)| {
                index.properties.len() == self.keys.len()
                    && index
                        .properties
                        .iter()
                        .all(|property| self.keys.contains_key(&property.name))
            })
            .collect();
        if let Some((name, _)) = over_find_by
            .iter()
            .find(|(_, index)| index.unique && index.bucketing().is_none())
        {
            return Ok(name.to_string());
        }
        if let Some((name, index)) = over_find_by.first() {
            return Err(match index.bucketing() {
                Some(bucketing) if index.unique => format!(
                    "index \"{name}\" of \"{}\" over ({find_by}) buckets its first property by \
                     a {}, so no referring value could name a key of it",
                    referenced.name(),
                    bucketing.keyword()
                ),
                _ => format!(
                    "index \"{name}\" of \"{}\" over ({find_by}) is not unique: findBy must find \
                     at most one document",
                    referenced.name()
                ),
            });
        }
        let unique: Vec<String> = referenced
            .indexes()
            .iter()
            .filter(|(_, index)| index.unique)
            .map(|(name, index)| {
                let properties: Vec<&str> = index
                    .properties
                    .iter()
                    .map(|property| property.name.as_str())
                    .collect();
                format!("{name} ({})", properties.join(", "))
            })
            .collect();
        Err(format!(
            "\"{}\" has no unique index over exactly ({find_by}): findBy must name every \
             property of one of its unique indexes and nothing else{}",
            referenced.name(),
            if unique.is_empty() {
                ", and it has none".to_string()
            } else {
                format!(" ({})", unique.join("; "))
            }
        ))
    }

    /// The computed key of this lookup, `None` when every key part is read
    /// from the referring document as is.
    pub fn hash_key(&self) -> Option<(&str, &LookupHashKey)> {
        self.keys
            .iter()
            .find_map(|(index_property, source)| match source {
                LookupKeySource::Hash(key) => Some((index_property.as_str(), key)),
                _ => None,
            })
    }

    /// How many times the value carrying the reference fills the key when it
    /// is the property at `reference_path`: as a `"."` source, or as a param of
    /// the computed key naming that path. A param `"."` is not counted: a
    /// property carrying the reference is named by its path, which
    /// [`Self::referring_side_error`] demands.
    pub fn carrier_reads(&self, reference_path: &str) -> usize {
        self.keys
            .values()
            .map(|source| match source {
                LookupKeySource::ReferenceValue => 1,
                LookupKeySource::Hash(key) => key
                    .properties_read()
                    .filter(|path| *path == reference_path)
                    .count(),
                LookupKeySource::OwnerId | LookupKeySource::Property(_) => 0,
            })
            .sum()
    }

    /// How many times the reference's own value, `"."`, fills the key: as a
    /// source or as a param of the computed key.
    pub fn reference_value_uses(&self) -> usize {
        self.keys
            .values()
            .map(|source| match source {
                LookupKeySource::ReferenceValue => 1,
                LookupKeySource::Hash(key) => key.reference_value_uses(),
                LookupKeySource::OwnerId | LookupKeySource::Property(_) => 0,
            })
            .sum()
    }

    /// Whether this lookup is judged only when the referring document is
    /// created: it holds a computed key, a commitment the create reveals.
    /// Nothing such a lookup reads can change afterwards (registration makes
    /// every stored value it reads fixed once written), and the commitment it
    /// found may be consumed or deleted, so a replace never re-validates it.
    pub fn is_checked_on_create_only(&self) -> bool {
        self.hash_key().is_some()
    }

    /// The referring document type's property paths the key reads as is, in
    /// key order: a replace that changes one of them re-validates the
    /// reference. The properties a computed key reads are left out: its
    /// lookup is checked when the document is created only.
    pub fn referring_properties(&self) -> impl Iterator<Item = &str> {
        self.keys.values().filter_map(|source| match source {
            LookupKeySource::Property(path) => Some(path.as_str()),
            LookupKeySource::ReferenceValue
            | LookupKeySource::OwnerId
            | LookupKeySource::Hash(_) => None,
        })
    }

    /// Why the referring side of this lookup, declared on the property at
    /// `reference_path` of `declaring`, cannot always assemble a key; `None`
    /// when it can. Every property source must be a stored, required, single
    /// value of the declaring type (with every object around it required too),
    /// so the lookup never runs with a missing key part and a reader can
    /// assemble the same key from the stored document; it may not be the
    /// reference property itself, whose value is `"."`. A `"$ownerId"` source
    /// needs a declaring type whose documents cannot be transferred or
    /// traded: the reference is validated when the document is written, and
    /// a transfer or a purchase would move the writer part of the key
    /// without any write, leaving a validated reference that finds nothing.
    /// With that, every part of the referring side's key changes only through
    /// a replace, which re-validates the reference.
    ///
    /// A lookup holding a computed key is checked when the document is
    /// created only, so nothing it reads may change afterwards: every property
    /// source must be fixed once written as well, the computed key's params
    /// follow [`LookupHashKey::referring_side_error`], and a property carrying
    /// it must be fixed once written, so set when the document is created
    /// (not listed under `immutable` with a condition, which could let a replace
    /// set it later unchecked), unless it is transient, never stored and read from
    /// the create alone. On a property the reference's own value must fill
    /// the key exactly once, as a `"."` source or a param of the computed key,
    /// and on a string or byte array property, `reference_type` not an
    /// identifier, as a param only, a source having to match an index
    /// property of the same kind. A param names a single property carrying
    /// the reference by its path, never `"."`, which stands only for a value
    /// without one (each element of a typed array, the writer, the creator);
    /// only the lookup of an `ownerRefersTo` or `creatorRefersTo`,
    /// `reference_path` `$ownerId` or `$creatorId`, may leave the value out
    /// beside a computed key, its declaration applying to every create
    /// whatever the writer is.
    pub fn referring_side_error(
        &self,
        declaring: DocumentTypeRef,
        reference_path: &str,
    ) -> Option<String> {
        let computed = self.is_checked_on_create_only();
        let on_the_document = reference_path == OWNER_ID || reference_path == CREATOR_ID;
        let carrier_type = declaring
            .flattened_properties()
            .get(reference_path)
            .map(|property| &property.property_type);
        // A single property carrying the reference has a path, which a param
        // names it by; `"."` stays for the values without one: each element of
        // a typed array, the writer and the creator
        let named_by_path =
            !on_the_document && !matches!(carrier_type, Some(DocumentPropertyType::TypedArray(_)));
        if named_by_path {
            if let Some((index_property, _)) = self
                .hash_key()
                .filter(|(_, key)| key.reference_value_uses() > 0)
            {
                return Some(format!(
                    "findBy function \"{index_property}\" reads \".\": a param names the \
                     property carrying the reference by its path, \"{reference_path}\""
                ));
            }
        }
        // A string or byte array carrying the reference is no identifier
        let revealed = matches!(
            carrier_type,
            Some(DocumentPropertyType::String(_) | DocumentPropertyType::ByteArray(_))
        );
        if revealed
            && self
                .keys
                .values()
                .any(|source| matches!(source, LookupKeySource::ReferenceValue))
        {
            return Some(format!(
                "\"{reference_path}\" is not an identifier, so its value fills the key as a \
                 param of a computed key, not as a source of its own"
            ));
        }
        if computed && !on_the_document {
            let reads = if named_by_path {
                self.carrier_reads(reference_path)
            } else {
                self.reference_value_uses()
            };
            if reads != 1 {
                return Some(format!(
                    "findBy reads the value of \"{reference_path}\" {reads} times: a \
                     property's findBy reads it exactly once, as \".\" or a param of its \
                     function (by its path, or as \".\" for an element), and only the findBy of \
                     an ownerRefersTo or creatorRefersTo may leave it out beside a function"
                ));
            }
            if !is_transient(declaring, reference_path)
                && !schema_property_is_fixed_once_written(declaring, reference_path)
            {
                return Some(format!(
                    "\"{reference_path}\" carries a reference found by a computed key, which is \
                     checked when the document is created only, so the property must be fixed \
                     once written and set then (make the type immutable or list the property \
                     under `immutable` without a condition)"
                ));
            }
        }
        for (index_property, source) in &self.keys {
            let path = match source {
                LookupKeySource::ReferenceValue => continue,
                LookupKeySource::Hash(key) => {
                    if let Some(reason) = key.referring_side_error(declaring) {
                        return Some(format!("findBy function \"{index_property}\" {reason}"));
                    }
                    continue;
                }
                LookupKeySource::OwnerId => {
                    if owner_can_change(declaring) {
                        return Some(format!(
                            "findBy \"{index_property}\" reads \"$ownerId\", which a transfer or \
                             a purchase of the referring document changes without re-validating \
                             the reference: findBy may read the writer only on a document type \
                             that cannot be transferred or traded"
                        ));
                    }
                    continue;
                }
                LookupKeySource::Property(path) => path,
            };
            if path == reference_path {
                return Some(format!(
                    "findBy \"{index_property}\" names the reference property itself: write \
                     \".\" for the reference's own value"
                ));
            }
            let Some(property) = declaring.flattened_properties().get(path) else {
                return Some(format!(
                    "findBy \"{index_property}\" reads \"{path}\", which is not a property of \
                     the referring document type"
                ));
            };
            if matches!(
                property.property_type,
                DocumentPropertyType::TypedArray(_)
                    | DocumentPropertyType::Array(_)
                    | DocumentPropertyType::VariableTypeArray(_)
                    | DocumentPropertyType::Object(_)
            ) {
                return Some(format!(
                    "findBy \"{index_property}\" reads \"{path}\", which is not a single value"
                ));
            }
            if is_transient(declaring, path) {
                return Some(format!(
                    "findBy \"{index_property}\" reads \"{path}\", which is transient or inside \
                     a transient object: the key must be readable from the stored document"
                ));
            }
            // A required leaf inside an optional object is only present when the
            // object is, so every ancestor must be required as well
            let mut prefix = String::new();
            for segment in path.split('.') {
                if !prefix.is_empty() {
                    prefix.push('.');
                }
                prefix.push_str(segment);
                if !declaring.required_fields().contains(&prefix) {
                    return Some(format!(
                        "findBy \"{index_property}\" reads \"{path}\", which is not required: \
                         findBy never runs with a missing key part, so every property it reads \
                         (and every object around it) must be listed in `required`"
                    ));
                }
            }
            if computed && !schema_property_is_fixed_once_written(declaring, path) {
                return Some(format!(
                    "findBy \"{index_property}\" reads \"{path}\", which a replace can change: \
                     a computed key is checked when the document is created only, so every \
                     property findBy reads must be fixed once written (make the type immutable \
                     or list the property under `immutable`)"
                ));
            }
        }
        None
    }

    /// Why this lookup, declared on a property of `declaring`, cannot resolve
    /// in `referenced`, the referenced document type; `None` when it can. The
    /// properties `findBy` names must be exactly those of a unique index that
    /// does not bucket its first property ([`Self::resolve_index`]), so the key
    /// finds at most one document; the referenced type may not be `indexOnly`;
    /// no index property may be transient, a value no stored document holds;
    /// and each source must hold the same kind of value as the index property
    /// it fills, or no document could ever match. The key must also stay with
    /// the document it found, see [`Self::moving_key_part`].
    pub fn referenced_side_error(
        &self,
        declaring: DocumentTypeRef,
        referenced: DocumentTypeRef,
    ) -> Option<String> {
        let index_name = match self.resolve_index(referenced) {
            Ok(index_name) => index_name,
            Err(reason) => return Some(reason),
        };
        let index = referenced.indexes().get(&index_name)?;
        if referenced.index_only() {
            return Some(format!(
                "\"{}\" is an indexOnly document type, which findBy cannot reference",
                referenced.name()
            ));
        }
        // A transient value is never stored, so no document would ever sit in
        // the index under a key naming it
        if let Some(transient) = index
            .properties
            .iter()
            .find(|property| is_transient(referenced, &property.name))
        {
            return Some(format!(
                "findBy names \"{}\" of \"{}\", which is transient or inside a transient \
                 object: its value is never stored, so findBy could never find a document",
                transient.name,
                referenced.name()
            ));
        }
        for (index_property, source) in &self.keys {
            let Some(indexed_kind) = index_property_value_kind(referenced, index_property) else {
                return Some(format!(
                    "index property \"{index_property}\" is not a property of \"{}\"",
                    referenced.name()
                ));
            };
            let source_kind = match source {
                // Both are identifiers, the reference's own value because
                // `refersTo` is only allowed on identifiers
                LookupKeySource::ReferenceValue | LookupKeySource::OwnerId => {
                    std::mem::discriminant(&DocumentPropertyType::Identifier)
                }
                // A hash is a byte array exactly as long as the hash's output,
                // which the index property must hold
                LookupKeySource::Hash(key) => {
                    let length = key.function.output_length();
                    let holds_hash = matches!(
                        referenced
                            .flattened_properties()
                            .get(index_property)
                            .map(|property| &property.property_type),
                        Some(DocumentPropertyType::ByteArray(sizes))
                            if sizes.min_size == Some(length) && sizes.max_size == Some(length)
                    );
                    if !holds_hash {
                        return Some(format!(
                            "findBy function \"{index_property}\" is a {} hash, so the index \
                             property must be a byte array of exactly {length} bytes",
                            key.function.as_str()
                        ));
                    }
                    continue;
                }
                LookupKeySource::Property(path) => {
                    let Some(property) = declaring.flattened_properties().get(path) else {
                        return Some(format!(
                            "findBy \"{index_property}\" reads \"{path}\", which is not a \
                             property of the referring document type"
                        ));
                    };
                    property.property_type.value_kind()
                }
            };
            if source_kind != indexed_kind {
                return Some(format!(
                    "findBy \"{index_property}\" is filled from \"{}\", which holds a different \
                     kind of value: no document could ever match",
                    source.as_str()
                ));
            }
        }
        // A minimum age is judged against the found document's creation height,
        // so the type must record one; consuming deletes the document as its
        // own owner would, which the type must allow, or keep for consumption
        // alone (`canBeDeleted: "onlyWhenConsumed"`)
        if self.minimum_age_blocks.is_some()
            && !referenced
                .required_fields()
                .contains(CREATED_AT_BLOCK_HEIGHT)
        {
            return Some(format!(
                "minimumAgeBlocks is judged against the found document's \
                 {CREATED_AT_BLOCK_HEIGHT}, which \"{}\" does not record: list \
                 {CREATED_AT_BLOCK_HEIGHT} in its `required`",
                referenced.name()
            ));
        }
        if self.consume
            && !referenced.documents_can_be_deleted()
            && !referenced.documents_deleted_only_when_consumed()
        {
            return Some(format!(
                "consume deletes the found document as its owner would, which \"{}\" does not \
                 allow (`canBeDeleted: false`): set `canBeDeleted` to true, or to \
                 \"onlyWhenConsumed\" for documents only a consume deletes",
                referenced.name()
            ));
        }
        // The storage layer refuses to delete a document whose type keeps history. A full
        // parse, the only one this check runs under, already refuses such a type that allows
        // deletion; the rule is kept here too, as `ttl` and moderator deletion keep theirs
        if self.consume && referenced.documents_keep_history() {
            return Some(format!(
                "consume deletes the found document, which the storage layer refuses for \
                 \"{}\": its documents keep history",
                referenced.name()
            ));
        }
        // Consuming deletes the found document without a delete transition of its
        // own: no delete token cost is charged, no delete action fee, and the key
        // signing the create is only held to the creating type's security level.
        // A type whose delete costs something, or asks a stricter key than the
        // creating type, may not be consumed
        if self.consume
            && (referenced.document_deletion_token_cost().is_some()
                || referenced
                    .action_fees()
                    .is_some_and(|fees| fees.document_deletion_action_fee().is_some()))
        {
            return Some(format!(
                "consume deletes the found document without a delete transition, so \"{}\" \
                 may declare no delete token cost and no delete action fee: neither would be \
                 charged",
                referenced.name()
            ));
        }
        if self.consume
            && referenced.security_level_requirement() < declaring.security_level_requirement()
        {
            return Some(format!(
                "consume deletes the found document with the key that signs the create, so \
                 \"{}\" may not require a stricter signature security level ({:?}) than \"{}\" \
                 ({:?})",
                referenced.name(),
                referenced.security_level_requirement(),
                declaring.name(),
                declaring.security_level_requirement()
            ));
        }
        if let Some((index_property, why)) = self.moving_key_part(referenced) {
            // A field only moderators write is never fixed, whatever the type says
            let hint = if why == MODERATORS_CHANGE {
                "find it by a property only its owner writes"
            } else if why == CLEARED_ONCE_DELETED {
                "make the reference required or find it by a property that is no optional \
                 `deletableDocument` reference"
            } else {
                "make the type immutable or list the property under `immutable`"
            };
            return Some(format!(
                "findBy names \"{index_property}\" of \"{}\", which {why}: findBy must keep \
                 finding the document it found, so every property it names must be fixed once \
                 the document is written ({hint})",
                referenced.name()
            ));
        }
        None
    }

    /// The first key part a referenced document of `referenced` can change
    /// after it was written, with how, `None` when every part is fixed for
    /// good. A key must not move off the document it found, or a document
    /// validated against it would later find nothing; a `permanentDocument`
    /// reference promises it never dangles, and a `deletableDocument` one that
    /// the document it found is the one it checks again. A schema property is
    /// fixed on a type whose documents are
    /// immutable or when its top-level property is listed under `immutable`
    /// without a condition (one listed with a condition may change while it
    /// does not hold); `$ownerId` is
    /// fixed unless documents can be transferred or traded; `$id`,
    /// `$creatorId` and the creation times never change; the update and
    /// transfer times change with the document. Every flag read here is
    /// immutable on contract update, and the `immutable` list may only grow,
    /// so the answer holds for good.
    fn moving_key_part(&self, referenced: DocumentTypeRef) -> Option<(&str, &'static str)> {
        self.keys.keys().find_map(|index_property| {
            why_value_can_change(referenced, index_property)
                .map(|why| (index_property.as_str(), why))
        })
    }

    /// The equality values of the key the lookup assembles for one write, by
    /// index property name: `reference_value` for `"."` (the identifier
    /// property's value, one array element, the writer or the creator),
    /// `owner_id` for `"$ownerId"`, the value at each source path in
    /// `document_data`, and `computed_key`, the hash the caller computed and
    /// billed, for a computed key: this never hashes, so a hash is never taken
    /// unbilled. `None` when a source path holds no value or does not resolve,
    /// or a computed key has no `computed_key`. Registration admits required
    /// sources only, so a value is missing only on a document stamped before
    /// the property became required (`requiredSince`) or one the schema
    /// validation refuses anyway; a create whose preimage cannot be assembled
    /// is refused before any read (`DocumentReferencePreimageInvalidError`).
    /// Either way a key with a missing part finds no document.
    pub fn key_values(
        &self,
        reference_value: Identifier,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        computed_key: Option<&Value>,
    ) -> Option<BTreeMap<String, Value>> {
        self.keys
            .iter()
            .map(|(index_property, source)| {
                let value = match source {
                    LookupKeySource::ReferenceValue => {
                        Value::Identifier(reference_value.to_buffer())
                    }
                    LookupKeySource::OwnerId => Value::Identifier(owner_id.to_buffer()),
                    LookupKeySource::Property(path) => {
                        document_data.get_optional_at_path(path).ok()??.clone()
                    }
                    LookupKeySource::Hash(_) => computed_key?.clone(),
                };
                Some((index_property.clone(), value))
            })
            .collect()
    }
}

/// Whether a document of `document_type` can change owner after it was
/// written, by a transfer or a purchase. Both flags are immutable on contract
/// update, so the answer holds for good. Public for the document reference
/// validation, which re-checks a `contract` reference's `owner` requirement
/// on every replace of such a document: the replace may be written by an
/// owner the requirement never checked.
pub fn owner_can_change(document_type: DocumentTypeRef) -> bool {
    document_type.documents_transferable().is_transferable()
        || document_type.trade_mode() != TradeMode::None
}

/// How the value `property` names, of a document of `referenced`, can change
/// after the document is written, `None` when it is fixed for good: `$id`,
/// `$creatorId` and the creation times never change; `$ownerId` and the
/// transfer times change with a transfer or a purchase, the update times also
/// with a replace; a schema property is fixed when
/// [`schema_property_is_fixed_once_written`] says so and a replace cannot
/// clear it ([`clearable_once_its_document_is_deleted`]), and its moderators'
/// fields are named as such. Shared by a lookup's key and the
/// `summableOffCountIndex` lossless rule, so the two judge a value alike.
pub(crate) fn why_value_can_change(
    referenced: DocumentTypeRef,
    property: &str,
) -> Option<&'static str> {
    let changes_owner = owner_can_change(referenced);
    match property {
        ID | CREATOR_ID | CREATED_AT | CREATED_AT_BLOCK_HEIGHT | CREATED_AT_CORE_BLOCK_HEIGHT => {
            None
        }
        OWNER_ID => changes_owner.then_some("a transfer or a purchase changes"),
        UPDATED_AT | UPDATED_AT_BLOCK_HEIGHT | UPDATED_AT_CORE_BLOCK_HEIGHT => {
            (referenced.documents_mutable() || changes_owner)
                .then_some("a replace, a transfer or a purchase changes")
        }
        TRANSFERRED_AT | TRANSFERRED_AT_BLOCK_HEIGHT | TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
            changes_owner.then_some("a transfer or a purchase changes")
        }
        property => {
            if referenced
                .moderator_changeable_fields()
                .contains(top_level_property(property))
            {
                Some(MODERATORS_CHANGE)
            } else if !schema_property_is_fixed_once_written(referenced, property) {
                Some("a replace can change")
            } else {
                clearable_once_its_document_is_deleted(referenced, property)
                    .then_some(CLEARED_ONCE_DELETED)
            }
        }
    }
}

/// Whether the schema property at `path` of a document of `document_type` can
/// never change once the document is written: the type is immutable
/// (`documentsMutable: false`), or the property's top-level property is listed
/// under `immutable`, and in either case the contract's moderators do not
/// write it (`moderatorAbilities.changeFields`, which the parser keeps apart
/// from `immutable`). A property listed under `immutable` with a condition is
/// not fixed: a replace may change it while its condition does not hold. The
/// flags and the moderators' fields are immutable on contract update and the
/// properties `immutable` lists without a condition may only grow, so the
/// answer holds for good. The one rule
/// both a lookup's key parts and a list element's list are judged by.
pub(crate) fn schema_property_is_fixed_once_written(
    document_type: DocumentTypeRef,
    path: &str,
) -> bool {
    let top_level = top_level_property(path);
    (!document_type.documents_mutable() || document_type.immutable_fields().contains(top_level))
        && !document_type
            .moderator_changeable_fields()
            .contains(top_level)
}

/// Whether a replace may clear the top-level property of `path` of a document
/// of `document_type` though `immutable` lists it: an optional
/// `deletableDocument` reference by id, which document replace state
/// validation 1 lets a replace clear once its document is deleted, so that the
/// document can still be replaced. A required one is never cleared: replace
/// advanced structure validation refuses a document missing a required
/// property before state validation reads the clear. Clearing changes the
/// value to absent, so a value that must stay as written (a lookup's key part,
/// a param or `where` value a findBy function's key is judged by on the create,
/// a value a `summableOffCountIndex` index's group is fixed by) may not be one.
pub(crate) fn clearable_once_its_document_is_deleted(
    document_type: DocumentTypeRef,
    path: &str,
) -> bool {
    document_type.documents_mutable()
        && document_type
            .flattened_properties()
            .get(top_level_property(path))
            .is_some_and(|property| {
                !property.required
                    && matches!(
                        property.property_type,
                        DocumentPropertyType::IdentifierWithReference(
                            DocumentPropertyReferenceTarget::DeletableDocument { .. }
                        )
                    )
            })
}

/// The kind of value an index property of `document_type` holds: a system
/// property's fixed type, or the schema property's. `None` when the name is
/// neither.
fn index_property_value_kind(
    document_type: DocumentTypeRef,
    name: &str,
) -> Option<Discriminant<DocumentPropertyType>> {
    let system_type = match name {
        ID | OWNER_ID | CREATOR_ID => Some(DocumentPropertyType::Identifier),
        CREATED_AT | UPDATED_AT | TRANSFERRED_AT => Some(DocumentPropertyType::Date),
        CREATED_AT_BLOCK_HEIGHT | UPDATED_AT_BLOCK_HEIGHT | TRANSFERRED_AT_BLOCK_HEIGHT => {
            Some(DocumentPropertyType::U64)
        }
        CREATED_AT_CORE_BLOCK_HEIGHT
        | UPDATED_AT_CORE_BLOCK_HEIGHT
        | TRANSFERRED_AT_CORE_BLOCK_HEIGHT => Some(DocumentPropertyType::U32),
        _ => None,
    };
    match system_type {
        Some(system_type) => Some(system_type.value_kind()),
        None => document_type
            .flattened_properties()
            .get(name)
            .map(|property| property.property_type.value_kind()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;

    fn identifier() -> Value {
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32u32,
            "maxItems": 32u32,
            "contentMediaType": "application/x.dash.dpp.identifier"
        })
    }

    fn with_position(mut property: Value, position: u32) -> Value {
        property
            .insert("position".to_string(), Value::U32(position))
            .expect("the position inserts");
        property
    }

    fn document_type(name: &str, schema: Value) -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");
        DocumentType::try_from_schema(
            Identifier::from([1; 32]),
            1,
            config.version(),
            name,
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("the document type should parse")
    }

    /// A `joinRequest` with a unique (`submittedCharterId`, `$ownerId`) index
    /// and a non-unique `byNote` index, immutable and not transferable, with
    /// `extra` merged into its schema.
    fn join_request_with(extra: Value) -> DocumentType {
        let mut schema = platform_value!({
            "type": "object",
            "canBeDeleted": false,
            "documentsMutable": false,
            "properties": {
                "submittedCharterId": with_position(identifier(), 0),
                "note": { "type": "string", "maxLength": 32u32, "position": 1u32 }
            },
            "indices": [
                {
                    "name": "bySubmittedCharter",
                    "properties": [{ "submittedCharterId": "asc" }, { "$ownerId": "asc" }],
                    "unique": true
                },
                { "name": "byNote", "properties": [{ "note": "asc" }] }
            ],
            "required": ["submittedCharterId"],
            "additionalProperties": false
        });
        merge_keywords(&mut schema, extra);
        document_type("joinRequest", schema)
    }

    /// Merges the top-level keywords of `extra` into `schema`.
    fn merge_keywords(schema: &mut Value, extra: Value) {
        if let Value::Map(entries) = extra {
            for (key, value) in entries {
                let key = key.to_text().expect("a text key");
                schema.insert(key, value).expect("the keyword inserts");
            }
        }
    }

    fn join_request() -> DocumentType {
        join_request_with(platform_value!({}))
    }

    /// An `electedCharter` with a required `submittedCharterId`, an optional
    /// `optionalCharterId`, a string `note` and the reference `memberId`, with
    /// `extra` merged into its schema.
    fn elected_charter_with(extra: Value) -> DocumentType {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "submittedCharterId": with_position(identifier(), 0),
                "optionalCharterId": with_position(identifier(), 1),
                "note": { "type": "string", "maxLength": 32u32, "position": 2u32 },
                "memberId": with_position(identifier(), 3)
            },
            "required": ["submittedCharterId", "note"],
            "additionalProperties": false
        });
        merge_keywords(&mut schema, extra);
        document_type("electedCharter", schema)
    }

    fn elected_charter() -> DocumentType {
        elected_charter_with(platform_value!({}))
    }

    fn lookup(keys: &[(&str, &str)]) -> DocumentReferenceLookup {
        DocumentReferenceLookup {
            keys: keys
                .iter()
                .map(|(index_property, source)| {
                    (
                        index_property.to_string(),
                        LookupKeySource::from_wire_name(source).expect("a valid source"),
                    )
                })
                .collect(),
            minimum_age_blocks: None,
            consume: false,
        }
    }

    #[test]
    fn should_accept_a_lookup_covering_a_unique_index_with_sources_of_the_right_kind() {
        let lookup = lookup(&[
            ("submittedCharterId", "submittedCharterId"),
            ("$ownerId", "."),
        ]);
        let declaring = elected_charter();
        assert_eq!(
            lookup.referring_side_error(declaring.as_ref(), "memberId",),
            None
        );
        assert_eq!(
            lookup.referenced_side_error(declaring.as_ref(), join_request().as_ref()),
            None
        );
    }

    #[test]
    fn should_resolve_the_unique_index_over_exactly_the_properties_find_by_names() {
        // In either order: the index is found by its set of properties
        for keys in [
            [
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ],
            [
                ("$ownerId", "."),
                ("submittedCharterId", "submittedCharterId"),
            ],
        ] {
            assert_eq!(
                lookup(&keys).resolve_index(join_request().as_ref()),
                Ok("bySubmittedCharter".to_string())
            );
        }
    }

    #[test]
    fn should_refuse_find_by_naming_no_unique_index_exactly() {
        let declaring = elected_charter();
        for (keys, fragment) in [
            // Fewer or more properties than the unique index, or an index not
            // there at all, and the unique indexes the type has are listed
            (
                vec![("$ownerId", ".")],
                "\"joinRequest\" has no unique index over exactly ($ownerId): findBy must name \
                 every property of one of its unique indexes and nothing else \
                 (bySubmittedCharter (submittedCharterId, $ownerId))",
            ),
            (
                vec![
                    ("submittedCharterId", "submittedCharterId"),
                    ("$ownerId", "."),
                    ("note", "note"),
                ],
                "has no unique index over exactly ($ownerId, note, submittedCharterId)",
            ),
            (
                vec![("note", "."), ("x", "note")],
                "has no unique index over exactly (note, x)",
            ),
            // Exactly the properties of an index that is not unique
            (
                vec![("note", ".")],
                "index \"byNote\" of \"joinRequest\" over (note) is not unique",
            ),
        ] {
            let error = lookup(&keys)
                .referenced_side_error(declaring.as_ref(), join_request().as_ref())
                .expect("findBy should be refused");
            assert!(error.contains(fragment), "{keys:?}: {error}");
        }
    }

    #[test]
    fn should_refuse_a_source_of_the_wrong_value_kind() {
        let declaring = elected_charter();
        let error = lookup(&[("submittedCharterId", "note"), ("$ownerId", ".")])
            .referenced_side_error(declaring.as_ref(), join_request().as_ref())
            .expect("the string source should be refused");
        assert!(error.contains("holds a different kind of value"), "{error}");
    }

    #[test]
    fn should_refuse_an_optional_missing_or_self_referring_source() {
        let declaring = elected_charter();
        for (source, fragment) in [
            ("optionalCharterId", "which is not required"),
            (
                "ghostId",
                "which is not a property of the referring document type",
            ),
            ("memberId", "names the reference property itself"),
        ] {
            let error = lookup(&[("submittedCharterId", source), ("$ownerId", ".")])
                .referring_side_error(declaring.as_ref(), "memberId")
                .expect("the source should be refused");
            assert!(error.contains(fragment), "{source}: {error}");
        }
    }

    #[test]
    fn should_refuse_a_lookup_whose_key_the_referenced_document_can_move() {
        let lookup = lookup(&[
            ("submittedCharterId", "submittedCharterId"),
            ("$ownerId", "."),
        ]);
        let declaring = elected_charter();
        let refused = |referenced: DocumentType| {
            lookup.referenced_side_error(declaring.as_ref(), referenced.as_ref())
        };

        // A replace could move `submittedCharterId`, a transfer or a purchase `$ownerId`
        for (extra, moving) in [
            (
                platform_value!({ "documentsMutable": true }),
                "findBy names \"submittedCharterId\" of \"joinRequest\", which a replace can change",
            ),
            (
                platform_value!({ "transferable": 1u8 }),
                "findBy names \"$ownerId\" of \"joinRequest\", which a transfer or a purchase \
                 changes",
            ),
            (
                platform_value!({ "tradeMode": 1u8 }),
                "findBy names \"$ownerId\" of \"joinRequest\", which a transfer or a purchase \
                 changes",
            ),
        ] {
            let error =
                refused(join_request_with(extra.clone())).expect("a moving key should be refused");
            assert!(error.contains(moving), "{extra:?}: {error}");
        }

        // Frozen by the `immutable` list on a mutable type, the key holds
        assert_eq!(
            refused(join_request_with(platform_value!({
                "documentsMutable": true,
                "immutable": ["submittedCharterId"]
            }))),
            None
        );
    }

    /// A registration refuses an index over a transient property, but a type
    /// parsed without full validation (a stored contract) still reaches the
    /// lookup's own check, which must never find a document through it.
    #[test]
    fn should_refuse_a_lookup_into_an_index_reading_a_transient_property() {
        let lookup = lookup(&[
            ("submittedCharterId", "submittedCharterId"),
            ("$ownerId", "."),
        ]);
        let referenced = join_request_with(platform_value!({
            "transient": ["submittedCharterId"]
        }));
        let error = lookup
            .referenced_side_error(elected_charter().as_ref(), referenced.as_ref())
            .expect("an index over a transient property should be refused");
        assert!(
            error.contains(
                "findBy names \"submittedCharterId\" of \"joinRequest\", which is transient or \
                 inside a transient object"
            ),
            "{error}"
        );
    }

    /// A key part read from the writer would move with a transfer or a
    /// purchase of the referring document, which re-validates nothing, so
    /// only a type that can do neither may read it.
    #[test]
    fn should_refuse_a_writer_key_part_on_a_referring_type_that_can_change_owner() {
        let lookup = lookup(&[("submittedCharterId", "."), ("$ownerId", "$ownerId")]);
        for extra in [
            platform_value!({ "transferable": 1u8 }),
            platform_value!({ "tradeMode": 1u8 }),
        ] {
            let error = lookup
                .referring_side_error(elected_charter_with(extra.clone()).as_ref(), "memberId")
                .expect("a writer key part should be refused");
            assert!(
                error.contains(
                    "findBy \"$ownerId\" reads \"$ownerId\", which a transfer or a purchase of \
                     the referring document changes"
                ),
                "{extra:?}: {error}"
            );
        }

        // A type that can neither be transferred nor traded keeps its writer
        assert_eq!(
            lookup.referring_side_error(elected_charter().as_ref(), "memberId",),
            None
        );
    }

    #[test]
    fn should_accept_a_key_id_reference_as_a_source_for_a_key_id_index_property() {
        let key_id = |position: u32| {
            platform_value!({
                "type": "integer",
                "minimum": 0u32,
                "maximum": 4294967295u32,
                "position": position
            })
        };
        let referenced = document_type(
            "keyRequest",
            platform_value!({
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": { "keyId": key_id(0) },
                "indices": [{
                    "name": "byKey",
                    "properties": [{ "keyId": "asc" }, { "$ownerId": "asc" }],
                    "unique": true
                }],
                "required": ["keyId"],
                "additionalProperties": false
            }),
        );
        let mut signer_key_id = key_id(0);
        signer_key_id
            .insert(
                "refersTo".to_string(),
                platform_value!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
            )
            .expect("refersTo inserts");
        let declaring = document_type(
            "keyUse",
            platform_value!({
                "type": "object",
                "properties": {
                    "signerKeyId": signer_key_id,
                    "requesterId": with_position(identifier(), 1)
                },
                "required": ["signerKeyId"],
                "additionalProperties": false
            }),
        );
        assert!(matches!(
            declaring.as_ref().flattened_properties()["signerKeyId"].property_type,
            DocumentPropertyType::KeyIdWithReference(_)
        ));

        // A key id reference holds a u32 like the plain key id it is matched with
        let lookup = lookup(&[("keyId", "signerKeyId"), ("$ownerId", ".")]);
        assert_eq!(
            lookup.referenced_side_error(declaring.as_ref(), referenced.as_ref()),
            None
        );
    }

    #[test]
    fn should_refuse_consuming_a_type_that_keeps_history() {
        // Parsed without full validation, as a type stored before protocol version 14
        // may be: a full parse refuses a deletable type that keeps history first
        let keeps_history = join_request_with(platform_value!({
            "canBeDeleted": true,
            "documentsKeepHistory": true
        }));
        let consuming = DocumentReferenceLookup {
            consume: true,
            ..lookup(&[
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ])
        };
        let error = consuming
            .referenced_side_error(elected_charter().as_ref(), keeps_history.as_ref())
            .expect("consuming a type that keeps history should be refused");
        assert!(error.contains("its documents keep history"), "{error}");

        // The same type without the history may be consumed
        let deletable = join_request_with(platform_value!({ "canBeDeleted": true }));
        assert_eq!(
            consuming.referenced_side_error(elected_charter().as_ref(), deletable.as_ref()),
            None
        );
    }

    #[test]
    fn should_refuse_other_system_sources() {
        for source in ["$id", "$createdAt", "$creatorId"] {
            let error =
                LookupKeySource::from_wire_name(source).expect_err("the source should be refused");
            assert!(
                error.to_string().contains("not system property"),
                "{source}: {error}"
            );
        }
    }

    #[test]
    fn should_assemble_the_key_from_the_reference_value_the_owner_and_the_document() {
        let lookup = lookup(&[
            ("submittedCharterId", "submittedCharterId"),
            ("$ownerId", "."),
        ]);
        let member = Identifier::from([3; 32]);
        let owner = Identifier::from([4; 32]);
        let data = BTreeMap::from([("submittedCharterId".to_string(), Value::Identifier([5; 32]))]);

        assert_eq!(
            lookup.key_values(member, &data, owner, None),
            Some(BTreeMap::from([
                ("$ownerId".to_string(), Value::Identifier([3; 32])),
                ("submittedCharterId".to_string(), Value::Identifier([5; 32])),
            ]))
        );
        assert_eq!(
            lookup.key_values(member, &BTreeMap::new(), owner, None),
            None
        );
        assert_eq!(
            lookup.referring_properties().collect::<Vec<_>>(),
            vec!["submittedCharterId"]
        );
    }
}
