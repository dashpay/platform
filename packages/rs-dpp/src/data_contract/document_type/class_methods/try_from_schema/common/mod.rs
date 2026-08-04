//! Parameter-pure pieces shared by the document-type parser generations.
//!
//! Every function here is a *pure* function of its arguments: nothing in this
//! module reads
//! `platform_version.dpp.contract_versions.document_type_versions.schema
//! .document_type_schema`, and nothing branches on a protocol version to decide
//! which grammar to admit. All of that variability arrives as
//! [`ParserGeneration`], which each generation module fills in with **its own
//! constants**.
//!
//! That split is what keeps the "new grammar gets a new parser generation" rule
//! intact while the sub-steps stay shared: a shipped generation cannot pick up
//! grammar it did not have, because the grammar it admits is a literal in its
//! own driver rather than a table lookup performed down here.
//!
//! The one place a generation still has to read the table is where its own
//! behavior genuinely varies across the protocol versions it serves — see
//! `v1/mod.rs`, whose entry point serves generation 1 (schema 0) *and* backs
//! generation 2 (schema 1 and 2).

use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_value_error;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::property_names::{
    CAN_BE_DELETED, CREATION_RESTRICTION_MODE, DOCUMENTS_AVERAGEABLE, DOCUMENTS_COUNTABLE,
    DOCUMENTS_KEEP_HISTORY, DOCUMENTS_MUTABLE, DOCUMENTS_SUMMABLE, KEEPS_PRICING_HISTORY,
    KEEPS_PURCHASE_HISTORY, KEEPS_TRANSFER_HISTORY, RANGE_AVERAGEABLE, RANGE_COUNTABLE,
    RANGE_SUMMABLE, TRADE_MODE, TRANSFERABLE,
};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::v0::TokenCostsV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use crate::balances::credits::TokenAmount;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_amount_on_contract_token::{
    DocumentActionTokenCost, DocumentActionTokenEffect,
};
use indexmap::IndexMap;

use super::{insert_values, insert_values_nested};

#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::{
    ContestedUniqueIndexOnMutableDocumentTypeError, ContestedUniqueIndexWithUniqueIndexError,
    InvalidDocumentTypeNameError, RedundantDocumentPaidForByTokenWithContractId,
    TokenPaymentByBurningOnlyAllowedOnInternalTokenError,
};
#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::{
    DuplicateIndexNameError, InvalidIndexPropertyTypeError, InvalidIndexedPropertyConstraintError,
    SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError,
};
#[cfg(feature = "validation")]
use crate::consensus::basic::document::MissingPositionsInDocumentTypePropertiesError;
#[cfg(feature = "validation")]
use crate::consensus::basic::token::InvalidTokenPositionError;
#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::basic::UnsupportedFeatureError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::schema::validate_max_depth;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
#[cfg(feature = "validation")]
use crate::validation::meta_validators::{
    DOCUMENT_META_SCHEMA_V0, DOCUMENT_META_SCHEMA_V1, DOCUMENT_META_SCHEMA_V2,
    DOCUMENT_META_SCHEMA_V3,
};
#[cfg(feature = "validation")]
use jsonschema::JSONSchema;
#[cfg(feature = "validation")]
use std::collections::HashSet;

#[cfg(feature = "validation")]
use super::{
    MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH, MAX_INDEXED_STRING_PROPERTY_LENGTH,
    NOT_ALLOWED_SYSTEM_PROPERTIES,
};

/// RANKED: the extra index-property check a generation runs before the generic
/// index-key limits.
///
/// Only generation 3 has one (the ranked axes tighten the key ceiling); every
/// earlier generation passes [`no_ranked_index_key_length_check`], which is the
/// exact no-op those generations perform today. Passing the check in rather
/// than branching on a version inside the shared core keeps the generation-only
/// rule — and the constants it is derived from — in the generation that owns it.
///
/// This type and its no-op exist only for generation 3; without it there is no
/// per-property hook in the core at all.
pub(super) type RankedIndexKeyLengthCheck =
    fn(&str, &Index, &str, &DocumentPropertyType, &PlatformVersion) -> Result<(), ProtocolError>;

