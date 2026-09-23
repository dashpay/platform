//! The `lookup` of a document reference: the referenced document is the one a
//! unique index of the referenced document type finds for a key assembled from
//! the referring document, rather than the document whose id the property holds.
//!
//! Declared inside a `permanentDocument` `refersTo` (meta-schema v3, protocol
//! version 14); a `deletableDocument` reference cannot carry one, since a key
//! into a deletable type could find a new document once the one it found is
//! deleted, where an id is produced at most once:
//!
//! ```json
//! "refersTo": {
//!   "type": "permanentDocument",
//!   "documentType": "joinRequest",
//!   "lookup": {
//!     "index": "bySubmittedCharter",
//!     "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
//!   }
//! }
//! ```
//!
//! reads: the value must be the owner of a `joinRequest` whose
//! `submittedCharterId` equals this document's `submittedCharterId`. The rules
//! live here so the two places that check a declaration against its referenced
//! document type (the contract parse for a type of the same contract, the
//! registration state validation for a type of another contract) cannot drift.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property::{is_transient, DocumentPropertyType};
use crate::data_contract::document_type::DocumentTypeRef;
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
use serde::Serialize;
use std::collections::BTreeMap;
use std::mem::Discriminant;

/// The wire name of [`LookupKeySource::ReferenceValue`].
pub const LOOKUP_REFERENCE_VALUE: &str = ".";

/// Where the value of one key part of a [`DocumentReferenceLookup`] comes from,
/// on the referring side.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
#[serde(into = "String")]
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
                    "refersTo lookup keys take \".\", \"{OWNER_ID}\" or a property of the \
                     referring document type, not system property \"{name}\""
                )))
            }
            _ if name.is_empty() || name.len() > MAX_LOOKUP_PATH_LENGTH => {
                Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo lookup key sources must be between 1 and {MAX_LOOKUP_PATH_LENGTH} \
                     characters"
                )))
            }
            _ => Ok(LookupKeySource::Property(name.to_string())),
        }
    }

    /// The wire name, as the schema spells it.
    pub fn as_str(&self) -> &str {
        match self {
            LookupKeySource::ReferenceValue => LOOKUP_REFERENCE_VALUE,
            LookupKeySource::OwnerId => OWNER_ID,
            LookupKeySource::Property(path) => path.as_str(),
        }
    }
}

impl From<LookupKeySource> for String {
    fn from(source: LookupKeySource) -> Self {
        source.as_str().to_string()
    }
}

/// The longest index property name or source path a lookup may name, the bound
/// meta-schema v3 puts on index property names.
pub const MAX_LOOKUP_PATH_LENGTH: usize = 256;

/// The most key parts a lookup may map, the most properties an index can have.
pub const MAX_LOOKUP_KEYS: usize = 10;

/// The longest index name a lookup may name, the bound meta-schema v3 puts on
/// index names.
pub const MAX_LOOKUP_INDEX_NAME_LENGTH: usize = 32;

/// How a document reference finds the referenced document when its value is not
/// that document's id: through the unique index `index` of the referenced
/// document type, with one key part per index property.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct DocumentReferenceLookup {
    /// The name of a unique index of the referenced document type.
    pub index: String,
    /// Every property of the index, by its name on the referenced side
    /// (`$ownerId` among the system ones), mapped to the referring-side source
    /// of its value. [`LookupKeySource::ReferenceValue`] appears exactly once.
    pub keys: BTreeMap<String, LookupKeySource>,
}

