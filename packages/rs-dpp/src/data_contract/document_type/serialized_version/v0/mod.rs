//! Generation 0 of a document type's stored form: every field of [`DocumentTypeV2`] that
//! parsing decides, in a form of its own, so that a change to the in-memory types never changes
//! stored bytes. Left out: the name and the contract id, which the contract holds; the schema,
//! which the contract stores beside the compiled types; what this generation's loader derives,
//! by rules of its own, from what is stored: the index structure, rebuilt from the indexes with
//! generation 0 of its builder (`index_levels_from_indices` 0, the only one this generation is
//! written under) and stamped with the typed arrays stored resolved, and the paths declaring
//! `distinctFrom` and `generatedFrom`, read off the flattened properties; and the validators,
//! caches built on first use. What the type's documents are checked against is stored
//! ([`StoredValidation`]): the typed validation schema, as the bytes of its encoding, with the
//! generation of the builder that built it, or that builder's refusal.
//!
//! This module is frozen once released: a change to what is stored, or to how it is read back,
//! is a new generation.

mod costs;
mod index;
mod nested;
mod property;
mod rules;
mod validation_schema;

use crate::data_contract::config::moderation::SettledDeletionRule;
use crate::data_contract::document_type::property::{DocumentProperty, GeneratedFrom};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentType, DocumentTypeRef, Index, IndexLevel,
};
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::DefinitionName;
use crate::identity::SecurityLevel;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use costs::{StoredActionFees, StoredTokenCosts, StoredTradeMode, StoredTransferable};
use index::{StoredDerivedIndexProperty, StoredIndex};
use indexmap::IndexMap;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use property::{properties_from_stored, stored_properties, StoredProperties};
use rules::StoredRule;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use validation_schema::StoredValidation;

#[cfg(feature = "validation")]
pub(crate) use validation_schema::EncodedValidationSchema;

#[cfg(feature = "validation")]
use crate::data_contract::document_type::schema::validation_schema::native_validator_of;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::{
    LoadedValidationSchema, StatelessJsonSchemaLazyValidator,
};
#[cfg(feature = "validation")]
use crate::data_contract::errors::DataContractError;

/// The error of a stored document type that does not read back.
fn stored_error(message: String) -> ProtocolError {
    ProtocolError::CorruptedSerialization(format!("stored document type: {message}"))
}

/// A document type compiled, generation 0.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub struct DocumentTypeInSerializationFormatV0 {
    properties: StoredProperties,
    flattened_properties: StoredProperties,
    indices: BTreeMap<String, StoredIndex>,
    /// The typed arrays the index structure's levels fan documents out over, with their
    /// `maxItems`, as the parse stamped them: stored resolved, so reading back stamps the same
    /// levels whatever a later parser resolves.
    indexed_arrays: BTreeMap<String, u16>,
    identifier_paths: BTreeSet<String>,
    binary_paths: BTreeSet<String>,
    required_fields: BTreeSet<String>,
    transient_fields: BTreeSet<String>,
    immutable_fields: BTreeSet<String>,
    immutable_field_conditions: BTreeMap<String, StoredRule>,
    retracted_when: Option<StoredRule>,
    entry_payload: BTreeSet<String>,
    documents_keep_history: bool,
    documents_keep_transfer_history: bool,
    documents_keep_purchase_history: bool,
    documents_keep_pricing_history: bool,
    documents_mutable: bool,
    documents_can_be_deleted: bool,
    documents_deleted_only_when_consumed: bool,
    documents_transferable: StoredTransferable,
    trade_mode: StoredTradeMode,
    creation_restriction_mode: CreationRestrictionMode,
    requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
    security_level_requirement: SecurityLevel,
    token_costs: StoredTokenCosts,
    documents_countable: bool,
    range_countable: bool,
    documents_summable: Option<String>,
    range_summable: bool,
    index_only: bool,
    action_fees: Option<StoredActionFees>,
    documents_can_be_deleted_by_moderators: bool,
    documents_can_be_deleted_by_moderators_for: Option<u32>,
    moderator_deletions_keep_records: bool,
    moderator_deletions_refund_owner: bool,
    moderator_settled_deletion: Option<SettledDeletionRule>,
    moderator_deletion_kept_fields: BTreeSet<String>,
    moderator_changeable_fields: BTreeSet<String>,
    owner_reference: Option<DocumentPropertyReferenceTarget>,
    creator_reference: Option<DocumentPropertyReferenceTarget>,
    property_constraints: BTreeMap<String, StoredRule>,
    delete_constraints: BTreeMap<String, StoredRule>,
    documents_ttl_seconds: Option<u32>,
    derived_index_properties: BTreeMap<String, StoredDerivedIndexProperty>,
    validation: StoredValidation,
}