/// The [`RankedIndexKeyLengthCheck`] for a generation that has no ranking axes
/// to constrain — i.e. every generation whose index grammar rejects the
/// `ranked*` keywords outright.
pub(super) fn no_ranked_index_key_length_check(
    _document_type_name: &str,
    _index: &Index,
    _index_property_name: &str,
    _property_type: &DocumentPropertyType,
    _platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    Ok(())
}

/// Everything the shared parsing steps need to know about *which* generation is
/// running them.
///
/// Each generation module constructs one of these from its own constants. The
/// shared code never derives any of these fields from a platform version.
///
/// The fields are split into two groups on purpose: the ranked-aggregate group
/// is exactly what a build without generation 3 does not need, so it can be
/// removed as a block. See the `RANKED` markers through this module for the
/// matching call sites.
pub(super) struct ParserGeneration {
    // ---- present in every generation ----
    /// The `document_type_schema` table value to select the document
    /// meta-schema with. Read by the *driver*, not down here.
    pub document_type_schema_version: u16,
    /// Whether the `keeps*History` document-history subscription flags are part
    /// of this generation's grammar. When `false` they are ignored entirely,
    /// exactly as a node that predated them did.
    pub admit_history: bool,
    /// Whether `countable` / `rangeCountable` index features are admitted.
    /// They require GroveDB tree variants and query primitives (CountTree /
    /// ProvableCountTree / NonCounted / AggregateCountOnRange) that only
    /// exist from protocol v12 onward, so the driver passes `false` below
    /// that boundary and the index is rejected with `UnsupportedFeatureError`.
    pub admit_count_indexes: bool,
    /// Method name reported by the `UnknownVersionMismatch` raised for an
    /// unknown `document_type_schema`. Differs per generation, so it is a
    /// parameter rather than a constant.
    pub meta_schema_method_name: &'static str,

    // ---- RANKED: generation-3 additions ----
    // Every field below exists only because generation 3 does. Drop them
    // together with the `v3` module and the ranked arms they gate, and what is
    // left is the parser as it stood before the ranked aggregates.
    /// Whether document meta-schema v3 is part of this generation's table —
    /// i.e. whether `3` is a known value of `document_type_schema` for it.
    pub admit_meta_schema_v3: bool,
    /// Whether the index grammar admits the `ranked*` keywords. Forwarded to
    /// [`Index::try_from_value_map`], which rejects them as unknown keys when
    /// this is `false`.
    pub admit_ranked: bool,
    /// See [`RankedIndexKeyLengthCheck`].
    pub ranked_index_key_length_check: RankedIndexKeyLengthCheck,
}

/// Reject a document type whose name is not a non-empty ASCII
/// alphanumeric/`_`/`-` string of at most 64 characters.
#[cfg(feature = "validation")]
pub(super) fn validate_document_type_name(name: &str) -> Result<(), ProtocolError> {
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

    Ok(())
}

/// Validate the enriched schema's nesting depth and bill the caller for the
/// schema size.
///
/// The size operation is emitted on *both* paths — the rejection carries the
/// size it measured before returning the depth error — because the work was
/// done either way and the fee must not depend on whether the schema turned out
/// to be valid.
#[cfg(feature = "validation")]
pub(super) fn validate_schema_depth_and_account_for_size(
    root_schema: &Value,
    validation_operations: &mut impl Extend<ProtocolValidationOperation>,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let mut result = validate_max_depth(root_schema, platform_version)?;

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

    Ok(())
}

/// Pick the document meta-schema for a `document_type_schema` table value.
///
/// `admit_meta_schema_v3` is what makes this one match total across every
/// generation without any of them gaining a meta-schema it did not ship with:
/// a generation whose table never names meta-schema v3 passes `false` and keeps
/// reporting `known_versions: [0, 1, 2]`, exactly as its own copy of this match
/// did.
#[cfg(feature = "validation")]
pub(super) fn select_document_meta_schema(
    document_type_schema_version: u16,
    admit_meta_schema_v3: bool,
    method_name: &str,
) -> Result<&'static JSONSchema, ProtocolError> {
    Ok(match document_type_schema_version {
        0 => &*DOCUMENT_META_SCHEMA_V0,
        1 => &*DOCUMENT_META_SCHEMA_V1,
        2 => &*DOCUMENT_META_SCHEMA_V2,
        // RANKED: meta-schema v3 and the widened `known_versions` are
        // generation 3's. Without it this is a plain `vec![0, 1, 2]` arm and
        // the parameter goes away.
        3 if admit_meta_schema_v3 => &*DOCUMENT_META_SCHEMA_V3,
        version => {
            return Err(ProtocolError::UnknownVersionMismatch {
                method: method_name.to_string(),
                known_versions: if admit_meta_schema_v3 {
                    vec![0, 1, 2, 3]
                } else {
                    vec![0, 1, 2]
                },
                received: version,
            })
        }
    })
}

