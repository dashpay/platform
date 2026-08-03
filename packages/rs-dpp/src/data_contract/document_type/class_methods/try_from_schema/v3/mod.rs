//! Document-type parser **generation 3** — protocol version 14 and later.
//!
//! Generation 3 is generation 2 plus the ranked index keywords
//! (`rankedCountable` / `rankedSummable` / `rankedAverageable`).
//!
//! It is a full copy of the generation-2 parser — the wrapper in `v2/mod.rs`
//! *and* the generation-1 core that wrapper delegates to — rather than a
//! version gate added inside those modules. That follows the repository rule
//! that grammar introduced by a new protocol version gets a new parser
//! generation: shipped generations stay byte-identical to the code consensus
//! already ran, so replaying a historical block can never pick up grammar that
//! did not exist when that block was produced. Nothing here calls into the
//! `v1` or `v2` parser modules.
//!
//! The copy is kept structurally line-for-line with its sources on purpose:
//! diffing this file against `v1/mod.rs` + `v2/mod.rs` should show only the
//! ranked deltas.

#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::{
    DuplicateIndexNameError, InvalidIndexPropertyTypeError, InvalidIndexedPropertyConstraintError,
    SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError,
};
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::property::DocumentPropertyType;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::schema::validate_max_depth;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use indexmap::IndexMap;
use std::collections::BTreeMap;
#[cfg(feature = "validation")]
use std::collections::HashSet;
use std::convert::TryInto;

use crate::balances::credits::TokenAmount;
#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::ContestedUniqueIndexOnMutableDocumentTypeError;
#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::ContestedUniqueIndexWithUniqueIndexError;
#[cfg(any(test, feature = "validation"))]
use crate::consensus::basic::data_contract::InvalidDocumentTypeNameError;
#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::RedundantDocumentPaidForByTokenWithContractId;
#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::TokenPaymentByBurningOnlyAllowedOnInternalTokenError;
#[cfg(feature = "validation")]
use crate::consensus::basic::document::MissingPositionsInDocumentTypePropertiesError;
#[cfg(feature = "validation")]
use crate::consensus::basic::token::InvalidTokenPositionError;
#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::basic::UnsupportedFeatureError;
use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::try_from_schema::{
    insert_values, insert_values_nested,
};
#[cfg(feature = "validation")]
use crate::data_contract::document_type::class_methods::try_from_schema::{
    MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH, MAX_INDEXED_STRING_PROPERTY_LENGTH,
    NOT_ALLOWED_SYSTEM_PROPERTIES,
};
use crate::data_contract::document_type::class_methods::{
    consensus_or_protocol_data_contract_error, consensus_or_protocol_value_error,
};
use crate::data_contract::document_type::property_names::{
    CAN_BE_DELETED, CREATION_RESTRICTION_MODE, DOCUMENTS_AVERAGEABLE, DOCUMENTS_COUNTABLE,
    DOCUMENTS_KEEP_HISTORY, DOCUMENTS_MUTABLE, DOCUMENTS_SUMMABLE, KEEPS_PRICING_HISTORY,
    KEEPS_PURCHASE_HISTORY, KEEPS_TRANSFER_HISTORY, RANGE_AVERAGEABLE, RANGE_COUNTABLE,
    RANGE_SUMMABLE, TRADE_MODE, TRANSFERABLE,
};
use crate::data_contract::document_type::token_costs::v0::TokenCostsV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::identity::SecurityLevel;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_amount_on_contract_token::{
    DocumentActionTokenCost, DocumentActionTokenEffect,
};
#[cfg(feature = "validation")]
use crate::validation::meta_validators::{
    DOCUMENT_META_SCHEMA_V0, DOCUMENT_META_SCHEMA_V1, DOCUMENT_META_SCHEMA_V2,
    DOCUMENT_META_SCHEMA_V3,
};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};