impl DocumentTypeInSerializationFormatV0 {
    /// `document_type`, of a contract whose `$defs` are `schema_defs`, compiled. A type parsed
    /// by an earlier generation into an earlier structure is compiled as the latest structure
    /// holds it. Refuses a protocol version whose index structure builder
    /// (`index_levels_from_indices`) is not generation 0, the one this generation's loader
    /// rebuilds index structures with.
    pub(super) fn try_from_document_type(
        document_type: DocumentTypeRef,
        schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let index_builder = platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .index_versions
            .index_levels_from_indices;
        if index_builder != 0 {
            return Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTypeInSerializationFormatV0::try_from_document_type \
                         (index_levels_from_indices)"
                    .to_string(),
                known_versions: vec![0],
                received: index_builder,
            });
        }
        let document_type: Cow<DocumentTypeV2> = match document_type {
            DocumentTypeRef::V0(v0) => Cow::Owned(v0.clone().into()),
            DocumentTypeRef::V1(v1) => Cow::Owned(v1.clone().into()),
            DocumentTypeRef::V2(v2) => Cow::Borrowed(v2),
        };
        let validation = stored_validation(&document_type, schema_defs, platform_version)?;
        let rules = |rules: &BTreeMap<String, _>| {
            rules
                .iter()
                .map(|(name, rule)| (name.clone(), StoredRule::from(rule)))
                .collect()
        };
        let DocumentTypeV2 {
            name: _,
            schema: _,
            indices,
            index_structure,
            flattened_properties,
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            transient_fields,
            immutable_fields,
            immutable_field_conditions,
            retracted_when,
            distinct_from_fields: _,
            generated_from_fields: _,
            entry_payload,
            documents_keep_history,
            documents_keep_transfer_history,
            documents_keep_purchase_history,
            documents_keep_pricing_history,
            documents_mutable,
            documents_can_be_deleted,
            documents_deleted_only_when_consumed,
            documents_transferable,
            trade_mode,
            creation_restriction_mode,
            data_contract_id: _,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
            security_level_requirement,
            #[cfg(feature = "validation")]
                json_schema_validator: _,
            token_costs,
            documents_countable,
            range_countable,
            documents_summable,
            range_summable,
            index_only,
            action_fees,
            documents_can_be_deleted_by_moderators,
            documents_can_be_deleted_by_moderators_for,
            moderator_deletions_keep_records,
            moderator_deletions_refund_owner,
            moderator_settled_deletion,
            moderator_deletion_kept_fields,
            moderator_changeable_fields,
            owner_reference,
            creator_reference,
            property_constraints,
            delete_constraints,
            documents_ttl_seconds,
            derived_index_properties,
        } = document_type.as_ref();
        Ok(DocumentTypeInSerializationFormatV0 {
            properties: stored_properties(properties),
            flattened_properties: stored_properties(flattened_properties),
            indices: indices
                .iter()
                .map(|(name, index)| (name.clone(), StoredIndex::from(index)))
                .collect(),
            indexed_arrays: stamped_indexed_arrays(index_structure),
            identifier_paths: identifier_paths.clone(),
            binary_paths: binary_paths.clone(),
            required_fields: required_fields.clone(),
            transient_fields: transient_fields.clone(),
            immutable_fields: immutable_fields.clone(),
            immutable_field_conditions: rules(immutable_field_conditions),
            retracted_when: retracted_when.as_ref().map(StoredRule::from),
            entry_payload: entry_payload.clone(),
            documents_keep_history: *documents_keep_history,
            documents_keep_transfer_history: *documents_keep_transfer_history,
            documents_keep_purchase_history: *documents_keep_purchase_history,
            documents_keep_pricing_history: *documents_keep_pricing_history,
            documents_mutable: *documents_mutable,
            documents_can_be_deleted: *documents_can_be_deleted,
            documents_deleted_only_when_consumed: *documents_deleted_only_when_consumed,
            documents_transferable: (*documents_transferable).into(),
            trade_mode: (*trade_mode).into(),
            creation_restriction_mode: *creation_restriction_mode,
            requires_identity_encryption_bounded_key: *requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: *requires_identity_decryption_bounded_key,
            security_level_requirement: *security_level_requirement,
            token_costs: token_costs.into(),
            documents_countable: *documents_countable,
            range_countable: *range_countable,
            documents_summable: documents_summable.clone(),
            range_summable: *range_summable,
            index_only: *index_only,
            action_fees: action_fees.as_ref().map(StoredActionFees::from),
            documents_can_be_deleted_by_moderators: *documents_can_be_deleted_by_moderators,
            documents_can_be_deleted_by_moderators_for: *documents_can_be_deleted_by_moderators_for,
            moderator_deletions_keep_records: *moderator_deletions_keep_records,
            moderator_deletions_refund_owner: *moderator_deletions_refund_owner,
            moderator_settled_deletion: *moderator_settled_deletion,
            moderator_deletion_kept_fields: moderator_deletion_kept_fields.clone(),
            moderator_changeable_fields: moderator_changeable_fields.clone(),
            owner_reference: owner_reference.clone(),
            creator_reference: creator_reference.clone(),
            property_constraints: rules(property_constraints),
            delete_constraints: rules(delete_constraints),
            documents_ttl_seconds: *documents_ttl_seconds,
            derived_index_properties: derived_index_properties
                .iter()
                .map(|(name, derived)| (name.clone(), StoredDerivedIndexProperty::from(derived)))
                .collect(),
            validation,
        })
    }

    /// Whether this stored form is what `document_type`, of a contract whose `$defs` are
    /// `schema_defs`, compiles to at `platform_version`: the same stored form, what its
    /// documents are checked against included. A build without `validation`, which cannot
    /// compile a type, compares the type this form reads back as with `document_type`.
    pub(super) fn is_compiled_from(
        &self,
        document_type: DocumentTypeRef,
        schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        #[cfg(feature = "validation")]
        {
            Ok(
                *self
                    == Self::try_from_document_type(document_type, schema_defs, platform_version)?,
            )
        }
        #[cfg(not(feature = "validation"))]
        {
            let _ = (schema_defs, platform_version);
            let parsed: DocumentTypeV2 = match document_type {
                DocumentTypeRef::V0(v0) => v0.clone().into(),
                DocumentTypeRef::V1(v1) => v1.clone().into(),
                DocumentTypeRef::V2(v2) => v2.clone(),
            };
            let read_back = self.clone().into_document_type(
                &parsed.name,
                parsed.data_contract_id,
                parsed.schema.clone(),
            )?;
            Ok(read_back == DocumentType::V2(parsed))
        }
    }

    /// The document type `name` of the contract `data_contract_id` read back, with `schema`, the
    /// schema it was compiled from. The index structure is rebuilt by generation 0 of its
    /// builder, the one this generation of the stored form was written with.
    pub(super) fn into_document_type(
        self,
        name: &str,
        data_contract_id: Identifier,
        schema: Value,
    ) -> Result<DocumentType, ProtocolError> {
        let rules = |rules: BTreeMap<String, StoredRule>| {
            rules
                .into_iter()
                .map(|(name, rule)| (name, rule.into()))
                .collect()
        };
        let DocumentTypeInSerializationFormatV0 {
            properties,
            flattened_properties,
            indices,
            indexed_arrays,
            identifier_paths,
            binary_paths,
            required_fields,
            transient_fields,
            immutable_fields,
            immutable_field_conditions,
            retracted_when,
            entry_payload,
            documents_keep_history,
            documents_keep_transfer_history,
            documents_keep_purchase_history,
            documents_keep_pricing_history,
            documents_mutable,
            documents_can_be_deleted,
            documents_deleted_only_when_consumed,
            documents_transferable,
            trade_mode,
            creation_restriction_mode,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
            security_level_requirement,
            token_costs,
            documents_countable,
            range_countable,
            documents_summable,
            range_summable,
            index_only,
            action_fees,
            documents_can_be_deleted_by_moderators,
            documents_can_be_deleted_by_moderators_for,
            moderator_deletions_keep_records,
            moderator_deletions_refund_owner,
            moderator_settled_deletion,
            moderator_deletion_kept_fields,
            moderator_changeable_fields,
            owner_reference,
            creator_reference,
            property_constraints,
            delete_constraints,
            documents_ttl_seconds,
            derived_index_properties,
            validation,
        } = self;

        let properties = properties_from_stored(properties)?;
        let flattened_properties = properties_from_stored(flattened_properties)?;
        let indices: BTreeMap<String, Index> = indices
            .into_iter()
            .map(|(name, index)| (name, Index::from(index)))
            .collect();
        let mut index_structure = IndexLevel::try_from_indices_v0(indices.values(), name)?;
        // The levels named after a typed array fan the document out over its elements, as the
        // parse stamped them when the type was stored
        if !indexed_arrays.is_empty() {
            stamp_stored_indexed_arrays(&mut index_structure, &indexed_arrays);
        }

        let distinct_from_fields = stored_distinct_from_fields(&flattened_properties);
        let generated_from_fields = stored_generated_from_fields(&flattened_properties);

        // The documents are checked against the validation schema stored, decoded on first
        // use, by the generation that built it; a stored refusal keeps them refused, as the
        // version that stored the type refuses them, rather than leaving them to the builder of
        // whatever version reads it
        #[cfg(feature = "validation")]
        let json_schema_validator =
            StatelessJsonSchemaLazyValidator::with_loaded_validation_schema(match validation {
                StoredValidation::Built { generation, schema } => LoadedValidationSchema::Built {
                    generation,
                    schema: EncodedValidationSchema::new(schema),
                },
                StoredValidation::Refused { generation } => LoadedValidationSchema::Refused {
                    generation: Some(generation),
                    refusal: DataContractError::InvalidContractStructure(format!(
                        "validation schema builder {generation} refused its schema when the \
                         type was stored"
                    )),
                },
                StoredValidation::NoBuilder => LoadedValidationSchema::Refused {
                    generation: None,
                    refusal: DataContractError::InvalidContractStructure(
                        "the protocol version that stored the type selected no validation \
                         schema builder"
                            .to_string(),
                    ),
                },
            });
        // Without `validation` a document type has no validator to hold it: read past, and such
        // a build does not store a document type compiled again
        #[cfg(not(feature = "validation"))]
        let _ = validation;

        Ok(DocumentType::V2(DocumentTypeV2 {
            name: name.to_string(),
            schema,
            indices,
            index_structure,
            flattened_properties,
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            transient_fields,
            immutable_fields,
            immutable_field_conditions: rules(immutable_field_conditions),
            retracted_when: retracted_when.map(Into::into),
            distinct_from_fields,
            generated_from_fields,
            entry_payload,
            documents_keep_history,
            documents_keep_transfer_history,
            documents_keep_purchase_history,
            documents_keep_pricing_history,
            documents_mutable,
            documents_can_be_deleted,
            documents_deleted_only_when_consumed,
            documents_transferable: documents_transferable.into(),
            trade_mode: trade_mode.into(),
            creation_restriction_mode,
            data_contract_id,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
            security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator,
            token_costs: token_costs.into(),
            documents_countable,
            range_countable,
            documents_summable,
            range_summable,
            index_only,
            action_fees: action_fees.map(Into::into),
            documents_can_be_deleted_by_moderators,
            documents_can_be_deleted_by_moderators_for,
            moderator_deletions_keep_records,
            moderator_deletions_refund_owner,
            moderator_settled_deletion,
            moderator_deletion_kept_fields,
            moderator_changeable_fields,
            owner_reference,
            creator_reference,
            property_constraints: rules(property_constraints),
            delete_constraints: rules(delete_constraints),
            documents_ttl_seconds,
            derived_index_properties: derived_index_properties
                .into_iter()
                .map(|(name, derived)| Ok((name, derived.try_into()?)))
                .collect::<Result<_, ProtocolError>>()?,
        }))
    }
}