/// Compile the enriched schema to its validating JSON form, check it against
/// the generation's document meta-schema, and prime the document validator.
#[cfg(feature = "validation")]
pub(super) fn validate_against_meta_schema_and_compile(
    root_schema: &Value,
    document_type_schema_version: u16,
    admit_meta_schema_v3: bool,
    meta_schema_method_name: &str,
    json_schema_validator: &StatelessJsonSchemaLazyValidator,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    // Make sure JSON Schema is compilable
    let root_json_schema = root_schema.try_to_validating_json().map_err(|e| {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ValueError(e.into())).into(),
        )
    })?;

    // Select the appropriate document meta-schema based on platform version
    let meta_schema = select_document_meta_schema(
        document_type_schema_version,
        admit_meta_schema_v3,
        meta_schema_method_name,
    )?;

    // Validate against JSON Schema
    meta_schema
        .validate(&root_json_schema)
        .map_err(|mut errs| ConsensusError::from(errs.next().unwrap()))?;

    json_schema_validator.compile(&root_json_schema, platform_version)?;

    Ok(())
}

/// Read the three document-history subscription flags, or return all-`false`
/// without touching the schema when the generation's grammar does not have
/// them.
///
/// Not reading them is the load-bearing half: earlier meta-schema versions
/// either accepted and ignored unknown top-level keys (v0) or rejected them
/// outright (v1), so a historical contract carrying e.g. a *non-boolean* value
/// under one of these names parsed fine on the implementation that produced the
/// block and has to keep doing so.
pub(super) fn parse_keeps_history_flags(
    schema_map: &[(Value, Value)],
    admit_history: bool,
) -> Result<(bool, bool, bool), ProtocolError> {
    if !admit_history {
        return Ok((false, false, false));
    }

    Ok((
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
    ))
}

/// The inputs the stages of [`parse_document_type_core`] share.
///
/// Filled in once from the core's own arguments so that each stage takes its
/// own working data plus one context reference, instead of re-threading the
/// same nine values apiece. Several fields are read only by `validation`-gated
/// checks; they are documented as such below and simply go unread in a build
/// without the feature.
struct CoreParseContext<'a> {
    /// Validation only: names the contract in the property-position and
    /// token-cost errors.
    data_contract_id: Identifier,
    /// Validation only: selects which `$`-prefixed properties this contract's
    /// system schema already provides, and so may not be indexed by hand.
    data_contract_system_version: u16,
    /// Validation only: the same question for the contract config's own
    /// system properties.
    contract_config_version: u16,
    name: &'a str,
    /// Validation only: checks that a token cost names a token position the
    /// contract actually defines.
    token_configurations: &'a BTreeMap<TokenContractPosition, TokenConfiguration>,
    data_contact_config: &'a DataContractConfig,
    /// Whether the stages run their validation-gated checks. Always read
    /// behind `#[cfg(feature = "validation")]`: a build that compiled none of
    /// those checks in has nothing to skip.
    full_validation: bool,
    generation: &'a ParserGeneration,
    platform_version: &'a PlatformVersion,
}

/// The document-type level switches, each falling back to the contract-level
/// default when the schema does not override it.
struct DocumentTypeFlags {
    documents_keep_history: bool,
    documents_keep_transfer_history: bool,
    documents_keep_purchase_history: bool,
    documents_keep_pricing_history: bool,
    documents_mutable: bool,
    documents_can_be_deleted: bool,
    documents_transferable: Transferable,
    trade_mode: TradeMode,
    creation_restriction_mode: CreationRestrictionMode,
}