impl DocumentReferenceLookup {
    /// The referring document type's property paths the key reads, in key
    /// order: a replace that changes one of them re-validates the reference.
    pub fn referring_properties(&self) -> impl Iterator<Item = &str> {
        self.keys.values().filter_map(|source| match source {
            LookupKeySource::Property(path) => Some(path.as_str()),
            LookupKeySource::ReferenceValue | LookupKeySource::OwnerId => None,
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
    pub fn referring_side_error(
        &self,
        declaring: DocumentTypeRef,
        reference_path: &str,
    ) -> Option<String> {
        for (index_property, source) in &self.keys {
            let path = match source {
                LookupKeySource::ReferenceValue => continue,
                LookupKeySource::OwnerId => {
                    if owner_can_change(declaring) {
                        return Some(format!(
                            "key \"{index_property}\" reads \"$ownerId\", which a transfer or a \
                             purchase of the referring document changes without re-validating \
                             the reference: a lookup may read the writer only on a document type \
                             that cannot be transferred or traded"
                        ));
                    }
                    continue;
                }
                LookupKeySource::Property(path) => path,
            };
            if path == reference_path {
                return Some(format!(
                    "key \"{index_property}\" names the reference property itself: write \".\" \
                     for the reference's own value"
                ));
            }
            let Some(property) = declaring.flattened_properties().get(path) else {
                return Some(format!(
                    "key \"{index_property}\" reads \"{path}\", which is not a property of the \
                     referring document type"
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
                    "key \"{index_property}\" reads \"{path}\", which is not a single value"
                ));
            }
            if is_transient(declaring, path) {
                return Some(format!(
                    "key \"{index_property}\" reads \"{path}\", which is transient or inside a \
                     transient object: the key must be readable from the stored document"
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
                        "key \"{index_property}\" reads \"{path}\", which is not required: a \
                         lookup never runs with a missing key part, so every property it reads \
                         (and every object around it) must be listed in `required`"
                    ));
                }
            }
        }
        None
    }

    /// Why this lookup, declared on a property of `declaring`, cannot resolve
    /// in `referenced`, the referenced document type; `None` when it can. The
    /// index must exist and be unique, so the key finds at most one document;
    /// it may not bucket a timestamp (`timeRange`), since its first key part
    /// is then a bucket start no referring value names; the referenced type
    /// may not be `indexOnly`; no index property may be transient, a value
    /// no stored document holds; `keys` must map every property of the index
    /// exactly once and nothing else; and each source must hold the same kind
    /// of value as the index property it fills, or no document could ever
    /// match. The key must also stay with the document it found, see
    /// [`Self::moving_key_part`].
    pub fn referenced_side_error(
        &self,
        declaring: DocumentTypeRef,
        referenced: DocumentTypeRef,
    ) -> Option<String> {
        let Some(index) = referenced.indexes().get(&self.index) else {
            return Some(format!(
                "the referenced document type \"{}\" has no index named \"{}\"",
                referenced.name(),
                self.index
            ));
        };
        if !index.unique {
            return Some(format!(
                "index \"{}\" of \"{}\" is not unique: a lookup must find at most one document",
                self.index,
                referenced.name()
            ));
        }
        if index.time_range.is_some() {
            return Some(format!(
                "index \"{}\" of \"{}\" buckets its first property by a timeRange, so no \
                 referring value could name a key of it",
                self.index,
                referenced.name()
            ));
        }
        if referenced.index_only() {
            return Some(format!(
                "\"{}\" is an indexOnly document type, which a lookup cannot reference",
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
                "index \"{}\" of \"{}\" keys documents by \"{}\", which is transient or inside \
                 a transient object: its value is never stored, so the lookup could never find \
                 a document",
                self.index,
                referenced.name(),
                transient.name
            ));
        }
        if let Some(missing) = index
            .properties
            .iter()
            .find(|property| !self.keys.contains_key(&property.name))
        {
            return Some(format!(
                "keys does not map \"{}\", a property of index \"{}\": every index property \
                 needs a source",
                missing.name, self.index
            ));
        }
        if let Some(extra) = self.keys.keys().find(|name| {
            !index
                .properties
                .iter()
                .any(|property| &property.name == *name)
        }) {
            return Some(format!(
                "keys maps \"{extra}\", which is not a property of index \"{}\"",
                self.index
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
                LookupKeySource::Property(path) => {
                    let Some(property) = declaring.flattened_properties().get(path) else {
                        return Some(format!(
                            "key \"{index_property}\" reads \"{path}\", which is not a property \
                             of the referring document type"
                        ));
                    };
                    property.property_type.value_kind()
                }
            };
            if source_kind != indexed_kind {
                return Some(format!(
                    "key \"{index_property}\" is filled from \"{}\", which holds a different \
                     kind of value: no document could ever match",
                    source.as_str()
                ));
            }
        }
        if let Some((index_property, why)) = self.moving_key_part(referenced) {
            return Some(format!(
                "index \"{}\" of \"{}\" keys documents by \"{index_property}\", which {why}: a \
                 lookup must keep finding the document it found, so every part of its key must \
                 be fixed once the document is written (make the type immutable or list the \
                 property under `immutable`)",
                self.index,
                referenced.name()
            ));
        }
        None
    }

    /// The first key part a referenced document of `referenced` can change
    /// after it was written, with how, `None` when every part is fixed for
    /// good. A `permanentDocument` reference, the only kind that takes a
    /// lookup, promises it never dangles: its referenced type forbids
    /// deletion, and a lookup's key must not move off the document either, or
    /// a document validated against it would later find nothing. A schema property is fixed on a type whose documents are
    /// immutable or when its top-level property is listed under `immutable`
    /// (an `immutableAllowSetting` entry can only be set on a document that
    /// has no value for it, which no key could have found); `$ownerId` is
    /// fixed unless documents can be transferred or traded; `$id`,
    /// `$creatorId` and the creation times never change; the update and
    /// transfer times change with the document. Every flag read here is
    /// immutable on contract update, and the `immutable` list may only grow,
    /// so the answer holds for good.
    fn moving_key_part(&self, referenced: DocumentTypeRef) -> Option<(&str, &'static str)> {
        let changes_owner = owner_can_change(referenced);
        let replaceable = referenced.documents_mutable();
        self.keys.keys().find_map(|index_property| {
            let why = match index_property.as_str() {
                ID
                | CREATOR_ID
                | CREATED_AT
                | CREATED_AT_BLOCK_HEIGHT
                | CREATED_AT_CORE_BLOCK_HEIGHT => None,
                OWNER_ID => changes_owner.then_some("a transfer or a purchase changes"),
                UPDATED_AT | UPDATED_AT_BLOCK_HEIGHT | UPDATED_AT_CORE_BLOCK_HEIGHT => (replaceable
                    || changes_owner)
                    .then_some("a replace, a transfer or a purchase changes"),
                TRANSFERRED_AT | TRANSFERRED_AT_BLOCK_HEIGHT | TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
                    changes_owner.then_some("a transfer or a purchase changes")
                }
                property => (!schema_property_is_fixed_once_written(referenced, property))
                    .then_some("a replace can change"),
            };
            why.map(|why| (index_property.as_str(), why))
        })
    }

    /// The equality values of the key the lookup assembles for one write, by
    /// index property name: `reference_value` for `"."` (the property's value,
    /// or one array element's), `owner_id` for `"$ownerId"`, and the value at
    /// each source path in `document_data`. `None` when a source path holds no
    /// value or does not resolve. Registration admits required sources only,
    /// so a value is missing only on a document stamped before the property
    /// became required (`requiredSince`) or one the schema validation refuses
    /// anyway, and a key with a missing part finds no document.
    pub fn key_values(
        &self,
        reference_value: Identifier,
        document_data: &BTreeMap<String, Value>,
        owner_id: Identifier,
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
                };
                Some((index_property.clone(), value))
            })
            .collect()
    }
}