/// The typed arrays the levels of `index_structure` fan documents out over, with their
/// `maxItems`: what the parse stamped on them.
fn stamped_indexed_arrays(index_structure: &IndexLevel) -> BTreeMap<String, u16> {
    fn collect(level: &IndexLevel, stamped: &mut BTreeMap<String, u16>) {
        for (key, sub_level) in level.sub_levels() {
            if let Some(max_items) = sub_level.indexed_array_max_items() {
                stamped.insert(key.clone(), max_items);
            }
            collect(sub_level, stamped);
        }
    }
    let mut stamped = BTreeMap::new();
    collect(index_structure, &mut stamped);
    stamped
}

/// Stamps every level of `index_structure` keyed by one of `indexed_arrays` with its stored
/// `maxItems`, at any depth, as generation 0 reads them back. Generation 0's own walk, apart
/// from the parser's (`IndexLevel::stamp_indexed_arrays`), so that a change to how the parser
/// stamps a fresh index structure never changes what generation 0 bytes read back as.
fn stamp_stored_indexed_arrays(level: &mut IndexLevel, indexed_arrays: &BTreeMap<String, u16>) {
    for (key, sub_level) in level.sub_levels_mut().iter_mut() {
        if let Some(max_items) = indexed_arrays.get(key) {
            sub_level.set_indexed_array_max_items(Some(*max_items));
        }
        stamp_stored_indexed_arrays(sub_level, indexed_arrays);
    }
}