/// The document type's properties, in both the shapes the parse produces.
struct ParsedProperties {
    /// Sub-objects flattened out, which is what the index stage looks
    /// properties up in.
    flattened_document_properties: IndexMap<String, DocumentProperty>,
    /// The nested form, which keeps sub-objects.
    document_properties: IndexMap<String, DocumentProperty>,
    required_fields: BTreeSet<String>,
    transient_fields: BTreeSet<String>,
}

/// The path sets derived from the parsed properties, together with the
/// security level and key requirements read off the schema.
struct PathsAndKeyRequirements {
    identifier_paths: BTreeSet<String>,
    binary_paths: BTreeSet<String>,
    security_level_requirement: SecurityLevel,
    requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
}

/// The shared parsing core behind every generation from 1 onward.
///
/// Builds the `DocumentTypeV1` value that generations 2 and 3 then layer the
/// doctype-level aggregate fields onto (see
/// [`parse_doctype_aggregate_keywords`] / [`apply_doctype_aggregates`]).
///
/// The body is a pipeline over the stage functions below it; each stage owns
/// one section of the document type schema together with the validation that
/// belongs to that section.
#[allow(clippy::too_many_arguments)]
pub(super) fn parse_document_type_core(
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
    generation: &ParserGeneration,
    platform_version: &PlatformVersion,
) -> Result<DocumentTypeV1, ProtocolError> {
    let ctx = CoreParseContext {
        data_contract_id,
        data_contract_system_version,
        contract_config_version,
        name,
        token_configurations,
        data_contact_config,
        full_validation,
        generation,
        platform_version,
    };

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
        validate_document_type_schema(
            &ctx,
            &root_schema,
            &json_schema_validator,
            validation_operations,
        )?;
    }

    // This has already been validated, but we leave the map_err here for consistency
    let schema_map = schema.to_map().map_err(|err| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            format!("document schema must be an object: {err}"),
        ))
    })?;

    let flags = parse_document_type_flags(&ctx, schema_map)?;

    let properties =
        parse_document_properties(&ctx, schema_map, &root_schema, validation_operations)?;

    let (indices, index_structure) = parse_indices(
        &ctx,
        schema_map,
        &flags,
        &properties.flattened_document_properties,
        validation_operations,
    )?;

    let paths_and_key_requirements =
        parse_paths_and_key_requirements(&ctx, &schema, &properties.document_properties)?;

    // Note: the doctype-level aggregate keys (documentsCountable /
    // rangeCountable / documentsSummable / rangeSummable and the averageable
    // shorthands) are intentionally ignored here. This core produces a
    // `DocumentTypeV1`, which has no aggregate fields; the generations that do
    // carry them read those keys in their own wrapper (see
    // `parse_doctype_aggregate_keywords`). The core must never *reject* unknown
    // keys — it simply doesn't map them to its output type.

    let token_costs = parse_token_costs(&ctx, &schema)?;

    let DocumentTypeFlags {
        documents_keep_history,
        documents_keep_transfer_history,
        documents_keep_purchase_history,
        documents_keep_pricing_history,
        documents_mutable,
        documents_can_be_deleted,
        documents_transferable,
        trade_mode,
        creation_restriction_mode,
    } = flags;
    let ParsedProperties {
        flattened_document_properties,
        document_properties,
        required_fields,
        transient_fields,
    } = properties;
    let PathsAndKeyRequirements {
        identifier_paths,
        binary_paths,
        security_level_requirement,
        requires_identity_encryption_bounded_key,
        requires_identity_decryption_bounded_key,
    } = paths_and_key_requirements;

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

/// Everything `full_validation` asks of the schema before anything is parsed
/// out of it: the document type's name, the enriched schema's depth (and the
/// fee for its size), and the schema itself against this generation's document
/// meta-schema — which also primes the document validator.
#[cfg(feature = "validation")]
fn validate_document_type_schema(
    ctx: &CoreParseContext<'_>,
    root_schema: &Value,
    json_schema_validator: &StatelessJsonSchemaLazyValidator,
    validation_operations: &mut impl Extend<ProtocolValidationOperation>,
) -> Result<(), ProtocolError> {
    // Make sure a document type name is compliant
    validate_document_type_name(ctx.name)?;

    // Validate document schema depth
    validate_schema_depth_and_account_for_size(
        root_schema,
        validation_operations,
        ctx.platform_version,
    )?;

    validate_against_meta_schema_and_compile(
        root_schema,
        ctx.generation.document_type_schema_version,
        ctx.generation.admit_meta_schema_v3,
        ctx.generation.meta_schema_method_name,
        json_schema_validator,
        ctx.platform_version,
    )?;

    Ok(())
}