impl DocumentTypeV1 {
    /// The shared parsing core of generation 3 — a copy of the generation-1
    /// core (`v1/mod.rs`), private to this module. It builds the
    /// `DocumentTypeV1` value that [`DocumentTypeV2::try_from_schema_generation_3`]
    /// then layers the generation-2 doctype-level aggregate fields onto.
    ///
    /// Copied rather than called so that `v1/mod.rs` stays byte-identical to
    /// the shipped generation; see the module doc.
    // TODO: Split into multiple functions
    #[allow(unused_variables)]
    #[allow(clippy::too_many_arguments)]
    fn try_from_schema_generation_3_core(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool, // we don't need to validate if loaded from state
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // Create a full root JSON Schema from shorten contract document type schema
        let root_schema = DocumentType::enrich_with_base_schema(
            schema.clone(),
            schema_defs.map(|defs| Value::from(defs.clone())),
            platform_version,
        )?;

        #[cfg(not(feature = "validation"))]
        if full_validation {
            // TODO we are silently dropping this error when we shouldn't be
            // but returning this error causes tests to fail; investigate more.
            "validation is not enabled but is being called on try_from_schema".to_string();
        }

        #[cfg(feature = "validation")]
        let json_schema_validator = StatelessJsonSchemaLazyValidator::new();

        #[cfg(feature = "validation")]
        if full_validation {
            // Make sure a document type name is compliant
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                || name.is_empty()
                || name.len() > 64
            {
                return Err(ProtocolError::ConsensusError(Box::new(
                    InvalidDocumentTypeNameError::new(name.to_string()).into(),
                )));
            }

            // Validate document schema depth
            let mut result = validate_max_depth(&root_schema, platform_version)?;

            if !result.is_valid() {
                let error = result.errors.remove(0);

                let schema_size = result.into_data()?.size;

                validation_operations.extend(std::iter::once(
                    ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(schema_size),
                ));

                return Err(ProtocolError::ConsensusError(Box::new(error)));
            }

            let schema_size = result.into_data()?.size;

            validation_operations.extend(std::iter::once(
                ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(schema_size),
            ));

            // Make sure JSON Schema is compilable
            let root_json_schema = root_schema.try_to_validating_json().map_err(|e| {
                ProtocolError::ConsensusError(
                    ConsensusError::BasicError(BasicError::ValueError(e.into())).into(),
                )
            })?;

            // Select the appropriate document meta-schema based on platform version
            let meta_schema = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .document_type_schema
            {
                0 => &*DOCUMENT_META_SCHEMA_V0,
                1 => &*DOCUMENT_META_SCHEMA_V1,
                2 => &*DOCUMENT_META_SCHEMA_V2,
                3 => &*DOCUMENT_META_SCHEMA_V3,
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentType::try_from_schema_v3 (document_type_schema)"
                            .to_string(),
                        known_versions: vec![0, 1, 2, 3],
                        received: version,
                    })
                }
            };

            // Validate against JSON Schema
            meta_schema
                .validate(&root_json_schema)
                .map_err(|mut errs| ConsensusError::from(errs.next().unwrap()))?;

            json_schema_validator.compile(&root_json_schema, platform_version)?;
        }

        // This has already been validated, but we leave the map_err here for consistency
        let schema_map = schema.to_map().map_err(|err| {
            consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
                format!("document schema must be an object: {err}"),
            ))
        })?;

        // Do documents of this type keep history? (Overrides contract value)
        let documents_keep_history: bool =
            Value::inner_optional_bool_value(schema_map, DOCUMENTS_KEEP_HISTORY)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or(data_contact_config.documents_keep_history_contract_default());

        // The document history subscription flags are only recognized from
        // document meta-schema v2 (protocol version 13). Earlier meta-schema
        // versions either accepted and ignored unknown top-level keys (v0) or
        // rejected them outright (v1), so parsing them here for historical
        // protocol versions would change replay validation: a pre-v12
        // contract carrying e.g. a non-boolean value under one of these names
        // validated fine on the base implementation and must keep doing so.
        let (
            documents_keep_transfer_history,
            documents_keep_purchase_history,
            documents_keep_pricing_history,
        ): (bool, bool, bool) = if platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .document_type_schema
            >= 2
        {
            (
                // Are transfers of documents of this type recorded in the
                // document history system contract?
                Value::inner_optional_bool_value(schema_map, KEEPS_TRANSFER_HISTORY)
                    .map_err(consensus_or_protocol_value_error)?
                    .unwrap_or_default(),
                // Are purchases of documents of this type recorded in the
                // document history system contract?
                Value::inner_optional_bool_value(schema_map, KEEPS_PURCHASE_HISTORY)
                    .map_err(consensus_or_protocol_value_error)?
                    .unwrap_or_default(),
                // Are price updates on documents of this type recorded in the
                // document history system contract?
                Value::inner_optional_bool_value(schema_map, KEEPS_PRICING_HISTORY)
                    .map_err(consensus_or_protocol_value_error)?
                    .unwrap_or_default(),
            )
        } else {
            (false, false, false)
        };

        // Are documents of this type mutable? (Overrides contract value)
        let documents_mutable: bool =
            Value::inner_optional_bool_value(schema_map, DOCUMENTS_MUTABLE)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or(data_contact_config.documents_mutable_contract_default());

        // Can documents of this type be deleted? (Overrides contract value)
        let documents_can_be_deleted: bool =
            Value::inner_optional_bool_value(schema_map, CAN_BE_DELETED)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or(data_contact_config.documents_can_be_deleted_contract_default());

        // Are documents of this type transferable?
        let documents_transferable_u8: u8 =
            Value::inner_optional_integer_value(schema_map, TRANSFERABLE)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or_default();

        let documents_transferable = documents_transferable_u8.try_into()?;

        // What is the trade mode of these documents
        let documents_trade_mode_u8: u8 =
            Value::inner_optional_integer_value(schema_map, TRADE_MODE)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or_default();

        let trade_mode = documents_trade_mode_u8.try_into()?;

        // What is the creation restriction mode of this document type?
        let documents_creation_restriction_mode_u8: u8 =
            Value::inner_optional_integer_value(schema_map, CREATION_RESTRICTION_MODE)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or_default();

        let creation_restriction_mode = documents_creation_restriction_mode_u8.try_into()?;

        // Extract the properties
        let property_values = Value::inner_optional_index_map::<u64>(
            schema_map,
            property_names::PROPERTIES,
            property_names::POSITION,
        )
        .map_err(consensus_or_protocol_value_error)?
        .unwrap_or_default();

        #[cfg(feature = "validation")]
        if full_validation {
            validation_operations.extend(std::iter::once(
                ProtocolValidationOperation::DocumentTypeSchemaPropertyValidation(
                    property_values.values().len() as u64,
                ),
            ));

            // We should validate that the positions are continuous
            for (pos, value) in property_values.values().enumerate() {
                if value.get_integer::<u32>(property_names::POSITION)? != pos as u32 {
                    return Err(ConsensusError::BasicError(
                        BasicError::MissingPositionsInDocumentTypePropertiesError(
                            MissingPositionsInDocumentTypePropertiesError::new(
                                pos as u32,
                                data_contract_id,
                                name.to_string(),
                            ),
                        ),
                    )
                    .into());
                }
            }
        }

        // Prepare internal data for efficient querying
        let mut flattened_document_properties: IndexMap<String, DocumentProperty> = IndexMap::new();
        let mut document_properties: IndexMap<String, DocumentProperty> = IndexMap::new();

        let required_fields = Value::inner_recursive_optional_array_of_strings(
            schema_map,
            "".to_string(),
            property_names::PROPERTIES,
            property_names::REQUIRED,
        );

        let transient_fields = Value::inner_recursive_optional_array_of_strings(
            schema_map,
            "".to_string(),
            property_names::PROPERTIES,
            property_names::TRANSIENT,
        );

        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            // TODO: It's very inefficient. It must be done in one iteration and flattened properties
            //  must keep a reference? We even could keep only one collection
            insert_values(
                &mut flattened_document_properties,
                &required_fields,
                &transient_fields,
                None,
                property_key.clone(),
                property_value,
                &root_schema,
                data_contact_config,
            )
            .map_err(consensus_or_protocol_data_contract_error)?;

            insert_values_nested(
                &mut document_properties,
                &required_fields,
                &transient_fields,
                property_key,
                property_value,
                &root_schema,
                data_contact_config,
            )
            .map_err(consensus_or_protocol_data_contract_error)?;
        }

        // Initialize indices
        let index_values =
            Value::inner_optional_array_slice_value(schema_map, property_names::INDICES)
                .map_err(consensus_or_protocol_value_error)?;

        #[cfg(feature = "validation")]
        let mut index_names: HashSet<String> = HashSet::new();
        #[cfg(feature = "validation")]
        let mut unique_indices_count = 0;

        #[cfg(feature = "validation")]
        let mut last_non_contested_unique_index_name: Option<String> = None;

        #[cfg(feature = "validation")]
        let mut last_contested_unique_index_name: Option<String> = None;

        #[cfg(feature = "validation")]
        let mut contested_indices_count = 0;

        let indices: BTreeMap<String, Index> = index_values
            .map(|index_values| {
                index_values
                    .iter()
                    .map(|index_value| {
                        // `true`: parser generation 3 exists if and only if the
                        // document meta-schema is v3, which is what admits the
                        // ranked keywords — so there is no version to read here.
                        let index: Index = Index::try_from_value_map(
                            index_value
                                .to_map()
                                .map_err(consensus_or_protocol_value_error)?
                                .as_slice(),
                            true,
                        )
                        .map_err(consensus_or_protocol_data_contract_error)?;

                        #[cfg(feature = "validation")]
                        if full_validation {
                            // `countable` and `rangeCountable` index features
                            // require GroveDB tree variants and query primitives
                            // (CountTree / ProvableCountTree / NonCounted /
                            // AggregateCountOnRange) that only exist from
                            // protocol v12 onward. NOTE: at protocol v12+ the
                            // dispatch routes to `try_from_schema_v2`, but v2
                            // delegates to V1's parser internally for the
                            // shared core — so this body IS reached at v12+
                            // and the `< 12` check is load-bearing, not
                            // defense-in-depth. Without it, v12 contracts
                            // with countable / range_countable indexes would
                            // be rejected here.
                            if index.countable.is_countable()
                                && platform_version.protocol_version < 12
                            {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    UnsupportedFeatureError::new(
                                        "count index".to_string(),
                                        platform_version.protocol_version,
                                    )
                                    .into(),
                                )));
                            }
                            if index.range_countable && platform_version.protocol_version < 12 {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    UnsupportedFeatureError::new(
                                        "range-countable index".to_string(),
                                        platform_version.protocol_version,
                                    )
                                    .into(),
                                )));
                            }

                            validation_operations.extend(std::iter::once(
                                ProtocolValidationOperation::DocumentTypeSchemaIndexValidation(
                                    index.properties.len() as u64,
                                    index.unique,
                                ),
                            ));

                            // Unique indices produces significant load on the system during state validation
                            // so we need to limit their number to prevent of spikes and DoS attacks
                            if index.unique {
                                unique_indices_count += 1;
                                if unique_indices_count
                                    > platform_version
                                        .dpp
                                        .validation
                                        .document_type
                                        .unique_index_limit
                                {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        UniqueIndicesLimitReachedError::new(
                                            name.to_string(),
                                            platform_version
                                                .dpp
                                                .validation
                                                .document_type
                                                .unique_index_limit,
                                            false,
                                        )
                                        .into(),
                                    )));
                                }

                                if let Some(last_contested_unique_index_name) =
                                    last_contested_unique_index_name.as_ref()
                                {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        ContestedUniqueIndexWithUniqueIndexError::new(
                                            name.to_string(),
                                            last_contested_unique_index_name.clone(),
                                            index.name,
                                        )
                                        .into(),
                                    )));
                                }

                                if index.contested_index.is_none() {
                                    last_non_contested_unique_index_name = Some(index.name.clone());
                                }
                            }

                            if index.contested_index.is_some() {
                                contested_indices_count += 1;
                                if contested_indices_count
                                    > platform_version
                                        .dpp
                                        .validation
                                        .document_type
                                        .contested_index_limit
                                {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        UniqueIndicesLimitReachedError::new(
                                            name.to_string(),
                                            platform_version
                                                .dpp
                                                .validation
                                                .document_type
                                                .contested_index_limit,
                                            true,
                                        )
                                        .into(),
                                    )));
                                }

                                if let Some(last_unique_index_name) =
                                    last_non_contested_unique_index_name.as_ref()
                                {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        ContestedUniqueIndexWithUniqueIndexError::new(
                                            name.to_string(),
                                            index.name,
                                            last_unique_index_name.clone(),
                                        )
                                        .into(),
                                    )));
                                }

                                if documents_mutable {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        ContestedUniqueIndexOnMutableDocumentTypeError::new(
                                            name.to_string(),
                                            index.name,
                                        )
                                        .into(),
                                    )));
                                }

                                last_contested_unique_index_name = Some(index.name.clone());
                            }

                            // Index names must be unique for the document type
                            if !index_names.insert(index.name.to_owned()) {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    DuplicateIndexNameError::new(name.to_string(), index.name)
                                        .into(),
                                )));
                            }

                            // Validate indexed properties
                            index.properties.iter().try_for_each(|index_property| {
                                // Do not allow to index already indexed system properties
                                if NOT_ALLOWED_SYSTEM_PROPERTIES
                                    .contains(&index_property.name.as_str())
                                {
                                    return Err(ProtocolError::ConsensusError(Box::new(
                                        SystemPropertyIndexAlreadyPresentError::new(
                                            name.to_owned(),
                                            index.name.to_owned(),
                                            index_property.name.to_owned(),
                                        )
                                        .into(),
                                    )));
                                }

                                // Indexed property must be defined in user schema if it's not a system one
                                if !DocumentType::system_properties_contains(
                                    data_contract_system_version,
                                    contract_config_version,
                                    documents_transferable,
                                    trade_mode,
                                    index_property.name.as_str(),
                                    platform_version,
                                )? {
                                    let property_definition = flattened_document_properties
                                        .get(&index_property.name)
                                        .ok_or_else(|| {
                                            ProtocolError::ConsensusError(Box::new(
                                                UndefinedIndexPropertyError::new(
                                                    name.to_owned(),
                                                    index.name.to_owned(),
                                                    index_property.name.to_owned(),
                                                )
                                                .into(),
                                            ))
                                        })?;

                                    // Validate indexed property type
                                    match &property_definition.property_type {
                                        // Array and objects aren't supported for indexing yet
                                        DocumentPropertyType::Array(_)
                                        | DocumentPropertyType::Object(_)
                                        | DocumentPropertyType::VariableTypeArray(_) => {
                                            Err(ProtocolError::ConsensusError(Box::new(
                                                InvalidIndexPropertyTypeError::new(
                                                    name.to_owned(),
                                                    index.name.to_owned(),
                                                    index_property.name.to_owned(),
                                                    property_definition.property_type.name(),
                                                )
                                                .into(),
                                            )))
                                        }
                                        // Indexed byte array size must be limited
                                        DocumentPropertyType::ByteArray(sizes)
                                            if sizes.max_size.is_none()
                                                || sizes.max_size.unwrap()
                                                    > MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH =>
                                        {
                                            Err(ProtocolError::ConsensusError(Box::new(
                                                InvalidIndexedPropertyConstraintError::new(
                                                    name.to_owned(),
                                                    index.name.to_owned(),
                                                    index_property.name.to_owned(),
                                                    "maxItems".to_string(),
                                                    format!(
                                                        "should be less or equal {}",
                                                        MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH
                                                    ),
                                                )
                                                .into(),
                                            )))
                                        }
                                        // Indexed string length must be limited
                                        DocumentPropertyType::String(sizes)
                                            if sizes.max_length.is_none()
                                                || sizes.max_length.unwrap()
                                                    > MAX_INDEXED_STRING_PROPERTY_LENGTH =>
                                        {
                                            Err(ProtocolError::ConsensusError(Box::new(
                                                InvalidIndexedPropertyConstraintError::new(
                                                    name.to_owned(),
                                                    index.name.to_owned(),
                                                    index_property.name.to_owned(),
                                                    "maxLength".to_string(),
                                                    format!(
                                                        "should be less or equal {}",
                                                        MAX_INDEXED_STRING_PROPERTY_LENGTH
                                                    ),
                                                )
                                                .into(),
                                            )))
                                        }
                                        _ => Ok(()),
                                    }
                                } else {
                                    Ok(())
                                }
                            })?;
                        }

                        Ok((index.name.clone(), index))
                    })
                    .collect::<Result<BTreeMap<String, Index>, ProtocolError>>()
            })
            .transpose()?
            .unwrap_or_default();

        let index_structure =
            IndexLevel::try_from_indices(indices.values(), name, platform_version)?;

        // Collect binary and identifier properties
        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &document_properties,
            &platform_version
                .dpp
                .contract_versions
                .document_type_versions,
        )?;

        let security_level_requirement = schema
            .get_optional_integer::<u8>(property_names::SECURITY_LEVEL_REQUIREMENT)
            .map_err(consensus_or_protocol_value_error)?
            .map(SecurityLevel::try_from)
            .transpose()?
            .unwrap_or(SecurityLevel::HIGH);

        let requires_identity_encryption_bounded_key = schema
            .get_optional_integer::<u8>(property_names::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY)
            .map_err(consensus_or_protocol_value_error)?
            .map(StorageKeyRequirements::try_from)
            .transpose()?;

        let requires_identity_decryption_bounded_key = schema
            .get_optional_integer::<u8>(property_names::REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY)
            .map_err(consensus_or_protocol_value_error)?
            .map(StorageKeyRequirements::try_from)
            .transpose()?;

        let token_costs_value = schema.get_optional_value("tokenCost")?;

        let extract_cost = |key: &str| -> Result<Option<DocumentActionTokenCost>, ProtocolError> {
            token_costs_value
                .and_then(|v| v.get_optional_value(key).transpose())
                .transpose()?
                .map(|action_cost| {
                    // Extract an optional contract_id. Adjust the key if necessary.
                    let target_contract_id = action_cost.get_optional_identifier("contractId")?;
                    // Extract token_contract_position as an integer, then convert it.
                    let token_contract_position =
                        action_cost.get_integer::<TokenContractPosition>("tokenPosition")?;
                    // Extract the token amount.
                    let token_amount = action_cost.get_integer::<TokenAmount>("amount")?;
                    // Extract the token effect
                    let effect = action_cost
                        .get_optional_integer::<u64>("effect")?
                        .map(|int| int.try_into())
                        .transpose()?
                        .unwrap_or(DocumentActionTokenEffect::TransferTokenToContractOwner);

                    #[cfg(feature = "validation")]
                    if full_validation {
                        // contract id is none if we are on our own contract
                        if target_contract_id.is_none() && !token_configurations.contains_key(&token_contract_position) {
                            return Err(ProtocolError::ConsensusError(
                                ConsensusError::BasicError(
                                    BasicError::InvalidTokenPositionError(
                                        InvalidTokenPositionError::new(
                                            token_configurations.last_key_value().map(|(position, _)| *position),
                                            token_contract_position,
                                        ),
                                    ),
                                )
                                    .into(),
                            ));
                        }

                        // If contractId is present and user tries to burn, bail out:
                        if let Some(target_contract_id) = target_contract_id {
                            if target_contract_id == data_contract_id {
                                // we are in the same contract, but we set the data contract id
                                return Err(ProtocolError::ConsensusError(
                                    ConsensusError::BasicError(
                                        BasicError::RedundantDocumentPaidForByTokenWithContractId(RedundantDocumentPaidForByTokenWithContractId::new(target_contract_id))
                                    )
                                        .into(),
                                ));
                            }
                            if effect == DocumentActionTokenEffect::BurnToken {
                                return Err(ProtocolError::ConsensusError(
                                    ConsensusError::BasicError(
                                        BasicError::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(
                                            TokenPaymentByBurningOnlyAllowedOnInternalTokenError::new(
                                                target_contract_id,
                                                token_contract_position,
                                                key.to_string(),
                                            ),
                                        ),
                                    )
                                        .into(),
                                ));
                            }
                        }
                    }

                    // Extract an optional string and map it to the enum, defaulting if missing or unrecognized.
                    let gas_fees_paid_by = action_cost
                        .get_optional_integer::<u64>("gasFeesPaidBy")?
                        .map(|int| int.try_into())
                        .transpose()?
                        .unwrap_or(GasFeesPaidBy::DocumentOwner);

                    Ok(DocumentActionTokenCost {
                        contract_id: target_contract_id,
                        token_contract_position,
                        token_amount,
                        effect,
                        gas_fees_paid_by,
                    })
                })
                .transpose()
        };

        // Note: documentsCountable / rangeCountable schema keys are intentionally
        // ignored here. The v1 parser produces DocumentTypeV1 which has no countable
        // fields. When protocol v12+ is active, the v2 parser is used instead, which
        // reads these keys and produces DocumentTypeV2. The v1 parser should never
        // reject unknown keys — it simply doesn't map them to its output type.

        let token_costs = TokenCostsV0 {
            create: extract_cost("create")?,
            replace: extract_cost("replace")?,
            delete: extract_cost("delete")?,
            transfer: extract_cost("transfer")?,
            update_price: extract_cost("update_price")?,
            purchase: extract_cost("purchase")?,
        }
        .into();

        Ok(DocumentTypeV1 {
            name: String::from(name),
            schema,
            indices,
            index_structure,
            flattened_properties: flattened_document_properties,
            properties: document_properties,
            identifier_paths,
            binary_paths,
            required_fields,
            transient_fields,
            documents_keep_history,
            documents_keep_transfer_history,
            documents_keep_purchase_history,
            documents_keep_pricing_history,
            documents_mutable,
            documents_can_be_deleted,
            documents_transferable,
            trade_mode,
            creation_restriction_mode,
            data_contract_id,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
            security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator,
            token_costs,
        })
    }
}

