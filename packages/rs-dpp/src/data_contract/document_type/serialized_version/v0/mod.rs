//! Generation 0 of a document type's stored form: every field of [`DocumentTypeV2`] that
//! parsing decides, in a form of its own, so that a change to the in-memory types never changes
//! stored bytes. Left out: the name and the contract id, which the contract holds; the schema,
//! which the contract stores beside the compiled types; the index structure, which this
//! generation's loader rebuilds from the indexes with generation 0 of its builder; and the
//! validators, caches built on first use. The typed validation schema is stored, with the
//! generation of the builder that built it, so the type's documents keep being checked against
//! it.
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
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use property::{properties_from_stored, stored_properties, StoredGeneratedFrom, StoredProperties};
use rules::StoredRule;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use validation_schema::StoredValidationSchema;

#[cfg(feature = "validation")]
use crate::data_contract::document_type::schema::validation_schema::{
    validated_schema, ValidationSchema,
};
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
    distinct_from_fields: Vec<String>,
    generated_from_fields: Vec<(String, StoredGeneratedFrom)>,
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
    validation_schema: Option<StoredValidationSchema>,
}

impl DocumentTypeInSerializationFormatV0 {
    /// `document_type`, of a contract whose `$defs` are `schema_defs`, compiled. A type parsed
    /// by an earlier generation into an earlier structure is compiled as the latest structure
    /// holds it.
    pub(super) fn try_from_document_type(
        document_type: DocumentTypeRef,
        schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let document_type: Cow<DocumentTypeV2> = match document_type {
            DocumentTypeRef::V0(v0) => Cow::Owned(v0.clone().into()),
            DocumentTypeRef::V1(v1) => Cow::Owned(v1.clone().into()),
            DocumentTypeRef::V2(v2) => Cow::Borrowed(v2),
        };
        let validation_schema =
            stored_validation_schema(&document_type, schema_defs, platform_version)?;
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
            distinct_from_fields: distinct_from_fields.clone(),
            generated_from_fields: generated_from_fields
                .iter()
                .map(|(path, generated_from)| (path.clone(), generated_from.into()))
                .collect(),
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
            validation_schema,
        })
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
            validation_schema,
        } = self;

        let properties = properties_from_stored(properties)?;
        let flattened_properties = properties_from_stored(flattened_properties)?;
        let indices: BTreeMap<String, Index> = indices
            .into_iter()
            .map(|(name, index)| Ok((name, Index::try_from(index)?)))
            .collect::<Result<_, ProtocolError>>()?;
        let mut index_structure = IndexLevel::try_from_indices_v0(indices.values(), name)?;
        // The levels named after a typed array fan the document out over its elements, as the
        // parse stamped them when the type was stored
        if !indexed_arrays.is_empty() {
            stamp_stored_indexed_arrays(&mut index_structure, &indexed_arrays);
        }

        // A stored form without a validation schema keeps its documents refused, as the
        // version that stored it refuses them, rather than leaving them to the builder of
        // whatever version reads it
        #[cfg(feature = "validation")]
        let json_schema_validator = StatelessJsonSchemaLazyValidator::with_loaded_validation_schema(
            match validation_schema {
                Some(stored) => {
                    let (schema, generation) = stored.into_validation_schema()?;
                    LoadedValidationSchema::Built { generation, schema }
                }
                None => {
                    LoadedValidationSchema::Refused(DataContractError::InvalidContractStructure(
                        "its stored form holds none, as the version that stored it built \
                             none from its schema"
                            .to_string(),
                    ))
                }
            },
        );
        // Without `validation` a document type has no validator to hold it: read and dropped,
        // and such a build does not store a document type compiled again
        #[cfg(not(feature = "validation"))]
        let _ = validation_schema;

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
            generated_from_fields: generated_from_fields
                .into_iter()
                .map(|(path, generated_from)| (path, generated_from.into()))
                .collect(),
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

/// The typed validation schema to store with `document_type`: what it was read back with from
/// a stored form, or the one the builder `platform_version` selects builds from its schema.
/// `None` when it was read back without one, when that version selects no builder, or when the
/// builder refuses the schema; its documents are then refused, at every version that reads it.
#[cfg(feature = "validation")]
fn stored_validation_schema(
    document_type: &DocumentTypeV2,
    schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
    platform_version: &PlatformVersion,
) -> Result<Option<StoredValidationSchema>, ProtocolError> {
    if let Some(loaded) = document_type
        .json_schema_validator
        .loaded_validation_schema()
    {
        return Ok(match loaded {
            LoadedValidationSchema::Built { generation, schema } => {
                Some(StoredValidationSchema::new(schema, *generation))
            }
            LoadedValidationSchema::Refused(_) => None,
        });
    }
    let Some(generation) = platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validation_schema
    else {
        return Ok(None);
    };
    let root = validated_schema(&document_type.schema, schema_defs, platform_version)?;
    match ValidationSchema::build(&root, platform_version) {
        Ok((schema, _)) => Ok(Some(StoredValidationSchema::new(&schema, generation))),
        Err(ProtocolError::DataContractError(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Without `validation` a document type holds no validation schema read back from a stored
/// form (it has no validator to hold one), so the stored form written here could differ from
/// the one a node writes for the same contract: storing a document type compiled needs it.
#[cfg(not(feature = "validation"))]
fn stored_validation_schema(
    _document_type: &DocumentTypeV2,
    _schema_defs: Option<&BTreeMap<DefinitionName, Value>>,
    _platform_version: &PlatformVersion,
) -> Result<Option<StoredValidationSchema>, ProtocolError> {
    Err(ProtocolError::NotSupported(
        "storing a document type compiled needs the validation feature, whose document types \
         keep the validation schema they were read back with"
            .to_string(),
    ))
}

#[cfg(test)]
mod tests;