/// The document-type level switches, read straight off the schema map.
fn parse_document_type_flags(
    ctx: &CoreParseContext<'_>,
    schema_map: &[(Value, Value)],
) -> Result<DocumentTypeFlags, ProtocolError> {
    // Do documents of this type keep history? (Overrides contract value)
    let documents_keep_history: bool =
        Value::inner_optional_bool_value(schema_map, DOCUMENTS_KEEP_HISTORY)
            .map_err(consensus_or_protocol_value_error)?
            .unwrap_or(
                ctx.data_contact_config
                    .documents_keep_history_contract_default(),
            );

    let (
        documents_keep_transfer_history,
        documents_keep_purchase_history,
        documents_keep_pricing_history,
    ) = parse_keeps_history_flags(schema_map, ctx.generation.admit_history)?;

    // Are documents of this type mutable? (Overrides contract value)
    let documents_mutable: bool = Value::inner_optional_bool_value(schema_map, DOCUMENTS_MUTABLE)
        .map_err(consensus_or_protocol_value_error)?
        .unwrap_or(ctx.data_contact_config.documents_mutable_contract_default());

    // Can documents of this type be deleted? (Overrides contract value)
    let documents_can_be_deleted: bool =
        Value::inner_optional_bool_value(schema_map, CAN_BE_DELETED)
            .map_err(consensus_or_protocol_value_error)?
            .unwrap_or(
                ctx.data_contact_config
                    .documents_can_be_deleted_contract_default(),
            );

    // Are documents of this type transferable?
    let documents_transferable_u8: u8 =
        Value::inner_optional_integer_value(schema_map, TRANSFERABLE)
            .map_err(consensus_or_protocol_value_error)?
            .unwrap_or_default();

    let documents_transferable = documents_transferable_u8.try_into()?;

    // What is the trade mode of these documents
    let documents_trade_mode_u8: u8 = Value::inner_optional_integer_value(schema_map, TRADE_MODE)
        .map_err(consensus_or_protocol_value_error)?
        .unwrap_or_default();

    let trade_mode = documents_trade_mode_u8.try_into()?;

    // What is the creation restriction mode of this document type?
    let documents_creation_restriction_mode_u8: u8 =
        Value::inner_optional_integer_value(schema_map, CREATION_RESTRICTION_MODE)
            .map_err(consensus_or_protocol_value_error)?
            .unwrap_or_default();

    let creation_restriction_mode = documents_creation_restriction_mode_u8.try_into()?;

    Ok(DocumentTypeFlags {
        documents_keep_history,
        documents_keep_transfer_history,
        documents_keep_purchase_history,
        documents_keep_pricing_history,
        documents_mutable,
        documents_can_be_deleted,
        documents_transferable,
        trade_mode,
        creation_restriction_mode,
    })
}

/// The document type's properties, in both the flattened and the nested
/// form, together with the required and transient field sets they are built
/// against.
///
/// `validation_operations` is only extended when validation is compiled in.
#[cfg_attr(not(feature = "validation"), allow(unused_variables))]
fn parse_document_properties(
    ctx: &CoreParseContext<'_>,
    schema_map: &[(Value, Value)],
    root_schema: &Value,
    validation_operations: &mut impl Extend<ProtocolValidationOperation>,
) -> Result<ParsedProperties, ProtocolError> {
    // Extract the properties
    let property_values = Value::inner_optional_index_map::<u64>(
        schema_map,
        property_names::PROPERTIES,
        property_names::POSITION,
    )
    .map_err(consensus_or_protocol_value_error)?
    .unwrap_or_default();

    #[cfg(feature = "validation")]
    if ctx.full_validation {
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
                            ctx.data_contract_id,
                            ctx.name.to_string(),
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
            root_schema,
            ctx.data_contact_config,
        )
        .map_err(consensus_or_protocol_data_contract_error)?;

        insert_values_nested(
            &mut document_properties,
            &required_fields,
            &transient_fields,
            property_key,
            property_value,
            root_schema,
            ctx.data_contact_config,
        )
        .map_err(consensus_or_protocol_data_contract_error)?;
    }

    Ok(ParsedProperties {
        flattened_document_properties,
        document_properties,
        required_fields,
        transient_fields,
    })
}