impl DocumentTypeV2 {
    /// Parses a document type schema with the doctype-level aggregate fields
    /// (`documentsCountable`, `rangeCountable`, `documentsSummable`,
    /// `rangeSummable` and the `documentsAverageable` / `rangeAverageable`
    /// shorthands), then wraps the parsed core in a `DocumentTypeV2` with those
    /// fields set. A copy of the generation-2 wrapper; core parsing goes to
    /// this module's own [`DocumentTypeV1::try_from_schema_generation_3_core`].
    ///
    /// This parser is only reachable from protocol version 14+ (via
    /// CONTRACT_VERSIONS_V6).
    #[allow(clippy::too_many_arguments)]
    fn try_from_schema_generation_3(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // Extract V2-specific fields before the V1 parser consumes the schema map.
        //
        // Note on pre-v12 contracts: contracts created before v12 used the v1 parser
        // which ignores these fields. After v12 upgrade, deserialization uses the v2
        // parser which will read them. This is safe because the contract update path
        // runs through the v2 parser with full_validation=true, and the primary key
        // tree type is set correctly at contract creation time. Pre-v12 contracts
        // can only have these flags if they were explicitly set in the schema — the
        // meta-schema allows them as optional boolean properties.
        let schema_map_opt = schema.to_map().ok();

        let documents_countable = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, DOCUMENTS_COUNTABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?
            .unwrap_or(false);

        // Keep the raw `Option<bool>` so the averageable desugar below
        // can distinguish "field absent (default false)" from
        // "field explicit false" — same explicit-vs-default tracking
        // the Index parser does for its range axes. `range_countable`
        // (the resolved bool) flows into the rest of the logic.
        let range_countable_opt = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_COUNTABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?;
        let range_countable = range_countable_opt.unwrap_or(false);

        // `documentsSummable` names the integer property whose values are
        // summed across all documents of this type. When set, the primary
        // key tree is a `SumTree` (or `ProvableSumTree` if `rangeSummable`
        // is also true). Accepted shapes:
        //   - absent / null → no sum tree
        //   - non-empty string → property name
        //   - empty string → rejected (ValueWrongType)
        let documents_summable: Option<String> = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                schema_map
                    .iter()
                    .find(|(k, _)| k.as_text() == Some(DOCUMENTS_SUMMABLE))
            })
            .map(|(_, v)| match v {
                Value::Null => Ok(None),
                Value::Text(s) if !s.is_empty() => Ok(Some(s.clone())),
                Value::Text(_) => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsSummable must be a non-empty string naming an integer \
                         property, or null"
                            .to_string(),
                    ),
                )),
                _ => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsSummable value must be a string or null".to_string(),
                    ),
                )),
            })
            .transpose()?
            .flatten();

        let range_summable_opt = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_SUMMABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?;
        let range_summable = range_summable_opt.unwrap_or(false);

        // `documentsAverageable` is syntactic sugar for
        // `documentsCountable: true` + `documentsSummable: "<prop>"`.
        // `rangeAverageable` is shorthand for both range_* flags.
        // Both desugar into the underlying flags below.
        let documents_averageable: Option<String> = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                schema_map
                    .iter()
                    .find(|(k, _)| k.as_text() == Some(DOCUMENTS_AVERAGEABLE))
            })
            .map(|(_, v)| match v {
                Value::Null => Ok(None),
                Value::Text(s) if !s.is_empty() => Ok(Some(s.clone())),
                Value::Text(_) => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsAverageable must be a non-empty string naming an integer \
                         property, or null"
                            .to_string(),
                    ),
                )),
                _ => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsAverageable value must be a string or null".to_string(),
                    ),
                )),
            })
            .transpose()?
            .flatten();

        let range_averageable = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_AVERAGEABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?
            .unwrap_or(false);

        // Desugar averageable into count + sum flags. Conflict rules
        // mirror the per-index dispatch: if both `averageable` and
        // `documentsSummable` are set, the property names must match;
        // `documentsCountable: false` alongside `averageable` is a
        // contradiction.
        let (documents_countable, documents_summable, range_countable, range_summable) =
            if let Some(avg_prop) = &documents_averageable {
                if let Some(sum_prop) = &documents_summable {
                    if sum_prop != avg_prop {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "documentsAverageable=\"{}\" conflicts with \
                                 documentsSummable=\"{}\" on document type \"{}\": both name \
                                 the property aggregated into the primary-key sum tree, so \
                                 they must agree (or set only one — documentsAverageable is \
                                 shorthand for documentsCountable + documentsSummable on the \
                                 same property)",
                                avg_prop, sum_prop, name,
                            )),
                        ));
                    }
                }
                // averageable implies countable; explicit
                // `documentsCountable: false` alongside is a contradiction.
                if let Some(schema_map) = schema_map_opt.as_ref() {
                    if let Some(explicit_countable) =
                        Value::inner_optional_bool_value(schema_map, DOCUMENTS_COUNTABLE)
                            .map_err(consensus_or_protocol_value_error)?
                    {
                        if !explicit_countable {
                            return Err(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(format!(
                                    "documentsAverageable=\"{}\" on document type \"{}\" \
                                     implies documentsCountable: true, but the schema \
                                     explicitly sets documentsCountable: false. Remove the \
                                     explicit false (or drop documentsAverageable in favor \
                                     of just documentsSummable).",
                                    avg_prop, name,
                                )),
                            ));
                        }
                    }
                }
                // When `rangeAverageable: true` is set, BOTH range axes
                // are promoted. Reject explicit-`false` contradictions
                // on either axis (silently flipping the author's
                // explicit value would emit the wrong on-disk layout).
                // Omitted / default-false → silently promoted.
                if range_averageable {
                    if range_countable_opt == Some(false) {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "rangeAverageable: true on document type \"{}\" conflicts \
                                 with explicit rangeCountable: false: rangeAverageable is \
                                 shorthand for rangeCountable + rangeSummable on the \
                                 averageable property. Remove the explicit \
                                 `rangeCountable: false` (or drop rangeAverageable in \
                                 favor of rangeSummable alone).",
                                name,
                            )),
                        ));
                    }
                    if range_summable_opt == Some(false) {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "rangeAverageable: true on document type \"{}\" conflicts \
                                 with explicit rangeSummable: false: rangeAverageable is \
                                 shorthand for rangeCountable + rangeSummable on the \
                                 averageable property. Remove the explicit \
                                 `rangeSummable: false` (or drop rangeAverageable in favor \
                                 of rangeCountable alone).",
                                name,
                            )),
                        ));
                    }
                }
                // Promote each range axis independently: `rangeAverageable`
                // (shorthand) sets BOTH; explicit `rangeCountable` /
                // `rangeSummable` only set their own axis. Mirrors the
                // per-index parser at `index/mod.rs` (search for
                // `if range_averageable {`) — without this split, the
                // shorthand `documentsAverageable + rangeSummable: true`
                // would silently flip `range_countable` to true, which
                // diverges from the longhand `documentsCountable +
                // documentsSummable + rangeSummable: true` form
                // (`range_countable` stays false there) and emits a
                // different on-disk tree shape than the author asked
                // for.
                let merged_range_countable = range_countable || range_averageable;
                let merged_range_summable = range_summable || range_averageable;
                (
                    true,
                    Some(avg_prop.clone()),
                    merged_range_countable,
                    merged_range_summable,
                )
            } else if range_averageable {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "rangeAverageable: true on document type \"{}\" requires \
                         documentsAverageable: \"<prop>\" to name the integer property to \
                         average; rangeAverageable on its own has no property to aggregate",
                        name,
                    )),
                ));
            } else {
                (
                    documents_countable,
                    documents_summable,
                    range_countable,
                    range_summable,
                )
            };

        // Cross-validation: `rangeSummable: true` requires
        // `documentsSummable` to be set. (Mirrors count's
        // `rangeCountable implies documentsCountable` rule at the
        // doctype level.) This also catches the
        // `rangeAverageable + no documentsAverageable + no documentsSummable`
        // case above, but the earlier explicit error gives a better
        // message for the averageable-specific path.
        if range_summable && documents_summable.is_none() {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidContractStructure(
                    "rangeSummable: true requires documentsSummable to name an integer \
                     property; range-sum queries on the primary key only make sense on \
                     a sum-bearing doctype"
                        .to_string(),
                ),
            ));
        }

        // Delegate core parsing to this generation's own copy of the core
        let v1 = DocumentTypeV1::try_from_schema_generation_3_core(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        // Convert to V2 and set the new fields
        let mut v2: DocumentTypeV2 = v1.into();
        v2.documents_countable = documents_countable || range_countable;
        v2.range_countable = range_countable;
        v2.documents_summable = documents_summable.clone();
        v2.range_summable = range_summable;

        // `documentsKeepHistory: true` + `documentsSummable: <prop>` IS
        // supported (as of the keep-history sum-aware-reference change).
        // Layout: the per-document subtree at `[..doctype, doc_id]`
        // becomes a `SumTree` (was `NormalTree`); the version bodies
        // under `[..doctype, doc_id, t_N]` stay plain `Item`s (NOT
        // `ItemWithSumItem`) so historical versions don't double-count;
        // the `[..doctype, doc_id, 0]` "current pointer" becomes a
        // `ReferenceWithSumItem` carrying the current version's
        // `sum_property` value. Aggregation walks:
        //
        //   - Per-doc SumTree aggregate = `0`-key's sum_value (= current
        //     version's amount) + 0 from each history Item. Result: the
        //     current version's contribution.
        //   - Doctype-level SumTree aggregate = sum over per-doc SumTree
        //     aggregates = total of CURRENT versions across all docs.
        //
        // On update, rewriting the `0`-key reference with the new
        // version's sum_value triggers grovedb's standard
        // delete-then-insert merk propagation, which carries the delta
        // up to ancestors automatically. No separate shadow tree or
        // parallel bookkeeping. Same `Element::ReferenceWithSumItem`
        // primitive the per-index sum-tree path already uses (see
        // `make_document_reference_with_sum_item` on the rs-drive side).

        // Cross-validate: every index with `summable` set must name the
        // same property as `documents_summable` (if doctype-level
        // summable is set). Reason: grovedb sum trees aggregate `i64`
        // per merk node — there's no per-tree property tag, so all sum
        // contributions feeding into a doctype's storage must come from
        // the same document property. If one index claimed
        // `summable: "fee"` while another claimed `summable: "amount"`
        // they'd both write `ItemWithSumItem` contributions into the
        // same merk hierarchy and produce a meaningless aggregation.
        //
        // We also enforce this when `documents_summable` is unset: in
        // that case every per-index `summable` must agree with all
        // other per-index `summable`s (the first one wins as the
        // canonical name).
        //
        // These checks are structural invariants of the on-disk
        // grovedb sum-tree layout, NOT optional schema lints — mixed
        // sum properties corrupt ancestor aggregation, U64 summable
        // values silently overflow grovedb's `i64` SumValue at insert,
        // and non-required summable properties silently underflow
        // ancestor sums on delete. They run regardless of
        // `full_validation` because this function sits on the
        // untrusted-contract boundary (restore / migration /
        // cache-warmup / future query-side parsing paths may pass
        // `full_validation: false` against attacker-controlled
        // contract bytes — admitting malformed contracts there would
        // let SUM/AVG queries compute over meaningless state while
        // still looking structurally valid). `flattened_properties`
        // and `required_fields` are populated by the V1 parser on
        // both validation paths so the lookups below are safe to
        // execute unconditionally.
        let mut canonical: Option<String> = documents_summable.clone();
        for index in v2.indices.values() {
            if let Some(index_sum_property) = &index.summable {
                match &canonical {
                    Some(existing) if existing != index_sum_property => {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "all `summable` declarations on document type \"{}\" \
                                 must name the same property; saw \"{}\" and \"{}\". \
                                 Sum trees aggregate i64 per merk node and have no \
                                 per-tree property tag — mixed sum properties would \
                                 produce a meaningless aggregation.",
                                name, existing, index_sum_property,
                            )),
                        ));
                    }
                    None => canonical = Some(index_sum_property.clone()),
                    _ => {}
                }
            }
        }

        // Also verify the named property is `type: integer` and
        // listed in `required`. The integer check goes through
        // `v2.flattened_properties` (set by the V1 parser, which
        // resolves $ref). The required check goes through
        // `v2.required_fields`.
        if let Some(prop_name) = &canonical {
            let prop = v2.flattened_properties.get(prop_name).ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                    format!(
                        "summable property \"{}\" referenced by document type \"{}\" \
                         does not exist on that document type",
                        prop_name, name,
                    ),
                ))
            })?;
            // U64 is intentionally NOT accepted: grovedb's sum-tree
            // aggregates `i64`, so a u64 value > i64::MAX would
            // overflow the aggregator silently. Authors who want
            // unbounded positive integers as summable should set
            // the schema's `maximum` explicitly to `i64::MAX`
            // (9_223_372_036_854_775_807) — that bound forces the
            // property-type inference at
            // `property/mod.rs::find_unsigned_integer_type_for_max_value`
            // through `find_integer_type_for_min_and_max_values`'s
            // unsigned branch (still U64 today because max > U32),
            // BUT we also reject U64 unconditionally here so the
            // rule is enforced regardless of the inference path.
            //
            // The accepted list (I64 + I32/U32 + I16/U16 + I8/U8) is
            // the set of integer types that fit losslessly into
            // grovedb's i64 sum value. Without an explicit `maximum
            // <= i64::MAX` on the property, no integer schema
            // currently infers I64 — authors must add either
            // `maximum: 9223372036854775807` or pick a smaller
            // signed/unsigned type that's not U64.
            if !matches!(
                prop.property_type,
                DocumentPropertyType::I64
                    | DocumentPropertyType::I32
                    | DocumentPropertyType::U32
                    | DocumentPropertyType::I16
                    | DocumentPropertyType::U16
                    | DocumentPropertyType::I8
                    | DocumentPropertyType::U8
            ) {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "summable property \"{}\" on document type \"{}\" must be an \
                         integer type whose values fit in i64 (i8..i64 / u8..u32); got \
                         {:?}. U64 is rejected because values above i64::MAX would \
                         overflow grovedb's i64 sum aggregator. To use a positive-only \
                         integer property as summable, either pick u8/u16/u32, OR set the \
                         property's schema `maximum` to 9223372036854775807 (i64::MAX) \
                         AND have it parse as i64 (today this requires a negative \
                         `minimum` to force the signed inference branch; tracked as a \
                         property-inference follow-up).",
                        prop_name, name, prop.property_type,
                    )),
                ));
            }
            if !v2.required_fields.contains(prop_name) {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "summable property \"{}\" on document type \"{}\" must be \
                         listed in the document type's `required` array; a missing \
                         value at insert time would leave the reference with no sum \
                         contribution and silently underflow ancestor sums on delete.",
                        prop_name, name,
                    )),
                ));
            }
        }

        Ok(v2)
    }
}