/// The paths of the flattened properties that declare `distinctFrom`, in their (schema)
/// order, as generation 0 reads them back.
fn stored_distinct_from_fields(
    flattened_properties: &IndexMap<String, DocumentProperty>,
) -> Vec<String> {
    flattened_properties
        .iter()
        .filter(|(_, property)| property.distinct_from.is_some())
        .map(|(path, _)| path.clone())
        .collect()
}

/// The flattened properties that declare `generatedFrom`, with their declarations, in their
/// (schema) order, as generation 0 reads them back.
fn stored_generated_from_fields(
    flattened_properties: &IndexMap<String, DocumentProperty>,
) -> Vec<(String, GeneratedFrom)> {
    flattened_properties
        .iter()
        .filter_map(|(path, property)| Some((path.clone(), property.generated_from.clone()?)))
        .collect()
}

/// What `document_type`'s documents are checked against, to store: what it was read back with
/// from a stored form, as stored; otherwise what the builder `platform_version` selects makes of
/// its schema, the build registration kept when there is one ([`native_validator_of`]), or none
/// when that version selects no builder.
#[cfg(feature = "validation")]
fn stored_validation(
    document_type: &DocumentTypeV2,
    schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
    platform_version: &PlatformVersion,
) -> Result<StoredValidation, ProtocolError> {
    let validators = &document_type.json_schema_validator;
    match validators.loaded_validation_schema() {
        Some(LoadedValidationSchema::Built { generation, schema }) => {
            return Ok(StoredValidation::Built {
                generation: *generation,
                schema: schema.bytes().to_vec(),
            })
        }
        Some(LoadedValidationSchema::Refused {
            generation: Some(generation),
            ..
        }) => {
            return Ok(StoredValidation::Refused {
                generation: *generation,
            })
        }
        Some(LoadedValidationSchema::Refused {
            generation: None, ..
        }) => return Ok(StoredValidation::NoBuilder),
        None => {}
    }
    let Some(generation) = platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validation_schema
    else {
        return Ok(StoredValidation::NoBuilder);
    };
    Ok(
        match native_validator_of(
            validators,
            &document_type.schema,
            schema_defs,
            platform_version,
        )? {
            Ok(validator) => StoredValidation::Built {
                generation: validator.generation(),
                schema: EncodedValidationSchema::encode(validator.schema())?
                    .bytes()
                    .to_vec(),
            },
            Err(_) => StoredValidation::Refused { generation },
        },
    )
}

/// Without `validation` a document type holds no validation schema read back from a stored
/// form (it has no validator to hold one), so the stored form written here could differ from
/// the one a node writes for the same contract: storing a document type compiled needs it.
#[cfg(not(feature = "validation"))]
fn stored_validation(
    _document_type: &DocumentTypeV2,
    _schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
    _platform_version: &PlatformVersion,
) -> Result<StoredValidation, ProtocolError> {
    Err(ProtocolError::NotSupported(
        "storing a document type compiled needs the validation feature, whose document types \
         keep the validation schema they were read back with"
            .to_string(),
    ))
}

#[cfg(test)]
mod tests;