/// The document type's indices: the index grammar this generation admits,
/// the admission checks for index features it does not have, the per-index
/// validation limits, and the index tree built from the result.
///
/// Which keywords an index may carry and which of them this generation admits
/// is one decision, so it is deliberately one function.
///
/// `flags`, `flattened_document_properties` and `validation_operations` are
/// only read by the validation-gated checks.
#[cfg_attr(not(feature = "validation"), allow(unused_variables))]
fn parse_indices(
    ctx: &CoreParseContext<'_>,
    schema_map: &[(Value, Value)],
    flags: &DocumentTypeFlags,
    flattened_document_properties: &IndexMap<String, DocumentProperty>,
    validation_operations: &mut impl Extend<ProtocolValidationOperation>,
) -> Result<(BTreeMap<String, Index>, IndexLevel), ProtocolError> {
    // Initialize indices
    let index_values = Value::inner_optional_array_slice_value(schema_map, property_names::INDICES)
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
                    // RANKED: whether the `ranked*` keywords are part of the
                    // grammar at all is the generation's own constant. When it
                    // is `false` they fall through to the unknown-key arm and
                    // are rejected with exactly the error the generation always
                    // produced — which is what `TryFrom<&[(Value, Value)]> for
                    // Index` does, so without generation 3 this whole call
                    // collapses back to `.as_slice().try_into()`.
                    let index: Index = Index::try_from_value_map(
                        index_value
                            .to_map()
                            .map_err(consensus_or_protocol_value_error)?
                            .as_slice(),
                        ctx.generation.admit_ranked,
                    )
                    .map_err(consensus_or_protocol_data_contract_error)?;

                    #[cfg(feature = "validation")]
                    if ctx.full_validation {
                        // This check is load-bearing, not defense-in-depth:
                        // v2 delegates to V1's parser internally for the
                        // shared core, so this body serves both sides of
                        // the count-index boundary and the driver's
                        // `admit_count_indexes` decides which side we are
                        // on.
                        if index.countable.is_countable() && !ctx.generation.admit_count_indexes {
                            return Err(ProtocolError::ConsensusError(Box::new(
                                UnsupportedFeatureError::new(
                                    "count index".to_string(),
                                    ctx.platform_version.protocol_version,
                                )
                                .into(),
                            )));
                        }
                        if index.range_countable && !ctx.generation.admit_count_indexes {
                            return Err(ProtocolError::ConsensusError(Box::new(
                                UnsupportedFeatureError::new(
                                    "range-countable index".to_string(),
                                    ctx.platform_version.protocol_version,
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
                                > ctx
                                    .platform_version
                                    .dpp
                                    .validation
                                    .document_type
                                    .unique_index_limit
                            {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    UniqueIndicesLimitReachedError::new(
                                        ctx.name.to_string(),
                                        ctx.platform_version
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
                                        ctx.name.to_string(),
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
                                > ctx
                                    .platform_version
                                    .dpp
                                    .validation
                                    .document_type
                                    .contested_index_limit
                            {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    UniqueIndicesLimitReachedError::new(
                                        ctx.name.to_string(),
                                        ctx.platform_version
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
                                        ctx.name.to_string(),
                                        index.name,
                                        last_unique_index_name.clone(),
                                    )
                                    .into(),
                                )));
                            }

                            if flags.documents_mutable {
                                return Err(ProtocolError::ConsensusError(Box::new(
                                    ContestedUniqueIndexOnMutableDocumentTypeError::new(
                                        ctx.name.to_string(),
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
                                DuplicateIndexNameError::new(ctx.name.to_string(), index.name)
                                    .into(),
                            )));
                        }

                        // Validate indexed properties
                        validate_index_properties(
                            ctx,
                            &index,
                            flags,
                            flattened_document_properties,
                        )?;
                    }

                    Ok((index.name.clone(), index))
                })
                .collect::<Result<BTreeMap<String, Index>, ProtocolError>>()
        })
        .transpose()?
        .unwrap_or_default();

    let index_structure =
        IndexLevel::try_from_indices(indices.values(), ctx.name, ctx.platform_version)?;

    Ok((indices, index_structure))
}

/// The per-property half of index validation: an already-indexed system
/// property may not be indexed again, a user property must be defined, and an
/// indexed property's type must be one the index encoding supports within its
/// key length limits.
#[cfg(feature = "validation")]
fn validate_index_properties(
    ctx: &CoreParseContext<'_>,
    index: &Index,
    flags: &DocumentTypeFlags,
    flattened_document_properties: &IndexMap<String, DocumentProperty>,
) -> Result<(), ProtocolError> {
    index.properties.iter().try_for_each(|index_property| {
        // Do not allow to index already indexed system properties
        if NOT_ALLOWED_SYSTEM_PROPERTIES.contains(&index_property.name.as_str()) {
            return Err(ProtocolError::ConsensusError(Box::new(
                SystemPropertyIndexAlreadyPresentError::new(
                    ctx.name.to_owned(),
                    index.name.to_owned(),
                    index_property.name.to_owned(),
                )
                .into(),
            )));
        }

        // Indexed property must be defined in user schema if it's not a system one
        if !DocumentType::system_properties_contains(
            ctx.data_contract_system_version,
            ctx.contract_config_version,
            flags.documents_transferable,
            flags.trade_mode,
            index_property.name.as_str(),
            ctx.platform_version,
        )? {
            let property_definition = flattened_document_properties
                .get(&index_property.name)
                .ok_or_else(|| {
                    ProtocolError::ConsensusError(Box::new(
                        UndefinedIndexPropertyError::new(
                            ctx.name.to_owned(),
                            index.name.to_owned(),
                            index_property.name.to_owned(),
                        )
                        .into(),
                    ))
                })?;

            // RANKED: a ranking axis tightens the generic key
            // limits below: the property's encoded value
            // becomes the item key of a grovedb indexed
            // tree, whose ordered secondary prefixes it
            // with a sort key. Checked before the generic
            // limits so a ranked index reports the bound
            // that actually applies to it. System
            // properties skip this the same way they skip
            // the generic limits — every one of them
            // encodes to a fixed 32 bytes or fewer. A
            // generation without ranking axes passes the
            // no-op check.
            (ctx.generation.ranked_index_key_length_check)(
                ctx.name,
                index,
                index_property.name.as_str(),
                &property_definition.property_type,
                ctx.platform_version,
            )?;

            // Validate indexed property type
            match &property_definition.property_type {
                // Array and objects aren't supported for indexing yet
                DocumentPropertyType::Array(_)
                | DocumentPropertyType::Object(_)
                | DocumentPropertyType::VariableTypeArray(_) => {
                    Err(ProtocolError::ConsensusError(Box::new(
                        InvalidIndexPropertyTypeError::new(
                            ctx.name.to_owned(),
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
                        || sizes.max_size.unwrap() > MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH =>
                {
                    Err(ProtocolError::ConsensusError(Box::new(
                        InvalidIndexedPropertyConstraintError::new(
                            ctx.name.to_owned(),
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
                        || sizes.max_length.unwrap() > MAX_INDEXED_STRING_PROPERTY_LENGTH =>
                {
                    Err(ProtocolError::ConsensusError(Box::new(
                        InvalidIndexedPropertyConstraintError::new(
                            ctx.name.to_owned(),
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
    })
}

/// The identifier and binary paths implied by the parsed properties, plus
/// the security level and the encryption/decryption key requirements the
/// schema asks for.
fn parse_paths_and_key_requirements(
    ctx: &CoreParseContext<'_>,
    schema: &Value,
    document_properties: &IndexMap<String, DocumentProperty>,
) -> Result<PathsAndKeyRequirements, ProtocolError> {
    // Collect binary and identifier properties
    let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
        document_properties,
        &ctx.platform_version
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

    Ok(PathsAndKeyRequirements {
        identifier_paths,
        binary_paths,
        security_level_requirement,
        requires_identity_encryption_bounded_key,
        requires_identity_decryption_bounded_key,
    })
}

/// The token costs attached to each document action.
///
/// `ctx` is only read by the validation-gated checks on those costs.
#[cfg_attr(not(feature = "validation"), allow(unused_variables))]
fn parse_token_costs(
    ctx: &CoreParseContext<'_>,
    schema: &Value,
) -> Result<TokenCosts, ProtocolError> {
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
                    if ctx.full_validation {
                        // contract id is none if we are on our own contract
                        if target_contract_id.is_none() && !ctx.token_configurations.contains_key(&token_contract_position) {
                            return Err(ProtocolError::ConsensusError(
                                ConsensusError::BasicError(
                                    BasicError::InvalidTokenPositionError(
                                        InvalidTokenPositionError::new(
                                            ctx.token_configurations.last_key_value().map(|(position, _)| *position),
                                            token_contract_position,
                                        ),
                                    ),
                                )
                                    .into(),
                            ));
                        }

                        // If contractId is present and user tries to burn, bail out:
                        if let Some(target_contract_id) = target_contract_id {
                            if target_contract_id == ctx.data_contract_id {
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

    Ok(TokenCostsV0 {
        create: extract_cost("create")?,
        replace: extract_cost("replace")?,
        delete: extract_cost("delete")?,
        transfer: extract_cost("transfer")?,
        update_price: extract_cost("update_price")?,
        purchase: extract_cost("purchase")?,
    }
    .into())
}

/// The doctype-level aggregate configuration, already desugared.
///
/// Produced by [`parse_doctype_aggregate_keywords`] *before* the core parse
/// consumes the schema, and applied by [`apply_doctype_aggregates`] afterwards.
pub(super) struct DoctypeAggregates {
    documents_countable: bool,
    documents_summable: Option<String>,
    range_countable: bool,
    range_summable: bool,
}

/// Read the doctype-level aggregate keywords off the raw schema and desugar the
/// `documentsAverageable` / `rangeAverageable` shorthands into the underlying
/// count and sum flags.
///
/// Runs before the core parse because the core takes `schema` by value.
pub(super) fn parse_doctype_aggregate_keywords(
    schema: &Value,
    name: &str,
) -> Result<DoctypeAggregates, ProtocolError> {
    // Extract the aggregate fields before the core parser consumes the schema map.
    //
    // Note on pre-v12 contracts: contracts created before v12 used the
    // generation-1 parser, which ignores these fields. After v12 upgrade,
    // deserialization uses the generation-2 parser which will read them. This
    // is safe because the contract update path runs through that parser with
    // full_validation=true, and the primary key tree type is set correctly at
    // contract creation time. Pre-v12 contracts can only have these flags if
    // they were explicitly set in the schema — the meta-schema allows them as
    // optional boolean properties.
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

    Ok(DoctypeAggregates {
        documents_countable,
        documents_summable,
        range_countable,
        range_summable,
    })
}

/// Write the desugared aggregate configuration onto the parsed document type
/// and run the structural cross-checks the on-disk sum-tree layout depends on.
pub(super) fn apply_doctype_aggregates(
    document_type: &mut DocumentTypeV2,
    aggregates: DoctypeAggregates,
    name: &str,
) -> Result<(), ProtocolError> {
    let DoctypeAggregates {
        documents_countable,
        documents_summable,
        range_countable,
        range_summable,
    } = aggregates;

    document_type.documents_countable = documents_countable || range_countable;
    document_type.range_countable = range_countable;
    document_type.documents_summable = documents_summable.clone();
    document_type.range_summable = range_summable;

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
    // and `required_fields` are populated by the core parser on
    // both validation paths so the lookups below are safe to
    // execute unconditionally.
    let mut canonical: Option<String> = documents_summable.clone();
    for index in document_type.indices.values() {
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
    // `flattened_properties` (set by the core parser, which
    // resolves $ref). The required check goes through
    // `required_fields`.
    if let Some(prop_name) = &canonical {
        let prop = document_type
            .flattened_properties
            .get(prop_name)
            .ok_or_else(|| {
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
        if !document_type.required_fields.contains(prop_name) {
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

    Ok(())
}