impl DocumentType {
    /// Dispatches to this module's generation-3 parser and wraps the result.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract::document_type::class_methods) fn try_from_schema_v3(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        DocumentTypeV2::try_from_schema_generation_3(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )
        .map(DocumentType::V2)
    }
}

#[cfg(test)]
mod tests {
    //! Ranked aggregate index keywords — parser-generation gating.
    //!
    //! The keywords live on the *index*, not the document type, but what
    //! admits them is the parser generation the dispatcher selects
    //! (`try_from_schema`: 2 at PV13, 3 at PV14), so the tests belong on this
    //! parser. Two independent halves have to hold on the pre-PV14 side:
    //!
    //!   * `full_validation: true`  — the v2 meta-schema has
    //!     `additionalProperties: false` on index entries and rejects the key.
    //!   * `full_validation: false` — no meta-schema runs at all (check_tx,
    //!     cache warm-up, restore), so the *grammar* itself must not know the
    //!     keyword. That is the smuggling path, and the one worth pinning.
    //!
    //! The PV13 half of every test goes through the real dispatcher
    //! (`DocumentType::try_from_schema`) rather than through anything in this
    //! module, so what it exercises is generation 2 refusing the keys — the
    //! cross-generation behavior, not this generation's internals.
    use super::*;
    use platform_value::platform_value;

