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
    DOCUMENTS_KEEP_HISTORY, DOCUMENTS_MUTABLE, DOCUMENTS_SUMMABLE, INDEX_ONLY,
    KEEPS_PRICING_HISTORY, KEEPS_PURCHASE_HISTORY, KEEPS_TRANSFER_HISTORY, RANGE_AVERAGEABLE,
    RANGE_COUNTABLE, RANGE_SUMMABLE, TRADE_MODE, TRANSFERABLE,
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

/// RANKED: the cross-index structural check a generation runs over a
/// document type's parsed indices, before the merged index tree is built.
///
/// Generation 3's implementation rejects the compound-ranked prefix-overlap
/// shape the storage layer cannot lay out; earlier generations pass
/// [`no_ranked_index_structure_check`] — their grammar rejects the
/// `ranked*` keywords, so no index they parse can carry a ranking axis.
/// Unlike [`RankedIndexKeyLengthCheck`] this runs on **every** parse path,
/// not only under `full_validation`: a contract admitted through a
/// non-validating parse would brick the first document insert.
pub(super) type RankedIndexStructureCheck =
    fn(&BTreeMap<String, Index>) -> Result<(), ProtocolError>;

/// The [`RankedIndexStructureCheck`] for a generation that has no ranking
/// axes to constrain.
pub(super) fn no_ranked_index_structure_check(
    _indices: &BTreeMap<String, Index>,
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
    /// Whether the index grammar admits the `ranked*` keywords. Forwarded to
    /// [`Index::try_from_value_map`], which rejects them as unknown keys when
    /// this is `false`.
    pub admit_ranked: bool,
    /// See [`RankedIndexKeyLengthCheck`].
    pub ranked_index_key_length_check: RankedIndexKeyLengthCheck,
    /// See [`RankedIndexStructureCheck`].
    pub ranked_index_structure_check: RankedIndexStructureCheck,

    // ---- TIME RANGE: the other generation-3 addition ----
    /// Whether the index grammar admits the `timeRange` keyword. Forwarded to
    /// [`Index::try_from_value_map`] exactly like `admit_ranked`: when `false`
    /// the key falls through to the unknown-key arm and is rejected as any
    /// pre-generation-3 node rejected it.
    pub admit_time_range: bool,

    // ---- INDEX ONLY: the third generation-3 addition ----
    /// Whether the index grammar admits the `terminal` keyword (indexOnly
    /// document types). Forwarded to [`Index::try_from_value_map`] exactly
    /// like `admit_ranked` and `admit_time_range`. The doc-type-level
    /// `indexOnly` keyword needs no admission flag of its own: it is read
    /// only by the generation-3 driver (`parse_index_only_keyword`), so
    /// earlier generations ignore it exactly as they ignore every other
    /// doctype-level keyword they predate.
    pub admit_index_terminal: bool,
    /// Whether the index grammar admits the `preallocated` keyword
    /// (refersTo-determined indexOnly indexes). Forwarded to
    /// [`Index::try_from_value_map`] exactly like the admissions above.
    pub admit_index_preallocated: bool,
    /// Whether the index grammar admits the `skipIfAbsent` keyword
    /// (conditional-participation indexOnly indexes). Forwarded to
    /// [`Index::try_from_value_map`] exactly like the admissions above.
    pub admit_index_skip_if_absent: bool,
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
/// This is a total function of the table value alone: the version tables pair
/// each `try_from_schema` generation with its `document_type_schema`, so the
/// pairing is enforced where it is authored, not re-checked down here. The
/// grammar a generation admits is gated by its own `ParserGeneration` flags,
/// not by which meta-schema validated the JSON.
#[cfg(feature = "validation")]
pub(super) fn select_document_meta_schema(
    document_type_schema_version: u16,
    method_name: &str,
) -> Result<&'static JSONSchema, ProtocolError> {
    Ok(match document_type_schema_version {
        0 => &*DOCUMENT_META_SCHEMA_V0,
        1 => &*DOCUMENT_META_SCHEMA_V1,
        2 => &*DOCUMENT_META_SCHEMA_V2,
        3 => &*DOCUMENT_META_SCHEMA_V3,
        version => {
            return Err(ProtocolError::UnknownVersionMismatch {
                method: method_name.to_string(),
                known_versions: vec![0, 1, 2, 3],
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
    let meta_schema =
        select_document_meta_schema(document_type_schema_version, meta_schema_method_name)?;

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
    /// Whether the document type being parsed declared `indexOnly: true`.
    /// Read by [`parse_indices`] to default an omitted index `terminal` to
    /// `$ownerId` BEFORE the index structure is built — the level info
    /// stamps the terminal off the `Index`, and the write path reads it off
    /// the level, so the structure must be born already normalized. Only a
    /// generation admitting the keyword can ever pass `true` (the caller
    /// reads it off the schema); generations 1 and 2 always pass `false`.
    index_only: bool,
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
    index_only: bool,
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
        index_only,
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
        &properties.required_fields,
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
            ctx.platform_version,
        )
        .map_err(consensus_or_protocol_data_contract_error)?;

        insert_values_nested(
            &mut document_properties,
            &required_fields,
            &transient_fields,
            true,
            property_key,
            property_value,
            root_schema,
            ctx.data_contact_config,
            ctx.platform_version,
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
    required_fields: &BTreeSet<String>,
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
                        crate::data_contract::document_type::index::IndexGrammarAdmissions {
                            ranked: ctx.generation.admit_ranked,
                            time_range: ctx.generation.admit_time_range,
                            terminal: ctx.generation.admit_index_terminal,
                            preallocated: ctx.generation.admit_index_preallocated,
                            skip_if_absent: ctx.generation.admit_index_skip_if_absent,
                        },
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

                        // TIME RANGE: the source must be a millisecond
                        // timestamp — a system timestamp ($createdAt /
                        // $updatedAt / $transferredAt) or a user `Date`
                        // property. Structural checks (first-property,
                        // range % step, the uniqueness rules — unique only
                        // over non-overlapping windows on `$createdAt` —
                        // and non-contested) already happened in `Index`
                        // parsing; the checks here need the document schema
                        // or the platform version, so they live here. A
                        // generation without the `timeRange` grammar never
                        // parses a transform, so this is a no-op there.
                        if let Some(transform) = &index.time_range {
                            // The overlap factor is the number of index
                            // entries a single document produces on this
                            // index — its write amplification — so its cap
                            // is a versioned system limit rather than a
                            // structural constant: retuning it is a
                            // protocol-version decision, not a code edit.
                            // `None` means a protocol version predating
                            // time-range indexes, which cannot reach here
                            // because the keyword does not parse there.
                            if let Some(max_overlap_factor) = ctx
                                .platform_version
                                .system_limits
                                .max_time_range_overlap_factor
                            {
                                let overlap = transform.overlap_factor();
                                if overlap > max_overlap_factor {
                                    return Err(consensus_or_protocol_data_contract_error(
                                        DataContractError::InvalidContractStructure(format!(
                                            "timeRange overlap factor (range / step = {}) \
                                             exceeds the maximum of {}; a smaller window or a \
                                             larger step is required to bound per-document \
                                             index entries",
                                            overlap, max_overlap_factor
                                        )),
                                    ));
                                }
                            }
                            // The TTL cap is likewise a versioned system
                            // limit — it is what makes billing TTL'd bytes
                            // at a flat processing rate honest, so retuning
                            // it is a protocol-version decision. The lower
                            // bound (`ttl >= range`) is structural and
                            // checked in `Index` parsing.
                            if let Some(ttl_seconds) = transform.ttl_seconds {
                                if let Some(max_ttl) = ctx
                                    .platform_version
                                    .system_limits
                                    .max_time_range_ttl_seconds
                                {
                                    if ttl_seconds > max_ttl {
                                        return Err(consensus_or_protocol_data_contract_error(
                                            DataContractError::InvalidContractStructure(format!(
                                                "timeRange.ttl ({} seconds) exceeds the maximum \
                                                 of {} seconds: the flat ephemeral-storage \
                                                 pricing TTL'd entries bill under is only an \
                                                 honest rate while the lifetime it covers is \
                                                 bounded",
                                                ttl_seconds, max_ttl
                                            )),
                                        ));
                                    }
                                } else {
                                    return Err(consensus_or_protocol_data_contract_error(
                                        DataContractError::InvalidContractStructure(
                                            "timeRange.ttl is not supported by this protocol \
                                             version"
                                                .to_string(),
                                        ),
                                    ));
                                }
                            }
                            let source = transform.source.as_str();
                            let is_system_timestamp = matches!(
                                source,
                                property_names::CREATED_AT
                                    | property_names::UPDATED_AT
                                    | property_names::TRANSFERRED_AT
                            );
                            // A system timestamp is only ever populated when
                            // the schema *requires* it. Without this check a
                            // contract could declare `timeRange.on:
                            // "$createdAt"` on a doctype that never sets
                            // $createdAt: every document would take the null
                            // branch, the index would hold nothing but null
                            // entries, and — the transform being immutable —
                            // the owner could never fix it.
                            if is_system_timestamp && !required_fields.contains(source) {
                                return Err(consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "timeRange.on (\"{}\") names a system timestamp the \
                                         document type does not require; add it to the \
                                         document type's required fields so documents actually \
                                         carry it",
                                        source
                                    )),
                                ));
                            }
                            // Only the system timestamps can be a source. A
                            // user property cannot: the document-schema
                            // grammar has no type that parses to
                            // `DocumentPropertyType::Date` (`type: "string"`
                            // with `format: "date-time"` stays `String`, and
                            // the meta-schema's `type` enum has no `"date"`),
                            // so accepting `Date`-typed user properties here
                            // would be a dead branch advertising a source no
                            // valid contract can declare. Lift this together
                            // with a reachable millisecond-timestamp property
                            // representation, not before.
                            if !is_system_timestamp {
                                return Err(consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "timeRange.on (\"{}\") must name one of the system \
                                         timestamps ($createdAt, $updatedAt or $transferredAt); \
                                         user-defined properties are not supported as a \
                                         time-range source",
                                        source
                                    )),
                                ));
                            }
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

    // INDEX ONLY: an omitted `terminal` on an indexOnly document type means
    // `$ownerId`. Normalize before the index structure is built below — the
    // level info stamps the terminal off the `Index` and the write path
    // reads it off the level, so the structure must be born normalized.
    // Doing it here also keeps every downstream consumer (the walkers, the
    // query planner, the update-immutability comparison) reading one
    // canonical spelling: both spellings of the same index parse to equal
    // `Index` values. `apply_index_only` then validates the normalized set.
    let mut indices = indices;
    if ctx.index_only {
        use crate::document::property_names::OWNER_ID;
        for index in indices.values_mut() {
            if index.terminal.is_none() {
                index.terminal = Some(OWNER_ID.to_string());
            }
        }
    }

    // Cross-index structural check owned by the generation, exactly like
    // the per-property key-length check above: generations whose index
    // grammar rejects the `ranked*` keywords pass the no-op, so the shared
    // core never branches on a version.
    (ctx.generation.ranked_index_structure_check)(&indices)?;

    // TIME RANGE: indices that share a first property may bucket it with
    // different grids (or not at all) — each grid forks into its own index
    // level, keyed by the property name qualified with the grid parameters
    // (`TimeRangeTransform::storage_key`), so a bucketed level never shares
    // a keyspace with a plain level or with another grid's level. The ONE
    // cross-index agreement rule is the TTL: it is deliberately excluded
    // from the grid identity (declaring or changing it must not fork the
    // storage level), so two indexes sharing a grid on one field share one
    // level's subtrees — and a level cannot have two lifecycles. Identical
    // grids must declare identical TTLs (including both declaring none).
    for (name_a, index_a) in indices.iter() {
        let Some(transform_a) = &index_a.time_range else {
            continue;
        };
        for (name_b, index_b) in indices.iter() {
            if name_b <= name_a {
                continue;
            }
            let Some(transform_b) = &index_b.time_range else {
                continue;
            };
            if transform_a.source == transform_b.source
                && transform_a.range_seconds == transform_b.range_seconds
                && transform_a.step_seconds == transform_b.step_seconds
                && transform_a.phase_seconds == transform_b.phase_seconds
                && transform_a.ttl_seconds != transform_b.ttl_seconds
            {
                return Err(consensus_or_protocol_data_contract_error(
                    DataContractError::InvalidContractStructure(format!(
                        "indexes \"{}\" and \"{}\" bucket \"{}\" with the same grid but \
                         different TTLs ({:?} vs {:?} seconds): indexes sharing a grid share \
                         its storage level, and one level cannot have two lifecycles — \
                         declare the same ttl on both (or on neither)",
                        name_a,
                        name_b,
                        transform_a.source,
                        transform_a.ttl_seconds,
                        transform_b.ttl_seconds
                    )),
                ));
            }
        }
    }

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

/// Read the doctype-level `indexOnly` keyword off the raw schema.
///
/// Runs before the core parse because the core takes `schema` by value —
/// same shape as [`parse_doctype_aggregate_keywords`]. Only the generation-3
/// driver calls this; earlier generations ignore the keyword exactly as they
/// ignore every doctype-level keyword they predate (their meta-schemas still
/// reject it under `full_validation`).
pub(super) fn parse_index_only_keyword(schema: &Value) -> Result<bool, ProtocolError> {
    let schema_map_opt = schema.to_map().ok();

    Ok(schema_map_opt
        .as_ref()
        .and_then(|schema_map| {
            Value::inner_optional_bool_value(schema_map, INDEX_ONLY)
                .map_err(consensus_or_protocol_value_error)
                .transpose()
        })
        .transpose()?
        .unwrap_or(false))
}

/// Write the `indexOnly` flag onto the parsed document type, normalize each
/// index's `terminal` (an omitted terminal defaults to `$ownerId`), and run
/// the structural cross-checks the index-only on-disk layout depends on.
///
/// An indexOnly document type has no primary-storage row: the index entries
/// ARE the rows, each terminating in an `Item` keyed by the index's terminal
/// property. Only what is in the indexes exists and is recoverable, which is
/// why every check below is a storage-layout invariant rather than a schema
/// lint. Like [`apply_doctype_aggregates`], this runs regardless of
/// `full_validation`: this function sits on the untrusted-contract boundary,
/// and admitting a malformed indexOnly type through a non-validating parse
/// would brick the first document insert or make deletes unauthorizable.
///
/// Must run AFTER [`apply_doctype_aggregates`] — it rejects the doctype-level
/// aggregate flags, which describe the primary-key tree an indexOnly type
/// does not have.
pub(super) fn apply_index_only(
    document_type: &mut DocumentTypeV2,
    index_only: bool,
    name: &str,
) -> Result<(), ProtocolError> {
    use crate::document::property_names::{CREATED_AT, OWNER_ID};

    let structure_error = |message: String| {
        ProtocolError::DataContractError(DataContractError::InvalidContractStructure(message))
    };

    if !index_only {
        // `terminal` is only meaningful on indexOnly document types: it names
        // the member key that replaces the document id, and a non-indexOnly
        // index keys its members by document id unconditionally.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.terminal.is_some())
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" declares `terminal`, which is only \
                 allowed on indexOnly document types (set `indexOnly: true` on the document \
                 type, or remove the terminal)",
                index_name, name,
            )));
        }
        // Same for `preallocated`: only an indexOnly index's trees are cheap
        // permanent structure whose member entries carry the data — on a
        // normal document type the trees hold references to stored rows and
        // the preallocation/no-prune contract has no meaning.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.preallocated)
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" declares `preallocated`, which is only \
                 allowed on indexOnly document types (set `indexOnly: true` on the document \
                 type, or remove the flag)",
                index_name, name,
            )));
        }
        // Same for `skipIfAbsent`: conditional participation only means
        // anything when the index entries ARE the storage — a stored type's
        // optional properties already have the null index layout.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.skip_if_absent)
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" declares `skipIfAbsent`, which is only \
                 allowed on indexOnly document types (set `indexOnly: true` on the document \
                 type, or remove the flag)",
                index_name, name,
            )));
        }
        return Ok(());
    }

    document_type.index_only = true;

    // ---- doctype-level flags -------------------------------------------
    // Every rejection here names the flag the author must change: silently
    // overriding a flag would emit a document type whose declared behavior
    // and on-disk layout disagree.
    if document_type.documents_mutable {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must set documentsMutable: false: there is no \
             stored row (and no revision) to mutate",
            name,
        )));
    }
    if document_type.documents_transferable != Transferable::Never {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must not be transferable: ownership is embedded \
             in the index entries themselves and cannot be reassigned",
            name,
        )));
    }
    if document_type.trade_mode != TradeMode::None {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must set tradeMode to none: there is no stored \
             row to trade",
            name,
        )));
    }
    if document_type.documents_keep_history
        || document_type.documents_keep_transfer_history
        || document_type.documents_keep_purchase_history
        || document_type.documents_keep_pricing_history
    {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" cannot keep history (documentsKeepHistory / \
             keepsTransferHistory / keepsPurchaseHistory / keepsPricingHistory): documents \
             of this type are only ever created and deleted, and have no stored body to \
             version",
            name,
        )));
    }
    if !document_type.transient_fields.is_empty() {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" cannot declare transient properties: on an \
             indexOnly type only indexed values exist, and a transient property is by \
             definition not stored — the two declarations contradict each other",
            name,
        )));
    }
    if document_type.documents_countable
        || document_type.range_countable
        || document_type.documents_summable.is_some()
        || document_type.range_summable
    {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" cannot use the doctype-level aggregate keywords \
             (documentsCountable / rangeCountable / documentsSummable / rangeSummable / \
             the averageable sugar): they describe the primary-key tree, which an indexOnly \
             type does not have. Use the index-level `countable` / `rangeCountable` / \
             `rankedCountable` flags instead",
            name,
        )));
    }

    if document_type.indices.is_empty() {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must declare at least one index: the indexes \
             are the storage",
            name,
        )));
    }

    // Terminals are already normalized: `parse_indices` defaulted every
    // omitted `terminal` to `$ownerId` before the index structure was built
    // (the same `index_only` value was passed into the core parse), so the
    // structure's level info and the `Index` values below agree, and every
    // check here reads `Some`.

    // ---- per-index rules ------------------------------------------------
    for (index_name, index) in document_type.indices.iter() {
        if index.properties.is_empty() {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" has no properties: an \
                 indexOnly entry is `[…property values, 0, terminal value]`, so at least \
                 one prefix property is required above the terminal",
                index_name, name,
            )));
        }
        if index.unique {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" cannot be unique: \
                 uniqueness is structural on an indexOnly type — one entry per value tuple \
                 and terminal, enforced at insert — and an index without $ownerId already \
                 enforces global uniqueness of its value tuple",
                index_name, name,
            )));
        }
        if index.contested_index.is_some() {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" cannot be contested: the \
                 contested-resource machinery is document-based",
                index_name, name,
            )));
        }
        if !index.null_searchable {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" cannot set nullSearchable: \
                 false: an indexOnly property is either required or an absent-skipping \
                 index's trigger, so no null entries exist to suppress (a skipIfAbsent \
                 index writes nothing for an absent trigger; nullSearchable suppresses \
                 stored-type null-layout entries, which indexOnly types never write)",
                index_name, name,
            )));
        }
        // `skipIfAbsent`: the index participates only for documents that
        // carry its FIRST property — the skip trigger. The trigger sits at
        // position 0 so the whole branch is pruned at the top of the index
        // walk before any tree is inserted: a deeper skip would leave the
        // prefix trees above it inserted-but-unterminated (the merged index
        // structure shares levels across indexes, and upward pruning only
        // runs from a terminal), silently charging for structure no entry
        // uses. The trigger must be a top-level schema property so its
        // presence is a single map lookup shared verbatim by the write
        // walkers (which derive the skip from `required` membership), the
        // probes (which read the flag), and the row commitment — rules
        // below force those three views to agree.
        if index.skip_if_absent {
            let trigger = &index
                .properties
                .first()
                .expect("non-empty checked above")
                .name;
            if trigger.starts_with('$') {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares `skipIfAbsent` \
                     with system property \"{}\" first: the skip trigger must be an \
                     optional schema property — system properties are always present \
                     (or, for $createdAt, forced into `required` when indexed), so the \
                     index could never skip",
                    index_name, name, trigger,
                )));
            }
            if trigger.contains('.') {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares `skipIfAbsent` \
                     with nested property \"{}\" first: the skip trigger must be a \
                     top-level property, so that presence is a single lookup with no \
                     partially-present ancestor states",
                    index_name, name, trigger,
                )));
            }
            if document_type.required_fields.contains(trigger.as_str()) {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares `skipIfAbsent`, \
                     but its first property \"{}\" is listed in `required`: a required \
                     trigger can never be absent, so the index could never skip — remove \
                     \"{}\" from `required` (making this the property's skip trigger) or \
                     drop the flag",
                    index_name, name, trigger, trigger,
                )));
            }
        }
        // `timeRange` is admitted: a bucketed indexOnly index writes one
        // entry per containing bucket, exactly as stored types do (the
        // walkers' bucket fan-out is shared). No indexOnly-specific
        // source rule is needed — the transform's source must be a
        // system timestamp (the shared timeRange rules), it must be the
        // index's first property, and the prefix rule below admits only
        // `$ownerId` and `$createdAt` as system properties, which pins
        // the source to `$createdAt` (the only timestamp an immutable,
        // create-once document carries). Delete-by-values stays
        // deterministic: `$createdAt` is forced into `required` (rule
        // below), so the carried value reproduces the exact bucket set
        // the create wrote. A bucketed index involves `$createdAt` and
        // therefore never counts as the required `$createdAt`-free
        // proof index.
        // The sum axes (summable / rangeSummable / rankedSummable /
        // rankedAverageable / the averageable sugar) are admitted: a
        // summable index's terminal entry is an
        // `ItemWithSumItem(commitment, amount)` carrying the summed
        // property's value, and the doctype-level summable cross-checks
        // (canonical property, i64-safe integer type, `required`
        // membership) run for every doctype, indexOnly included.

        let terminal = index.terminal.as_deref().expect("normalized to Some above");

        if index
            .properties
            .iter()
            .any(|property| property.name == terminal)
        {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" repeats its terminal \
                 (\"{}\") in its properties: the terminal is the member key below the \
                 listed properties, so listing it again would index the same dimension \
                 twice",
                index_name, name, terminal,
            )));
        }

        // The terminal is the member key — it must be a referable entity id:
        // the owner identity, or a property carrying a refersTo declaration
        // whose value alone IS the referenced entity's id (identity,
        // contract, token, or permanent document — all kinds that can never
        // dangle). `identityPublicKey` is deliberately NOT admitted: it is a
        // compound reference — this property carries the identity id while a
        // separate `keyIdProperty` carries the key id — so a terminal keyed
        // by it would conflate references to different keys of the same
        // identity.
        if terminal != OWNER_ID {
            use crate::data_contract::document_type::property::DocumentPropertyReferenceTarget;
            match document_type.flattened_properties.get(terminal) {
                Some(property)
                    if matches!(
                        property.property_type,
                        DocumentPropertyType::IdentifierWithReference(
                            DocumentPropertyReferenceTarget::Identity
                                | DocumentPropertyReferenceTarget::Contract
                                | DocumentPropertyReferenceTarget::Token
                                | DocumentPropertyReferenceTarget::PermanentDocument { .. }
                        )
                    ) => {}
                Some(_) => {
                    return Err(structure_error(format!(
                        "terminal \"{}\" of index \"{}\" on indexOnly document type \"{}\" \
                         must be \"$ownerId\" or an identifier property with a refersTo \
                         declaration targeting identity, contract, token, or \
                         permanentDocument: the terminal is the entry's member key and must \
                         alone be a referable entity id (an identityPublicKey reference is \
                         compound — its key id lives in a separate property — and is not \
                         admitted)",
                        terminal, index_name, name,
                    )));
                }
                None => {
                    return Err(structure_error(format!(
                        "terminal \"{}\" of index \"{}\" on indexOnly document type \"{}\" \
                         does not name a property of the document type",
                        terminal, index_name, name,
                    )));
                }
            }
        }

        // Prefix properties: schema properties plus exactly two system
        // properties — `$ownerId` (ownership) and `$createdAt` (assigned
        // from block time at create, recoverable from the path). Every
        // other system property either cannot exist on an immutable type
        // ($updatedAt and friends) or has no stored home ($revision &co).
        for property in index.properties.iter() {
            if property.name.starts_with('$')
                && property.name != OWNER_ID
                && property.name != CREATED_AT
            {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" indexes system property \
                     \"{}\": only $ownerId and $createdAt may be indexed on an indexOnly \
                     type (documents are immutable, so no other system property can carry \
                     information)",
                    index_name, name, property.name,
                )));
            }
        }

        // EVERY index must embed `$ownerId` (as a prefix property or the
        // terminal). This is what makes each entry self-authorizing: a
        // delete recomputes entries with owner = signer, so an entry the
        // signer does not own is simply not there. With an owner-less
        // index, a crafted delete could splice values from two different
        // documents — its own owner-bearing row and a victim's owner-less
        // row — and remove an entry it never created; binding every entry
        // to its owner closes that, at the cost of the (unneeded) global-
        // uniqueness-without-owner shape.
        if terminal != OWNER_ID
            && !index
                .properties
                .iter()
                .any(|property| property.name == OWNER_ID)
        {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" must include $ownerId (as \
                 a property or as the terminal): every entry must be bound to its owner so \
                 deletes can only ever remove the signer's own entries",
                index_name, name,
            )));
        }

        // `$createdAt` in an index is only coherent when the document
        // actually carries a timestamp — and document creation assigns
        // `created_at` only when `$createdAt` is in `required`. Without
        // this, an indexed `$createdAt` would silently take the missing-
        // value branch instead of storing block time.
        if (terminal == CREATED_AT
            || index
                .properties
                .iter()
                .any(|property| property.name == CREATED_AT))
            && !document_type.required_fields.contains(CREATED_AT)
        {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" involves $createdAt, so \
                 \"$createdAt\" must be listed in `required`: document creation only \
                 assigns the timestamp for required system times, and an indexOnly entry \
                 cannot represent a missing value",
                index_name, name,
            )));
        }

        // `preallocated` promises that the whole index path is a pure
        // function of one same-contract refersTo-referenced document, so the
        // referenced document's insert can create the trees. A bucketed
        // index breaks that promise structurally: its leading level is
        // keyed by grid-qualified bucket starts fanned out from a
        // timestamp, not by a stored property value the binding could
        // resolve — and its `$createdAt` source can never be
        // reference-bound anyway.
        if index.preallocated && index.time_range.is_some() {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" declares `preallocated` \
                 together with `timeRange`: a bucketed level is keyed by bucket starts \
                 computed from a timestamp at write time, so its path cannot be \
                 preallocated from a referenced document",
                index_name, name,
            )));
        }

        // The binding derivation is shared with the rs-drive insert path
        // (see `index::preallocation`); rejecting a flag with no binding
        // here is what lets that path trust every `preallocated: true` it
        // sees.
        if index.preallocated
            && index
                .preallocation_bindings(
                    &document_type.flattened_properties,
                    document_type.data_contract_id,
                )
                .is_empty()
        {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" declares `preallocated`, \
                 but its path is not determined by a reference: every index property must \
                 be either a property with a same-contract permanentDocument `refersTo` \
                 declaration (the referring property — its value is the referenced \
                 document's $id) or a key of that declaration's `propertyAgreement` \
                 (consensus-equal to a referenced-document property). System properties \
                 like $ownerId cannot be determined by the referenced document, so a \
                 preallocated index may carry $ownerId only as its terminal",
                index_name, name,
            )));
        }
    }

    // At least one index must involve no `$createdAt` at all AND not be
    // `skipIfAbsent` — the PROOF index. Executed-transition proofs
    // (waitForStateTransitionResult) locate the entry a create or delete
    // produced from the transition's values alone; a client verifier
    // cannot know the block timestamp an entry was keyed with, and a
    // skipIfAbsent index has no entry at all for trigger-absent documents.
    // If every index were time-keyed or skippable, creates and deletes of
    // the type would work while transition-proof requests failed. (Every
    // index already embeds `$ownerId`, so any `$createdAt`-free non-skip
    // index qualifies as the proof index.)
    let has_proof_index = document_type.indices.values().any(|index| {
        !index.skip_if_absent
            && index.terminal.as_deref() != Some(CREATED_AT)
            && !index
                .properties
                .iter()
                .any(|property| property.name == CREATED_AT)
    });
    if !has_proof_index {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must declare at least one index that neither \
             involves $createdAt nor sets skipIfAbsent: executed-transition proofs locate \
             entries from the transition's values alone — they cannot reproduce the block \
             timestamp a time-keyed entry was written with, and a skipIfAbsent index has \
             no entry for documents that omit its trigger",
            name,
        )));
    }

    // ---- coverage and requiredness --------------------------------------
    // The index content IS the document: a property in no index would not
    // exist, and an absent value has no representation in an index path.
    // The one sanctioned hole is a skipIfAbsent index's trigger: it may be
    // optional because absence removes the whole index entry — there is
    // genuinely nothing to store. Everything else must be required, and
    // must be covered by at least one NON-skip index: a skip index carries
    // no value at all for trigger-absent documents, so a property covered
    // only by skip indexes would be validated, committed into the row
    // commitment, and then written nowhere — unrecoverable by any query,
    // and the document undeletable once the client forgets the value.
    let skip_triggers: BTreeSet<&str> = document_type
        .indices
        .values()
        .filter(|index| index.skip_if_absent)
        .filter_map(|index| index.properties.first())
        .map(|property| property.name.as_str())
        .collect();
    for (property_name, property) in document_type.flattened_properties.iter() {
        if matches!(property.property_type, DocumentPropertyType::Object(_)) {
            // Containers are covered through their flattened leaves.
            continue;
        }
        let is_trigger = skip_triggers.contains(property_name.as_str());
        let covered = document_type.indices.values().any(|index| {
            // A skip index only counts as coverage for its own trigger.
            (is_trigger || !index.skip_if_absent)
                && (index.terminal.as_deref() == Some(property_name.as_str())
                    || index
                        .properties
                        .iter()
                        .any(|index_property| index_property.name == *property_name))
        });
        if !covered {
            return Err(structure_error(format!(
                "property \"{}\" on indexOnly document type \"{}\" does not appear in any \
                 non-skipIfAbsent index (as a property or terminal): on an indexOnly type \
                 only indexed values exist and are recoverable, and a skipIfAbsent index \
                 holds no value at all for documents that omit its trigger, so the \
                 property would be silently dropped",
                property_name, name,
            )));
        }
        if !document_type.required_fields.contains(property_name) {
            if !is_trigger {
                return Err(structure_error(format!(
                    "property \"{}\" on indexOnly document type \"{}\" must be listed in \
                     `required`: the index path is the storage, and an absent value would \
                     need the null index layout this mode deliberately has no equivalent \
                     of (only the first property of a skipIfAbsent index may be optional)",
                    property_name, name,
                )));
            }
            // An optional property is exactly a skip trigger, and every
            // index involving it must be a skipIfAbsent index with the
            // property FIRST (and never as a terminal). This is the
            // invariant the write walkers rely on: they skip a top-level
            // branch keyed by an unrequired property, which is only sound
            // when no non-skip index (and no deeper level of any index)
            // reaches through that branch.
            for (index_name, index) in document_type.indices.iter() {
                if index.terminal.as_deref() == Some(property_name.as_str()) {
                    return Err(structure_error(format!(
                        "optional property \"{}\" on indexOnly document type \"{}\" is the \
                         terminal of index \"{}\": a terminal is every entry's member key \
                         and can never be absent — list the property in `required` or \
                         change the terminal",
                        property_name, name, index_name,
                    )));
                }
                let position = index
                    .properties
                    .iter()
                    .position(|index_property| index_property.name == *property_name);
                match position {
                    None => {}
                    Some(0) if index.skip_if_absent => {}
                    Some(0) => {
                        return Err(structure_error(format!(
                            "optional property \"{}\" on indexOnly document type \"{}\" is \
                             the first property of index \"{}\", which does not set \
                             `skipIfAbsent`: an index participates for every document \
                             unless it skips, and an absent value has no index \
                             representation — set `skipIfAbsent: true` on the index or \
                             list the property in `required`",
                            property_name, name, index_name,
                        )));
                    }
                    Some(_) => {
                        return Err(structure_error(format!(
                            "optional property \"{}\" on indexOnly document type \"{}\" \
                             appears in index \"{}\" below its first position: an optional \
                             property may only be the FIRST property of a skipIfAbsent \
                             index, where absence prunes the whole branch before any tree \
                             is written — deeper, absence would strand the prefix levels \
                             above it",
                            property_name, name, index_name,
                        )));
                    }
                }
            }
        }

        // A required nested leaf inside an OPTIONAL ancestor object is only
        // conditionally present — `required: ["targetId"]` inside an
        // unrequired `profile` lets a valid document omit the whole object.
        // Every ancestor path of an indexed dotted property must therefore
        // be required too, or the no-null invariant silently breaks.
        let mut ancestor = String::new();
        for segment in property_name.split('.') {
            if !ancestor.is_empty() {
                if !document_type.required_fields.contains(&ancestor) {
                    return Err(structure_error(format!(
                        "property \"{}\" on indexOnly document type \"{}\" sits inside \
                         \"{}\", which is not listed in `required`: a valid document could \
                         omit the whole object, leaving the indexed leaf absent",
                        property_name, name, ancestor,
                    )));
                }
                ancestor.push('.');
            }
            ancestor.push_str(segment);
        }
    }

    Ok(())
}