/// Whether a document of `document_type` can change owner after it was
/// written, by a transfer or a purchase. Both flags are immutable on contract
/// update, so the answer holds for good.
pub(crate) fn owner_can_change(document_type: DocumentTypeRef) -> bool {
    document_type.documents_transferable().is_transferable()
        || document_type.trade_mode() != TradeMode::None
}

/// Whether the schema property at `path` of a document of `document_type` can
/// never change once the document is written: the type is immutable
/// (`documentsMutable: false`), or the property's top-level property is listed
/// under `immutable`. An `immutableAllowSetting` entry can only be set on a
/// document that has no value for it yet, so a value read once stays. Both
/// flags are immutable on contract update and the `immutable` list may only
/// grow, so the answer holds for good. The one rule both a lookup's key parts
/// and a list element's list are judged by.
pub(crate) fn schema_property_is_fixed_once_written(
    document_type: DocumentTypeRef,
    path: &str,
) -> bool {
    let top_level = path.split('.').next().unwrap_or(path);
    !document_type.documents_mutable() || document_type.immutable_fields().contains(top_level)
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

    fn lookup(index: &str, keys: &[(&str, &str)]) -> DocumentReferenceLookup {
        DocumentReferenceLookup {
            index: index.to_string(),
            keys: keys
                .iter()
                .map(|(index_property, source)| {
                    (
                        index_property.to_string(),
                        LookupKeySource::from_wire_name(source).expect("a valid source"),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn should_accept_a_lookup_covering_a_unique_index_with_sources_of_the_right_kind() {
        let lookup = lookup(
            "bySubmittedCharter",
            &[
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ],
        );
        let declaring = elected_charter();
        assert_eq!(
            lookup.referring_side_error(declaring.as_ref(), "memberId"),
            None
        );
        assert_eq!(
            lookup.referenced_side_error(declaring.as_ref(), join_request().as_ref()),
            None
        );
    }

    #[test]
    fn should_refuse_a_missing_or_non_unique_index() {
        let declaring = elected_charter();
        for (index, fragment) in [
            ("byMissing", "has no index named \"byMissing\""),
            ("byNote", "is not unique"),
        ] {
            let lookup = lookup(index, &[("note", "."), ("x", "note")]);
            let error = lookup
                .referenced_side_error(declaring.as_ref(), join_request().as_ref())
                .expect("the index should be refused");
            assert!(error.contains(fragment), "{index}: {error}");
        }
    }

    #[test]
    fn should_refuse_keys_that_miss_or_add_an_index_property() {
        let declaring = elected_charter();
        for (keys, fragment) in [
            (
                vec![("$ownerId", ".")],
                "does not map \"submittedCharterId\"",
            ),
            (
                vec![
                    ("submittedCharterId", "submittedCharterId"),
                    ("$ownerId", "."),
                    ("note", "note"),
                ],
                "maps \"note\", which is not a property of index",
            ),
        ] {
            let error = lookup("bySubmittedCharter", &keys)
                .referenced_side_error(declaring.as_ref(), join_request().as_ref())
                .expect("the keys should be refused");
            assert!(error.contains(fragment), "{keys:?}: {error}");
        }
    }

    #[test]
    fn should_refuse_a_source_of_the_wrong_value_kind() {
        let declaring = elected_charter();
        let error = lookup(
            "bySubmittedCharter",
            &[("submittedCharterId", "note"), ("$ownerId", ".")],
        )
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
            let error = lookup(
                "bySubmittedCharter",
                &[("submittedCharterId", source), ("$ownerId", ".")],
            )
            .referring_side_error(declaring.as_ref(), "memberId")
            .expect("the source should be refused");
            assert!(error.contains(fragment), "{source}: {error}");
        }
    }

    #[test]
    fn should_refuse_a_lookup_whose_key_the_referenced_document_can_move() {
        let lookup = lookup(
            "bySubmittedCharter",
            &[
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ],
        );
        let declaring = elected_charter();
        let refused = |referenced: DocumentType| {
            lookup.referenced_side_error(declaring.as_ref(), referenced.as_ref())
        };

        // A replace could move `submittedCharterId`, a transfer or a purchase `$ownerId`
        for (extra, moving) in [
            (
                platform_value!({ "documentsMutable": true }),
                "keys documents by \"submittedCharterId\", which a replace can change",
            ),
            (
                platform_value!({ "transferable": 1u8 }),
                "keys documents by \"$ownerId\", which a transfer or a purchase changes",
            ),
            (
                platform_value!({ "tradeMode": 1u8 }),
                "keys documents by \"$ownerId\", which a transfer or a purchase changes",
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
        let lookup = lookup(
            "bySubmittedCharter",
            &[
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ],
        );
        let referenced = join_request_with(platform_value!({
            "transient": ["submittedCharterId"]
        }));
        let error = lookup
            .referenced_side_error(elected_charter().as_ref(), referenced.as_ref())
            .expect("an index over a transient property should be refused");
        assert!(
            error.contains(
                "index \"bySubmittedCharter\" of \"joinRequest\" keys documents by \
                 \"submittedCharterId\", which is transient or inside a transient object"
            ),
            "{error}"
        );
    }

    /// A key part read from the writer would move with a transfer or a
    /// purchase of the referring document, which re-validates nothing, so
    /// only a type that can do neither may read it.
    #[test]
    fn should_refuse_a_writer_key_part_on_a_referring_type_that_can_change_owner() {
        let lookup = lookup(
            "bySubmittedCharter",
            &[("submittedCharterId", "."), ("$ownerId", "$ownerId")],
        );
        for extra in [
            platform_value!({ "transferable": 1u8 }),
            platform_value!({ "tradeMode": 1u8 }),
        ] {
            let error = lookup
                .referring_side_error(elected_charter_with(extra.clone()).as_ref(), "memberId")
                .expect("a writer key part should be refused");
            assert!(
                error.contains(
                    "key \"$ownerId\" reads \"$ownerId\", which a transfer or a purchase of \
                     the referring document changes"
                ),
                "{extra:?}: {error}"
            );
        }

        // A type that can neither be transferred nor traded keeps its writer
        assert_eq!(
            lookup.referring_side_error(elected_charter().as_ref(), "memberId"),
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
        let lookup = lookup("byKey", &[("keyId", "signerKeyId"), ("$ownerId", ".")]);
        assert_eq!(
            lookup.referenced_side_error(declaring.as_ref(), referenced.as_ref()),
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
        let lookup = lookup(
            "bySubmittedCharter",
            &[
                ("submittedCharterId", "submittedCharterId"),
                ("$ownerId", "."),
            ],
        );
        let member = Identifier::from([3; 32]);
        let owner = Identifier::from([4; 32]);
        let data = BTreeMap::from([("submittedCharterId".to_string(), Value::Identifier([5; 32]))]);

        assert_eq!(
            lookup.key_values(member, &data, owner),
            Some(BTreeMap::from([
                ("$ownerId".to_string(), Value::Identifier([3; 32])),
                ("submittedCharterId".to_string(), Value::Identifier([5; 32])),
            ]))
        );
        assert_eq!(lookup.key_values(member, &BTreeMap::new(), owner), None);
        assert_eq!(
            lookup.referring_properties().collect::<Vec<_>>(),
            vec!["submittedCharterId"]
        );
    }
}