    /// Parse through **this** generation, with the platform version and
    /// validation mode spelled out. Used for the PV14 side, which has to be
    /// exercised on both the meta-schema path (`full_validation: true`) and the
    /// structural path (`full_validation: false`).
    fn parse_with(
        schema: Value,
        platform_version: &PlatformVersion,
        full_validation: bool,
    ) -> Result<DocumentTypeV2, ProtocolError> {
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on this platform version");
        DocumentTypeV2::try_from_schema_generation_3(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test_doc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            platform_version,
        )
    }

    /// Parse through the **real dispatcher**, which picks the parser generation
    /// out of the platform version's `try_from_schema` table value. At PV13
    /// that is generation 2; at PV14, this module.
    fn parse_dispatched(
        schema: Value,
        platform_version: &PlatformVersion,
        full_validation: bool,
    ) -> Result<DocumentType, ProtocolError> {
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on this platform version");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test_doc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            platform_version,
        )
    }

    /// A `review` doctype with one index over `restaurantId`, averageable on
    /// `grade`, optionally carrying ranked keywords. Written so the v3
    /// meta-schema's prerequisite rules are satisfied: the ranked
    /// `if`/`then` conditionals demand the literal range keys, and the
    /// `dependentRequired` chain covers the rest
    /// (`rangeAverageable` → `averageable`).
    fn ranked_review_schema(ranked_keys: Vec<(&str, bool)>) -> Value {
        let mut index_entry: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("restaurantId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("grade".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
        ];
        for (key, value) in ranked_keys {
            index_entry.push((Value::Text(key.to_string()), Value::Bool(value)));
        }

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 1,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![
                    Value::Text("restaurantId".to_string()),
                    Value::Text("grade".to_string()),
                ]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![Value::Map(index_entry)]),
            ),
        ])
    }

    fn pv13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("protocol version 13 exists")
    }

    /// PV14 accepts the ranked keywords and carries them onto the parsed index
    /// — both when parsed through this generation directly and when reached
    /// the way production reaches it, through the dispatcher. The dispatcher
    /// half is what pins that `try_from_schema: 3` actually routes here.
    #[test]
    fn ranked_keywords_accepted_at_pv14() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema.clone(), PlatformVersion::latest(), true)
            .expect("meta-schema v3 must accept the ranked index keywords");

        let index = v2
            .indices
            .get("byRestaurant")
            .expect("index parsed under its name");
        assert!(index.ranked_averageable);
        assert!(!index.ranked_countable);
        assert!(!index.ranked_summable);
        assert!(index.range_countable && index.range_summable);

        // Same schema, same platform version, through the real dispatcher.
        let dispatched = parse_dispatched(schema, PlatformVersion::latest(), true)
            .expect("the dispatcher must route PV14 to a generation that accepts the keywords");
        let DocumentType::V2(dispatched) = dispatched else {
            panic!("generation 3 produces a V2-shaped document type");
        };
        assert!(
            dispatched
                .indices
                .get("byRestaurant")
                .expect("index parsed under its name")
                .ranked_averageable,
            "dispatching at PV14 must reach generation 3, not an earlier generation"
        );
    }

    /// PV13 + `full_validation`: generation 2's meta-schema rejects the unknown
    /// index key outright. Routed through the dispatcher so it is genuinely
    /// generation 2 doing the rejecting.
    #[test]
    fn ranked_keywords_rejected_at_pv13_under_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let result = parse_dispatched(schema, pv13(), true);
        assert!(
            result.is_err(),
            "meta-schema v2 must reject rankedAverageable (additionalProperties: false)"
        );
    }

    /// PV13 without `full_validation`: no meta-schema runs, so the structural
    /// grammar has to do the rejecting. Both `true` and `false` are rejected —
    /// the key's mere presence is what a pre-PV14 node refuses, and matching
    /// that exactly is what keeps replay of historical blocks identical.
    #[test]
    fn ranked_keywords_rejected_at_pv13_without_full_validation() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            for value in [true, false] {
                let schema = ranked_review_schema(vec![(key, value)]);
                let result = parse_dispatched(schema, pv13(), false);
                assert!(
                    result.is_err(),
                    "{key}: {value} must be rejected by the structural path at PV13 — the \
                     meta-schema does not run here, so this is the only gate"
                );
                let msg = format!("{:?}", result.unwrap_err());
                assert!(
                    msg.contains("unexpected property name"),
                    "PV13 must reject it as an unknown index key, exactly as a node without \
                     the feature does; got {msg}"
                );
            }
        }
    }

    /// Same schema, PV14, no full validation: accepted. Pins that the gate is
    /// the parser *generation* and not the validation mode.
    #[test]
    fn ranked_keywords_accepted_at_pv14_without_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema, PlatformVersion::latest(), false)
            .expect("PV14 structural parse must accept the ranked keywords");
        assert!(
            v2.indices
                .get("byRestaurant")
                .expect("index parsed under its name")
                .ranked_averageable
        );
    }

    /// The meta-schema's ranked `if`/`then` conditionals are the declarative
    /// half of the structural "ranking needs its range axis" rule:
    /// `rankedCountable: true` without `rangeCountable` fails meta
    /// validation at PV14.
    #[test]
    fn ranked_countable_without_range_countable_rejected_by_meta_schema() {
        // `averageable` + `rangeAverageable` give the index its range axes in
        // *effect*, but `rangeCountable` is not literally present, so the
        // `if rankedCountable == true then require rangeCountable`
        // conditional fails.
        let schema = ranked_review_schema(vec![("rankedCountable", true)]);
        let result = parse_with(schema, PlatformVersion::latest(), true);
        assert!(
            result.is_err(),
            "meta-schema v3 must demand rangeCountable alongside a true \
             rankedCountable"
        );
    }

    /// An index over `restaurantId` carrying exactly one ranked keyword and
    /// no aggregate layout whatsoever — the shape that separates "the key is
    /// present" from "a ranking axis was asked for".
    fn bare_ranked_index_schema(key: &str, value: bool) -> Value {
        let index_entry = Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("restaurantId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (Value::Text(key.to_string()), Value::Bool(value)),
        ]);

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![Value::Text("restaurantId".to_string())]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![index_entry]),
            ),
        ])
    }

    /// The ranked prerequisites are **value-sensitive**. `dependentRequired`
    /// fires on key *presence*, so expressing the opt-out explicitly
    /// (`"rankedCountable": false`) would have been made to demand a range
    /// axis the index never uses — a contract that says "no ranking here"
    /// rejected for not declaring the machinery of a ranking it declined.
    /// The structural parser reads `false` as "no ranking axis"; full
    /// validation at PV14 must agree.
    #[test]
    fn ranked_flags_written_out_as_false_do_not_require_a_range_axis() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            let v2 = parse_with(
                bare_ranked_index_schema(key, false),
                PlatformVersion::latest(),
                true,
            )
            .unwrap_or_else(|e| {
                panic!("`{key}: false` is an opt-out and must pass full validation: {e:?}")
            });

            let index = v2
                .indices
                .get("byRestaurant")
                .expect("index parsed under its name");
            assert!(
                !index.ranked_countable && !index.ranked_summable && !index.ranked_averageable,
                "`{key}: false` must leave every ranking axis off"
            );
            assert!(
                !index.range_countable && !index.range_summable,
                "`{key}: false` must not have conjured a range axis either"
            );
        }
    }

    /// The other half of the same rule: `true` without the matching range
    /// axis is still refused under full validation, on every one of the three
    /// axes. Making the prerequisite value-sensitive must not have made it
    /// toothless.
    #[test]
    fn ranked_flags_set_true_still_require_their_range_axis() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            let result = parse_with(
                bare_ranked_index_schema(key, true),
                PlatformVersion::latest(),
                true,
            );
            assert!(
                result.is_err(),
                "`{key}: true` with no range axis must be rejected under full validation"
            );
        }
    }

    /// Structural counterpart of the check above, on the path where no
    /// meta-schema runs: `rankedCountable` with neither `countable` nor
    /// `rangeCountable` in effect is rejected by the index parser itself.
    #[test]
    fn ranked_countable_without_range_countable_rejected_structurally() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "restaurantId": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0,
                },
            },
            "required": ["restaurantId"],
            "additionalProperties": false,
            "indices": [{
                "name": "byRestaurant",
                "properties": [{ "restaurantId": "asc" }],
                "rankedCountable": true,
            }],
        });
        let result = parse_with(schema, PlatformVersion::latest(), false);
        assert!(
            result.is_err(),
            "rankedCountable with no range-count layout must be rejected structurally"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedCountable") && msg.contains("rangeCountable"),
            "error must name both flags; got {msg}"
        );
    }
}
