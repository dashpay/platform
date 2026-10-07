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

use crate::data_contract::config::moderation::SettledDeletionRule;
use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::v2::DataContractConfigGettersV2;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_value_error;
use crate::data_contract::document_type::index::{
    reads_through_reference, Index, IndexGrammarAdmissions, IntegerRangeKeyType,
};
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::property::{
    is_transient, property_at_path, top_level_property, DocumentPropertyReferenceTarget,
    KeyIdReference, KeyReferenceIdentityProperty, PropertyReference, ReferenceHolder,
};
use crate::data_contract::document_type::property_constraints::{
    parse_property_constraint_condition, PropertyConstraint, SystemProperty, STORED_DOCUMENT_PREFIX,
};
use crate::data_contract::document_type::property_names::moderator_abilities::{
    delete_settled, CHANGE_FIELDS, DELETE, DELETE_KEEPS_FIELDS, DELETE_KEEPS_RECORD,
    DELETE_REFUNDS_OWNER, DELETE_SETTLED, DELETE_WITHIN,
};
use crate::data_contract::document_type::property_names::{
    CAN_BE_DELETED, CAN_BE_DELETED_ONLY_WHEN_CONSUMED, CREATION_RESTRICTION_MODE,
    DOCUMENTS_AVERAGEABLE, DOCUMENTS_COUNTABLE, DOCUMENTS_KEEP_HISTORY, DOCUMENTS_MUTABLE,
    DOCUMENTS_SUMMABLE, INDEX_ONLY, KEEPS_PRICING_HISTORY, KEEPS_PURCHASE_HISTORY,
    KEEPS_TRANSFER_HISTORY, MODERATOR_ABILITIES, RANGE_AVERAGEABLE, RANGE_COUNTABLE,
    RANGE_SUMMABLE, TRADE_MODE, TRANSFERABLE,
};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::v0::TokenCostsV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::document::property_names::{
    CREATED_AT, ID, MODERATED_AT, MODERATED_BY, OWNER_ID, UPDATED_AT,
};
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
    DuplicateIndexNameError, SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
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
use super::NOT_ALLOWED_SYSTEM_PROPERTIES;

/// The system properties a moderator's write of the fields a type keeps for its moderators
/// stamps on the document: indexable from generation 3, on such a type only.
const MODERATION_STAMP_PROPERTIES: [&str; 2] = [MODERATED_AT, MODERATED_BY];
use super::{MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH, MAX_INDEXED_STRING_PROPERTY_LENGTH};
use crate::consensus::basic::data_contract::{
    InvalidIndexPropertyTypeError, InvalidIndexedPropertyConstraintError,
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
    /// Whether the index grammar admits the `integerRange` keyword. Forwarded
    /// to [`Index::try_from_value_map`] exactly like `admit_time_range`.
    pub admit_integer_range: bool,

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
    /// Whether the index grammar admits the `outlivesDelete` keyword
    /// (indexOnly `timeRange` indexes with a `ttl`). Forwarded to
    /// [`Index::try_from_value_map`] exactly like the admissions above.
    pub admit_index_outlives_delete: bool,
    /// Whether the index grammar admits the `skipIfAbsent` keyword
    /// (conditional-participation indexOnly indexes). Forwarded to
    /// [`Index::try_from_value_map`] exactly like the admissions above.
    pub admit_index_skip_if_absent: bool,
    /// Whether the index grammar admits the `summableOffCountIndex` keyword (summableOffCountIndex
    /// indexOnly indexes). Forwarded to [`Index::try_from_value_map`] exactly
    /// like the admissions above.
    pub admit_index_summable_off_count_index: bool,
    /// Whether `rangeCountable: true` promotes an omitted `countable` to
    /// `"countable"` (generation 3 and later), as the doctype-level
    /// `rangeCountable` has always implied `documentsCountable`. Forwarded to
    /// [`Index::try_from_value_map`] exactly like the admissions above.
    pub admit_range_countable_implies_countable: bool,
    /// Whether a contested index may declare `"resolution": 1`, the masternode
    /// vote without a Lock choice. Forwarded to [`Index::try_from_value_map`]
    /// exactly like the admissions above.
    pub admit_index_no_locking_resolution: bool,
    /// Whether an index may name `$moderatedAt` or `$moderatedBy`, the stamp a moderator's
    /// write of the fields a type keeps for its moderators leaves. Admitted here for every
    /// type; `apply_moderator_abilities` then refuses it on a type that keeps no such field,
    /// and in a unique index.
    pub admit_moderation_stamp_indexes: bool,
    /// Whether an index may name a value of the document a reference of the type points at,
    /// as `"<reference property>.<field>"`: a derived index property. Admitted here for every
    /// name whose first segment is a top-level identifier carrying a `refersTo`;
    /// `apply_derived_index_properties` then judges the reference and the index, and the
    /// contract's parse the referenced field.
    pub admit_derived_index_properties: bool,
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
    /// Whether the document type being parsed declared `canBeDeleted: "onlyWhenConsumed"`,
    /// which [`parse_document_type_flags`] reads as `false` for the owner's delete instead of
    /// reading `canBeDeleted` as a boolean. Only a generation admitting the value can ever pass
    /// `true` (the caller reads it off the schema with
    /// [`parse_can_be_deleted_only_when_consumed_keyword`]); generations 1 and 2 always pass
    /// `false`.
    deleted_only_when_consumed: bool,
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
    deleted_only_when_consumed: bool,
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
        deleted_only_when_consumed,
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

    // Can documents of this type be deleted by their owners? (Overrides contract value)
    // `"onlyWhenConsumed"` says they can not: only a `refersTo` with `consume` deletes them
    let documents_can_be_deleted: bool = if ctx.deleted_only_when_consumed {
        false
    } else {
        Value::inner_optional_bool_value(schema_map, CAN_BE_DELETED)
            .map_err(consensus_or_protocol_value_error)?
            .unwrap_or(
                ctx.data_contact_config
                    .documents_can_be_deleted_contract_default(),
            )
    };

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

    // Every property is in the flattened map now, so a `distinctFrom` target
    // can be resolved against its siblings. Gated on the same version that
    // parsed the declarations, so the two halves of the rule move together.
    super::validate_distinct_from_targets(
        &flattened_document_properties,
        ctx.name,
        ctx.platform_version,
    )
    .map_err(consensus_or_protocol_data_contract_error)?;

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
                    let mut index: Index = Index::try_from_value_map(
                        index_value
                            .to_map()
                            .map_err(consensus_or_protocol_value_error)?
                            .as_slice(),
                        IndexGrammarAdmissions {
                            ranked: ctx.generation.admit_ranked,
                            time_range: ctx.generation.admit_time_range,
                            integer_range: ctx.generation.admit_integer_range,
                            terminal: ctx.generation.admit_index_terminal,
                            preallocated: ctx.generation.admit_index_preallocated,
                            outlives_delete: ctx.generation.admit_index_outlives_delete,
                            skip_if_absent: ctx.generation.admit_index_skip_if_absent,
                            summable_off_count_index: ctx
                                .generation
                                .admit_index_summable_off_count_index,
                            range_countable_implies_countable: ctx
                                .generation
                                .admit_range_countable_implies_countable,
                            no_locking_resolution: ctx.generation.admit_index_no_locking_resolution,
                        },
                    )
                    .map_err(consensus_or_protocol_data_contract_error)?;

                    // INTEGER RANGE: the index grammar cannot see property
                    // types, so the source's key encoding is resolved here,
                    // on every parse, validating or not: the write walkers,
                    // the uniqueness probe and the query resolver all read
                    // it, and a contract loaded from state must bucket
                    // exactly as it did at registration. The source must be
                    // a user integer property of at most 64 bits; a system
                    // property is not in the flattened map, so it is refused
                    // here too.
                    if let Some(transform) = index.integer_range.as_mut() {
                        transform.key_type = flattened_document_properties
                            .get(transform.source.as_str())
                            .and_then(|property| {
                                IntegerRangeKeyType::from_property_type(&property.property_type)
                            })
                            .ok_or_else(|| {
                                consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "integerRange.on (\"{}\") must name an integer \
                                         property of the document type of at most 64 bits",
                                        transform.source
                                    )),
                                )
                            })?;
                    }

                    // `skipIfAbsent: true` skips on every optional property of
                    // the index. Optionality is a document-type fact the index
                    // parser cannot see, so the set is resolved here, on every
                    // parse (validating or not), before the index structure is
                    // built from it. System properties are never in a skip set;
                    // `apply_index_only` refuses the index when none remains.
                    // Nor is a value read through a reference: whether it can
                    // be absent depends on the referenced type, which this
                    // parse does not see, so only the array form skips on one.
                    if index.skip_if_absent && index.skip_if_absent_properties.is_empty() {
                        index.skip_if_absent_properties = index
                            .properties
                            .iter()
                            .filter(|property| {
                                !property.name.starts_with('$')
                                    && !required_fields.contains(&property.name)
                                    && !reads_through_reference(
                                        &property.name,
                                        flattened_document_properties,
                                    )
                            })
                            .map(|property| property.name.clone())
                            .collect();
                    }

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
                        if let Some(bucketing) = index.bucketing() {
                            // The overlap factor is the number of index
                            // entries a single document produces on this
                            // index — its write amplification — so its cap
                            // is a versioned system limit rather than a
                            // structural constant: retuning it is a
                            // protocol-version decision, not a code edit.
                            // Both kinds of grid share the one limit.
                            // `None` means a protocol version predating
                            // bucketed indexes, which cannot reach here
                            // because neither keyword parses there.
                            if let Some(max_overlap_factor) = ctx
                                .platform_version
                                .system_limits
                                .max_time_range_overlap_factor
                            {
                                let overlap = bucketing.overlap_factor();
                                if overlap > max_overlap_factor {
                                    return Err(consensus_or_protocol_data_contract_error(
                                        DataContractError::InvalidContractStructure(format!(
                                            "{} overlap factor (range / step = {}) exceeds \
                                             the maximum of {}; a smaller window or a larger \
                                             step is required to bound per-document index \
                                             entries",
                                            bucketing.keyword(),
                                            overlap,
                                            max_overlap_factor
                                        )),
                                    ));
                                }
                            }
                            // The grid-qualified level key is a GroveDB key,
                            // capped at 255 bytes like every other key; the
                            // grid suffix makes a long property path reach
                            // the cap sooner, so it is refused here instead
                            // of failing when the contract's trees are made.
                            let level_key = bucketing.storage_key(bucketing.source());
                            if level_key.len() > usize::from(MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH)
                            {
                                return Err(consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "the {} level of index \"{}\" encodes to {} bytes, \
                                         over the {}-byte key cap: shorten the property path \
                                         or the grid numbers",
                                        bucketing.keyword(),
                                        index.name,
                                        level_key.len(),
                                        MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH
                                    )),
                                ));
                            }
                        }
                        if let Some(transform) = &index.time_range {
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

                        // INTEGER RANGE: the checks that need the schema.
                        // (The overlap cap and level key cap ran above.)
                        if let Some(transform) = &index.integer_range {
                            // A required source keeps every document in the
                            // windows: an optional one would leave documents
                            // without the property under the null entry,
                            // which no window selection reads. A nested
                            // source is only always present when every
                            // object above it is required too.
                            if let Some(ancestor) =
                                unrequired_ancestor(&transform.source, required_fields)
                            {
                                return Err(consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "integerRange.on (\"{}\") sits inside \"{}\", which \
                                         is not listed in `required`: a valid document could \
                                         omit the whole object, leaving it in no window",
                                        transform.source, ancestor
                                    )),
                                ));
                            }
                            if !required_fields.contains(transform.source.as_str()) {
                                return Err(consensus_or_protocol_data_contract_error(
                                    DataContractError::InvalidContractStructure(format!(
                                        "integerRange.on (\"{}\") names a property the \
                                         document type does not require; add it to the \
                                         document type's required fields",
                                        transform.source
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
    // A `summableOffCountIndex` index keeps no entries and so has no member
    // key: it stays without a terminal.
    let mut indices = indices;
    if ctx.index_only {
        use crate::document::property_names::OWNER_ID;
        for index in indices.values_mut() {
            if index.terminal.is_none() && !index.is_summable_off_count_index() {
                index.terminal = Some(vec![OWNER_ID.to_string()]);
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
    //
    // Document type generations 1-2 (protocol versions 9-13) run this loop too, but only the
    // generation 3 index grammar admits `timeRange`, so every index they parse has
    // `time_range: None` and the loop changes nothing for them. Generation 0 (protocol
    // versions 1-8) parses its indices inline and never reaches this loop.
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

        // The moderation stamps, where the generation admits them: whether the type carries
        // them is `apply_moderator_abilities`'s to judge
        if ctx.generation.admit_moderation_stamp_indexes
            && MODERATION_STAMP_PROPERTIES.contains(&index_property.name.as_str())
        {
            return Ok(());
        }

        // A value read through a reference, where the generation admits them: the reference,
        // the index and the referenced field are judged once the whole type, and then the
        // whole contract, is parsed
        if ctx.generation.admit_derived_index_properties
            && reads_through_reference(&index_property.name, flattened_document_properties)
        {
            return Ok(());
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

            // The shape limits every indexed value carries as a grovedb key,
            // shared with an indexOnly index's terminal.
            check_indexable_property_shape(
                ctx.name,
                &index.name,
                &index_property.name,
                &property_definition.property_type,
            )
        } else {
            Ok(())
        }
    })
}

/// The shape checks a property must pass to be indexed, shared by the
/// prefix positions of an index, an indexOnly index's terminal and the field
/// a derived index property reads: the encoded value becomes a grovedb key, so
/// arrays and objects are refused and byte arrays and strings must be bounded
/// (grovedb caps keys at 255 bytes; the string bound is in characters, each at
/// most four bytes).
pub(super) fn check_indexable_property_shape(
    document_type_name: &str,
    index_name: &str,
    property_name: &str,
    property_type: &DocumentPropertyType,
) -> Result<(), ProtocolError> {
    match property_type {
        // Array and objects aren't supported for indexing yet. A typed array
        // is stored inline in the document, with no index entry per element
        // and no query operator (see Drive's `allowed_ops_for_type`).
        DocumentPropertyType::Array(_)
        | DocumentPropertyType::Object(_)
        | DocumentPropertyType::VariableTypeArray(_)
        | DocumentPropertyType::TypedArray(_) => Err(ProtocolError::ConsensusError(Box::new(
            InvalidIndexPropertyTypeError::new(
                document_type_name.to_owned(),
                index_name.to_owned(),
                property_name.to_owned(),
                property_type.name(),
            )
            .into(),
        ))),
        // Indexed byte array size must be limited
        DocumentPropertyType::ByteArray(sizes)
            if sizes
                .max_size
                .is_none_or(|max_size| max_size > MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH) =>
        {
            Err(ProtocolError::ConsensusError(Box::new(
                InvalidIndexedPropertyConstraintError::new(
                    document_type_name.to_owned(),
                    index_name.to_owned(),
                    property_name.to_owned(),
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
            if sizes
                .max_length
                .is_none_or(|max_length| max_length > MAX_INDEXED_STRING_PROPERTY_LENGTH) =>
        {
            Err(ProtocolError::ConsensusError(Box::new(
                InvalidIndexedPropertyConstraintError::new(
                    document_type_name.to_owned(),
                    index_name.to_owned(),
                    property_name.to_owned(),
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
    let token_costs_value = schema.get_optional_value(property_names::TOKEN_COST)?;

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
                    // Whether a transition may skip the token payment and have its signer pay
                    // the gas in credits instead. Only the v3 meta-schema admits the flag.
                    // Document type generations 1-2 (protocol versions 9-13) also run this
                    // parser, but their meta-schemas (v0-v2) set `additionalProperties: false`
                    // on `documentActionTokenCost`, so no contract they accept carries the key
                    // and this reads `false` for them, as before the flag existed. Generation 0
                    // (protocol versions 1-8) never reaches this parser.
                    let optional = action_cost
                        .get_optional_bool("optional")?
                        .unwrap_or_default();

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
                        optional,
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
    // deserialization uses the generation-2 parser which will read them.
    // Meta-schema v0 does not declare these fields: it admits them as unknown
    // keys of any shape, so a pre-v12 contract carrying one was never
    // validated against it, and reading it would assume a primary key tree
    // type the contract was not created with. No such contract exists on
    // mainnet or testnet; see `try_from_schema_generation_3` for the census
    // and the rule that follows from it.
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

/// Reads a doctype-level keyword holding a number of seconds (`ttl`) before the core parse
/// consumes `schema`. Its shape is enforced here and not left
/// to the meta-schema: a stored contract is read without one, and no doctype-level keyword
/// of this generation is read more leniently there.
pub(super) fn parse_seconds_keyword(
    schema: &Value,
    keyword: &str,
) -> Result<Option<u32>, ProtocolError> {
    // A schema that is not an object carries no keyword. Like every other
    // doctype-level keyword read before the core parser, this one must not be
    // the first to fail on such a schema: the core parser refuses it as an
    // invalid contract structure, and a raw value error here would replace
    // that refusal.
    let Ok(schema_map) = schema.to_map() else {
        return Ok(None);
    };

    Value::inner_optional_integer_value::<u32>(schema_map, keyword)
        .map_err(consensus_or_protocol_value_error)
}

/// What a document type's `moderatorAbilities` object says (protocol version
/// 14), read before the core parse consumes `schema`. The default is a type
/// its moderators can do nothing to.
#[derive(Debug, Default)]
pub(super) struct ModeratorAbilitiesKeyword {
    /// `delete`: the moderators may delete documents of the type.
    pub(super) delete: bool,
    /// `deleteWithin`: for how many seconds after a document's last
    /// modification they may.
    pub(super) delete_within: Option<u32>,
    /// `deleteKeepsRecord`: whether their deletion leaves a removal record,
    /// `None` when left out.
    pub(super) delete_keeps_record: Option<bool>,
    /// `deleteRefundsOwner`: whether the owner of a document they delete is
    /// refunded its storage, `None` when left out.
    pub(super) delete_refunds_owner: Option<bool>,
    /// `deleteSettled`: who must approve their deletion of a document past
    /// `deleteWithin`, its defaults filled in; the number is checked against its
    /// limit when applied.
    pub(super) delete_settled: Option<SettledDeletionRule>,
    /// `deleteKeepsFields`: the property paths whose values their removal
    /// record keeps.
    pub(super) delete_keeps_fields: BTreeSet<String>,
    /// `changeFields`: the top-level properties only they write.
    pub(super) change_fields: BTreeSet<String>,
}

/// Reads the doctype-level `moderatorAbilities` object. Its shape is enforced
/// here and not left to the meta-schema, as for every doctype-level keyword of
/// this generation: a stored contract is read without one. An object, with
/// only the keys `delete`, `deleteKeepsRecord` and `deleteRefundsOwner`
/// (booleans), `deleteWithin` (seconds, a u32), `deleteSettled` (an object with
/// only `leader` and `approversPredateDocument`, booleans, and `approvals`, a u16,
/// at least one of them),
/// `deleteKeepsFields` (a non-empty list of property paths) and `changeFields`
/// (a non-empty list of property names), saying something.
pub(super) fn parse_moderator_abilities_keyword(
    schema: &Value,
    name: &str,
) -> Result<ModeratorAbilitiesKeyword, ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };

    // A schema that is not an object carries no keyword: the core parser refuses it as an
    // invalid contract structure, and no error here may replace that refusal.
    let Ok(schema_map) = schema.to_map() else {
        return Ok(ModeratorAbilitiesKeyword::default());
    };
    let Some(abilities) = Value::get_optional_from_map(schema_map, MODERATOR_ABILITIES) else {
        return Ok(ModeratorAbilitiesKeyword::default());
    };
    let Ok(abilities_map) = abilities.to_map() else {
        return Err(structure_error(format!(
            "document type \"{name}\": `{MODERATOR_ABILITIES}` must be an object"
        )));
    };
    if let Some(unknown) = abilities_map
        .iter()
        .find_map(|(key, _)| match key.as_text() {
            Some(
                DELETE | DELETE_WITHIN | DELETE_KEEPS_RECORD | DELETE_REFUNDS_OWNER
                | DELETE_SETTLED | DELETE_KEEPS_FIELDS | CHANGE_FIELDS,
            ) => None,
            Some(other) => Some(other.to_string()),
            None => Some(key.to_string()),
        })
    {
        return Err(structure_error(format!(
            "document type \"{name}\": `{MODERATOR_ABILITIES}` has no key \"{unknown}\", only \
             `{DELETE}`, `{DELETE_WITHIN}`, `{DELETE_KEEPS_RECORD}`, `{DELETE_REFUNDS_OWNER}`, \
             `{DELETE_SETTLED}`, `{DELETE_KEEPS_FIELDS}` and `{CHANGE_FIELDS}`"
        )));
    }

    let delete = Value::inner_optional_bool_value(abilities_map, DELETE)
        .map_err(consensus_or_protocol_value_error)?
        .unwrap_or(false);
    let delete_within = Value::inner_optional_integer_value::<u32>(abilities_map, DELETE_WITHIN)
        .map_err(consensus_or_protocol_value_error)?;
    let delete_keeps_record = Value::inner_optional_bool_value(abilities_map, DELETE_KEEPS_RECORD)
        .map_err(consensus_or_protocol_value_error)?;
    let delete_refunds_owner =
        Value::inner_optional_bool_value(abilities_map, DELETE_REFUNDS_OWNER)
            .map_err(consensus_or_protocol_value_error)?;
    let delete_settled = match Value::get_optional_from_map(abilities_map, DELETE_SETTLED) {
        None => None,
        Some(settled) => {
            let Ok(settled_map) = settled.to_map() else {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{MODERATOR_ABILITIES}.{DELETE_SETTLED}` must be \
                     an object"
                )));
            };
            if let Some(unknown) = settled_map.iter().find_map(|(key, _)| match key.as_text() {
                Some(
                    delete_settled::LEADER
                    | delete_settled::APPROVALS
                    | delete_settled::APPROVERS_PREDATE_DOCUMENT,
                ) => None,
                Some(other) => Some(other.to_string()),
                None => Some(key.to_string()),
            }) {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{MODERATOR_ABILITIES}.{DELETE_SETTLED}` has no \
                     key \"{unknown}\", only `{}`, `{}` and `{}`",
                    delete_settled::LEADER,
                    delete_settled::APPROVALS,
                    delete_settled::APPROVERS_PREDATE_DOCUMENT,
                )));
            }
            if settled_map.is_empty() {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{MODERATOR_ABILITIES}.{DELETE_SETTLED}` is empty: \
                     say whether the team's leader must approve (`{}`), how many must (`{}`), \
                     whether members the leader added after a document approve its deletion \
                     (`{}`), or any of them",
                    delete_settled::LEADER,
                    delete_settled::APPROVALS,
                    delete_settled::APPROVERS_PREDATE_DOCUMENT,
                )));
            }
            let leader = Value::inner_optional_bool_value(settled_map, delete_settled::LEADER)
                .map_err(consensus_or_protocol_value_error)?
                .unwrap_or(false);
            let approvals =
                Value::inner_optional_integer_value::<u16>(settled_map, delete_settled::APPROVALS)
                    .map_err(consensus_or_protocol_value_error)?
                    .unwrap_or(1);
            let approvers_predate_document = Value::inner_optional_bool_value(
                settled_map,
                delete_settled::APPROVERS_PREDATE_DOCUMENT,
            )
            .map_err(consensus_or_protocol_value_error)?
            // On by default only where it protects something: a rule one approval meets, the
            // leader meets alone, so the members it adds give it nothing.
            .unwrap_or(approvals > 1);
            Some(SettledDeletionRule {
                leader,
                approvals,
                approvers_predate_document,
            })
        }
    };
    let delete_keeps_fields = match Value::get_optional_from_map(abilities_map, DELETE_KEEPS_FIELDS)
    {
        None => BTreeSet::new(),
        Some(Value::Array(entries)) => {
            let fields = entries
                .iter()
                .map(|entry| {
                    entry.as_text().map(str::to_owned).ok_or_else(|| {
                        structure_error(format!(
                            "document type \"{name}\": every \
                             `{MODERATOR_ABILITIES}.{DELETE_KEEPS_FIELDS}` entry must be a \
                             property path (a string)"
                        ))
                    })
                })
                .collect::<Result<BTreeSet<_>, _>>()?;
            if fields.is_empty() {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{MODERATOR_ABILITIES}.{DELETE_KEEPS_FIELDS}` \
                     lists no property (leave it out for a type whose removal records keep \
                     none)"
                )));
            }
            fields
        }
        Some(_) => {
            return Err(structure_error(format!(
                "document type \"{name}\": `{MODERATOR_ABILITIES}.{DELETE_KEEPS_FIELDS}` must \
                 be an array of property paths"
            )));
        }
    };
    let change_fields = match Value::get_optional_from_map(abilities_map, CHANGE_FIELDS) {
        None => BTreeSet::new(),
        Some(_) => {
            let fields = parse_property_name_list_keyword(abilities, name, CHANGE_FIELDS)?;
            if fields.is_empty() {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{MODERATOR_ABILITIES}.{CHANGE_FIELDS}` lists no \
                     property (leave it out for a type whose moderators change nothing)"
                )));
            }
            fields
        }
    };
    if abilities_map.is_empty() {
        return Err(structure_error(format!(
            "document type \"{name}\": `{MODERATOR_ABILITIES}` is empty (leave it out for a \
             type its moderators can do nothing to)"
        )));
    }

    Ok(ModeratorAbilitiesKeyword {
        delete,
        delete_within,
        delete_keeps_record,
        delete_refunds_owner,
        delete_settled,
        delete_keeps_fields,
        change_fields,
    })
}

/// Applies the `moderatorAbilities` object and checks what each of its keys
/// requires. Anything it grants needs a contract that declares moderation, or
/// there would be nobody to use it (moderation can only be declared when the
/// contract is created).
///
/// `delete` lets the contract's moderators delete documents of the type, so:
/// - the type must not keep history: Drive refuses to delete such documents;
/// - the type must not be indexOnly: such a document has no stored row a
///   moderator could name by id;
/// - the type must not restrict creation: its documents are the contract
///   owner's, which no moderator may delete;
/// - the type must have no contested index: a document a moderator deleted is
///   restored by an ordinary insert, and a contested index only takes a
///   document through a vote.
///
/// `deleteWithin` limits how long after a document's last modification the
/// moderators may delete it, so:
/// - the type must let moderators delete its documents at all, or the window
///   would limit nothing;
/// - the type must require the clock the window is measured on. That is
///   `$updatedAt`, set at creation and moved by every replace, and where a type
///   does not carry it, `$createdAt`. A type whose documents can be replaced
///   must require `$updatedAt`: measured from creation alone, its author could
///   wait the window out and then rewrite the document into something no
///   moderator can remove any more. A type whose documents never change has
///   no modification after the creation, so `$createdAt` says as much;
/// - it lasts at least a second: a window of none would be a type moderators
///   can never delete from, which is said by leaving `delete` out.
///
/// `deleteKeepsRecord` (default `true`) and `deleteRefundsOwner` (default
/// `false`) say what a deletion leaves: a removal record under the contract,
/// which is also what a restore brings the document back from, and the owner's
/// storage refund, forfeited unless the type gives it back. Each describes the
/// moderators' deletion, so each needs `delete: true`.
///
/// `deleteSettled` says who must approve a deletion once the window has passed,
/// so:
/// - the type must set `deleteWithin`: without a window no document is ever
///   settled;
/// - the contract's moderators must be an elected team: the rule names the
///   seated team's leader and counts its members, and the owner and appointed
///   moderators of the other kinds are neither;
/// - `approvals` is at least 1 and, under full validation, at most the members
///   the declared team can hold: its leader,
///   `SystemLimits::max_moderation_charter_elected_members` elected members and
///   the declaration's `maxAddedModerators`;
/// - while `approversPredateDocument` is on (the default when `approvals` is
///   above 1), the type must list `$createdAt` in `required` under full
///   validation: a member the leader added approves only the deletion of
///   documents created after its addition, and a document without `$createdAt`
///   says nothing of when it was. A registration rule, as the bound on
///   `approvals` is: a stored type is read back without it, and a document of it
///   without `$createdAt` admits no added member.
///
/// `deleteKeepsFields` names what of a deleted document its removal record
/// keeps, copied from the document as it was deleted: what stays public once
/// it is gone. It needs `delete: true` and a record to keep them in, so it is
/// refused beside `deleteKeepsRecord: false`. Each entry is either:
/// - the path of a declared property at any depth (`hashtag`, `meta.tags`, or
///   an object, kept whole), not transient (no stored document holds it) and
///   not inside another listed path (which keeps it already); or
/// - one of the timestamps and block heights a document carries (a
///   [`SystemProperty`]), listed in `required`, without which no document
///   carries it. `$id` and `$ownerId` are in every record already.
///
/// `changeFields` names the properties only the moderators write. A
/// moderator's change is stored as an update that touches nothing else, and is
/// not checked against the type's references, so each property must be:
/// - declared at the top level of the type: a change sets whole top-level
///   values, as a replace compares them;
/// - optional: a document's owner can not set it when creating the document,
///   so it starts absent;
/// - stored, so not transient;
/// - not listed under `immutable`, with a condition or without, which would
///   freeze what the moderators are to change;
/// - neither a reference nor read by one (a `where` value, a `findBy` source
///   or function param, a key id): a reference is checked when the document is written by its
///   owner, and a moderator's change must leave every reference as it was
///   checked;
/// - neither generated (`generatedFrom`) nor read by a generated property,
///   whose value the change would leave stale;
/// - in no contested index, whose values are awarded by a vote.
///
/// The type must not be indexOnly either: there is no stored row to change.
///
/// The rules hold for every contract that could be stored (the keyword and
/// the moderation config both arrive with protocol version 14), so they are
/// not skipped when a stored contract is read back. Runs after
/// `apply_index_only`, `apply_immutable_fields` and the parse of the
/// references and generated properties, which it reads.
pub(super) fn apply_moderator_abilities(
    document_type: &mut DocumentTypeV2,
    abilities: ModeratorAbilitiesKeyword,
    data_contract_config: &DataContractConfig,
    name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let ModeratorAbilitiesKeyword {
        delete,
        delete_within,
        delete_keeps_record,
        delete_refunds_owner,
        delete_settled,
        delete_keeps_fields,
        change_fields,
    } = abilities;
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    // Only a type keeping fields for its moderators has its documents stamped, so only such a
    // type indexes the stamp. Never in a unique index: the stamp is the moderator's and the
    // time, and a moderator's change refused because another document holds the same would
    // make no sense.
    for (index_name, index) in &document_type.indices {
        let Some(stamp) = index
            .properties
            .iter()
            .map(|property| property.name.as_str())
            .find(|property| MODERATION_STAMP_PROPERTIES.contains(property))
        else {
            continue;
        };
        if change_fields.is_empty() {
            return Err(structure_error(format!(
                "index \"{index_name}\" of document type \"{name}\" reads `{stamp}`, which only \
                 the documents of a type listing `{MODERATOR_ABILITIES}.{CHANGE_FIELDS}` carry",
            )));
        }
        if index.unique {
            return Err(structure_error(format!(
                "unique index \"{index_name}\" of document type \"{name}\" reads `{stamp}`: a \
                 moderator's change would be refused because another document holds the same \
                 stamp",
            )));
        }
    }
    if !delete
        && delete_within.is_none()
        && delete_keeps_record.is_none()
        && delete_refunds_owner.is_none()
        && delete_settled.is_none()
        && delete_keeps_fields.is_empty()
        && change_fields.is_empty()
    {
        return Ok(());
    }

    if data_contract_config.moderation().is_none() {
        return Err(structure_error(format!(
            "document type \"{name}\" sets `{MODERATOR_ABILITIES}`, but the contract declares \
             no `moderation` in its config, so there are no moderators to use them \
             (moderation can only be declared when the contract is created)",
        )));
    }
    let has_contested_index = document_type
        .indices
        .values()
        .any(|index| index.contested_index.is_some());

    if delete {
        if document_type.documents_keep_history {
            return Err(structure_error(format!(
                "document type \"{name}\" sets both `documentsKeepHistory: true` and \
                 `{MODERATOR_ABILITIES}.{DELETE}: true`, but the storage layer refuses to \
                 delete a document whose type keeps history",
            )));
        }
        if document_type.index_only {
            return Err(structure_error(format!(
                "indexOnly document type \"{name}\" must not set \
                 `{MODERATOR_ABILITIES}.{DELETE}`: there is no stored row a moderator could \
                 name by id",
            )));
        }
        if document_type.creation_restriction_mode != CreationRestrictionMode::NoRestrictions {
            return Err(structure_error(format!(
                "document type \"{name}\" restricts document creation and must not set \
                 `{MODERATOR_ABILITIES}.{DELETE}`: its documents belong to the contract owner, \
                 whose documents no moderator may delete",
            )));
        }
        // A moderator's restore puts the document back through the ordinary insert, unique
        // indexes checked; a contested index awards its value by a vote, which no restore can
        // go through, so such a type would have deletions that can not be undone.
        if has_contested_index {
            return Err(structure_error(format!(
                "document type \"{name}\" has a contested index and must not set \
                 `{MODERATOR_ABILITIES}.{DELETE}`: a document a moderator deleted is restored by \
                 an ordinary insert, and a contested index only takes a document through a vote",
            )));
        }
        document_type.documents_can_be_deleted_by_moderators = true;
        document_type.moderator_deletions_keep_records = delete_keeps_record.unwrap_or(true);
        document_type.moderator_deletions_refund_owner = delete_refunds_owner.unwrap_or(false);
    }

    // What a deletion leaves is only said of a type moderators delete from
    for (key, given) in [
        (DELETE_KEEPS_RECORD, delete_keeps_record.is_some()),
        (DELETE_REFUNDS_OWNER, delete_refunds_owner.is_some()),
        (DELETE_KEEPS_FIELDS, !delete_keeps_fields.is_empty()),
    ] {
        if given && !delete {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{key}`, which says what a \
                 moderator's deletion leaves and means nothing without `{DELETE}: true`",
            )));
        }
    }

    if !delete_keeps_fields.is_empty() {
        apply_delete_keeps_fields(document_type, delete_keeps_fields, name)?;
    }

    if let Some(seconds) = delete_within {
        if !delete {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_WITHIN}`, which \
                 limits `{DELETE}: true` and means nothing without it",
            )));
        }
        if seconds == 0 {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_WITHIN}: 0`: a \
                 window lasts at least one second (leave `{DELETE}` out for a type moderators \
                 can not delete from)",
            )));
        }
        let requires_updated_at = document_type.required_fields.contains(UPDATED_AT);
        if document_type.documents_mutable && !requires_updated_at {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_WITHIN}`, which is \
                 measured from a document's last modification, and its documents can be \
                 replaced: list `$updatedAt` in `required`",
            )));
        }
        if !requires_updated_at && !document_type.required_fields.contains(CREATED_AT) {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_WITHIN}`, which is \
                 measured from a document's last modification: list `$updatedAt`, or \
                 `$createdAt` for documents that never change, in `required`",
            )));
        }
        document_type.documents_can_be_deleted_by_moderators_for = Some(seconds);
    }

    if let Some(rule) = delete_settled {
        if delete_within.is_none() {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_SETTLED}`, which \
                 rules deletions past `{DELETE_WITHIN}` and means nothing without it",
            )));
        }
        // The moderation config was checked present above
        let Some(elected) = data_contract_config
            .moderation()
            .and_then(|moderation| moderation.moderators.elected())
        else {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_SETTLED}`, which \
                 names the leader and the members of an elected moderation team, but the \
                 contract's moderators are not elected",
            )));
        };
        // The most members the declared team can hold: its leader, the members a charter
        // elects and those the declaration lets the leader add. A registration limit, like the
        // others: a stored contract was checked when it was registered, and the charter's
        // bound changing later must not make it unreadable.
        let max_approvals = 1u16
            .saturating_add(
                platform_version
                    .system_limits
                    .max_moderation_charter_elected_members,
            )
            .saturating_add(elected.max_added_moderators);
        if rule.approvals == 0 || (full_validation && rule.approvals > max_approvals) {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_SETTLED}.{}: {}`, \
                 which must be between 1 and {max_approvals}, the most members the declared \
                 team can hold",
                delete_settled::APPROVALS,
                rule.approvals,
            )));
        }
        if full_validation
            && rule.approvers_predate_document
            && !document_type.required_fields.contains(CREATED_AT)
        {
            return Err(structure_error(format!(
                "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_SETTLED}`, whose \
                 approvers the leader added must have been added before the document was created \
                 (`{}`, on by default when more than one approval is needed), which is read from \
                 `$createdAt`: list `$createdAt` in `required`, or set `{}: false`",
                delete_settled::APPROVERS_PREDATE_DOCUMENT,
                delete_settled::APPROVERS_PREDATE_DOCUMENT,
            )));
        }
        document_type.moderator_settled_deletion = Some(rule);
    }

    if change_fields.is_empty() {
        return Ok(());
    }
    if document_type.index_only {
        return Err(structure_error(format!(
            "indexOnly document type \"{name}\" must not set \
             `{MODERATOR_ABILITIES}.{CHANGE_FIELDS}`: there is no stored row a moderator could \
             change",
        )));
    }
    // A transient value is dropped before the document is stored, and a moderator's change is
    // judged against the schema: a required transient property would be missing from every
    // stored document, and no moderator can supply it.
    if let Some((path, _)) = document_type
        .flattened_properties
        .iter()
        .find(|(_, property)| {
            property.transient && (property.required || property.required_since.is_some())
        })
    {
        return Err(structure_error(format!(
            "document type \"{name}\" requires the transient property \"{path}\" and sets \
             `{MODERATOR_ABILITIES}.{CHANGE_FIELDS}`: a transient value is never stored, so \
             every moderator's change, judged against the schema without it, would be refused",
        )));
    }
    let read_by_references = properties_read_by_references(DocumentTypeRef::V2(document_type));
    let read_by_generated = document_type
        .generated_from_fields
        .iter()
        .flat_map(|(path, generated_from)| {
            std::iter::once(path.as_str()).chain(generated_from.property_params())
        })
        .map(top_level_property)
        .collect::<BTreeSet<_>>();
    for field in &change_fields {
        let refusal = if !document_type.properties.contains_key(field) {
            let hint = if field.contains('.') && !field.starts_with('$') {
                ": a moderator sets whole top-level values, so list the object around a nested \
                 property"
            } else {
                ""
            };
            Some(format!(
                "it is not a top-level property of the document type{hint}"
            ))
        } else if document_type.required_fields.contains(field) {
            Some(
                "it is required, and a document's owner can not set it, so it must start absent"
                    .to_string(),
            )
        } else if document_type.transient_fields.contains(field) {
            Some("it is transient, so no stored document holds it".to_string())
        } else if document_type.immutable_fields.contains(field) {
            Some("it is listed under `immutable`, which would freeze it".to_string())
        } else if document_type.immutable_field_conditions.contains_key(field) {
            Some(
                "it is listed under `immutable` with a condition, which would freeze it while \
                 the condition holds"
                    .to_string(),
            )
        } else if read_by_references.contains(field.as_str()) {
            Some(
                "it holds a reference or is read by one, and a moderator's change leaves every \
                 reference as it was checked"
                    .to_string(),
            )
        } else if read_by_generated.contains(field.as_str()) {
            Some(
                "it is generated or read by a generated property, which a moderator's change \
                 would leave stale"
                    .to_string(),
            )
        } else if document_type.indices.values().any(|index| {
            index.contested_index.is_some()
                && index
                    .properties
                    .iter()
                    .any(|property| top_level_property(&property.name) == field)
        }) {
            Some("it is in a contested index, whose values are awarded by a vote".to_string())
        } else {
            None
        };
        if let Some(refusal) = refusal {
            return Err(structure_error(format!(
                "document type \"{name}\" can not list \"{field}\" under \
                 `{MODERATOR_ABILITIES}.{CHANGE_FIELDS}`: {refusal}",
            )));
        }
    }
    document_type.moderator_changeable_fields = change_fields;
    Ok(())
}

/// Checks and applies `moderatorAbilities.deleteKeepsFields` on a type whose
/// moderators delete (see [`apply_moderator_abilities`] for the rules).
fn apply_delete_keeps_fields(
    document_type: &mut DocumentTypeV2,
    kept_fields: BTreeSet<String>,
    name: &str,
) -> Result<(), ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    if !document_type.moderator_deletions_keep_records {
        return Err(structure_error(format!(
            "document type \"{name}\" sets `{MODERATOR_ABILITIES}.{DELETE_KEEPS_FIELDS}` and \
             `{DELETE_KEEPS_RECORD}: false`: the values are kept in the removal record, which \
             its moderators' deletions do not leave",
        )));
    }
    for field in &kept_fields {
        let refusal = if [ID, OWNER_ID].contains(&field.as_str()) {
            Some("every removal record holds it already".to_string())
        } else if field.starts_with('$') {
            if SystemProperty::from_name(field).is_none() {
                Some(format!(
                    "the system properties a record keeps are {}",
                    SystemProperty::ALL.map(SystemProperty::name).join(", ")
                ))
            } else if !document_type.required_fields.contains(field) {
                Some("it is not listed in `required`, so no document carries it".to_string())
            } else {
                None
            }
        } else if let Some(outer) = kept_fields.iter().find(|outer| {
            field
                .strip_prefix(outer.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
        }) {
            Some(format!(
                "it is inside \"{outer}\", which the record keeps whole"
            ))
        } else if property_at_path(&document_type.properties, field).is_none() {
            Some("it is not a declared property of the document type".to_string())
        } else if is_transient(DocumentTypeRef::V2(document_type), field) {
            Some("it is transient, so no stored document holds it".to_string())
        } else {
            None
        };
        if let Some(refusal) = refusal {
            return Err(structure_error(format!(
                "document type \"{name}\" can not list \"{field}\" under \
                 `{MODERATOR_ABILITIES}.{DELETE_KEEPS_FIELDS}`: {refusal}",
            )));
        }
    }
    document_type.moderator_deletion_kept_fields = kept_fields;
    Ok(())
}

/// The top-level properties of `document_type` that a reference declared on it
/// reads when a document is written: each property holding a reference, and
/// the referring side of every `where`, `findBy` source (a function's params
/// included) and key id property, for every leaf of every reference expression, the
/// `ownerRefersTo` and `creatorRefersTo` declarations included. System
/// properties (`$ownerId`) are left out: they are no property a moderator can
/// name.
fn properties_read_by_references(document_type: DocumentTypeRef<'_>) -> BTreeSet<&str> {
    let mut read = BTreeSet::new();
    for (holder, reference) in document_type.reference_declarations() {
        if let ReferenceHolder::Property(path) = holder {
            read.insert(top_level_property(path));
        }
        // A key id property reads the identity property whose key it names
        if let PropertyReference::KeyId(KeyIdReference {
            identity_property: KeyReferenceIdentityProperty::Property(path),
            ..
        }) = reference
        {
            read.insert(top_level_property(path));
        }
        let Some(target) = reference.target() else {
            continue;
        };
        for leaf in target.leaves() {
            let agreement = match leaf {
                DocumentPropertyReferenceTarget::PermanentDocument {
                    property_agreement, ..
                }
                | DocumentPropertyReferenceTarget::DeletableDocument {
                    property_agreement, ..
                }
                | DocumentPropertyReferenceTarget::ModeratedDocument {
                    property_agreement, ..
                } => Some(property_agreement),
                DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                    property_agreement,
                    lookup,
                    ..
                }
                | DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                    property_agreement,
                    lookup,
                    ..
                } => {
                    read.extend(lookup.referring_properties().map(top_level_property));
                    // A computed key's params: the key is judged on the create only, so a
                    // change would leave the document no longer hashing to what it revealed
                    if let Some((_, key)) = lookup.hash_key() {
                        read.extend(key.properties_read().map(top_level_property));
                    }
                    Some(property_agreement)
                }
                DocumentPropertyReferenceTarget::ListElement(list_element) => {
                    Some(&list_element.property_agreement)
                }
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property, ..
                } => {
                    read.insert(top_level_property(key_id_property));
                    None
                }
                DocumentPropertyReferenceTarget::Identity
                | DocumentPropertyReferenceTarget::Contract { .. }
                | DocumentPropertyReferenceTarget::Token
                | DocumentPropertyReferenceTarget::AnyOf(_)
                | DocumentPropertyReferenceTarget::AllOf(_) => None,
            };
            if let Some(agreement) = agreement {
                read.extend(
                    agreement
                        .keys()
                        .filter(|property| !property.starts_with('$'))
                        .map(|property| top_level_property(property)),
                );
            }
        }
    }
    read
}

/// Applies the `ttl` keyword and checks what it requires.
///
/// The platform deletes every document of the type once `$createdAt` plus `ttl`
/// seconds has passed, finding it through the expirations tree entry written when the
/// document was created, so:
/// - the type must require `$createdAt`: a document's expiry is computed from it when it
///   is written, replaced and deleted, and it is set from block time, so nothing the
///   writer sends moves it;
/// - the type must not keep history: the storage layer refuses to delete a document whose
///   type keeps history;
/// - the type must not be indexOnly: such a document has no stored row to delete by id;
/// - the type must not have a contested index: a contested document waits in its vote
///   poll, outside the documents tree, until the poll awards it, keeping its `$createdAt`
///   from the create, so it could expire before it exists;
/// - the time to live is at least a second, and under full validation (a contract being
///   registered or updated) at least `min_document_ttl_seconds` and at most
///   `max_document_ttl_seconds`. The floor keeps a document in state well past the moment
///   its writer fetches the proof of its create, which proves it present.
///
/// What may point at the type follows from `documents_can_disappear`: a `permanentDocument`
/// reference, one with `findBy` or `inList` included, may not target it; a `deletableDocument`
/// reference may, one found by `findBy` included.
///
/// The rules other than the bounds hold for every contract that could be stored (the keyword
/// arrives with protocol version 14), so they are not skipped when a stored contract is
/// read back. Runs after `apply_index_only`, whose flag it reads.
pub(super) fn apply_documents_ttl(
    document_type: &mut DocumentTypeV2,
    ttl_seconds: Option<u32>,
    name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let Some(seconds) = ttl_seconds else {
        return Ok(());
    };
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };

    if seconds == 0 {
        return Err(structure_error(format!(
            "document type \"{}\" sets `ttl: 0`: a time to live lasts at least one second \
             (leave `ttl` out for documents that live until someone deletes them)",
            name,
        )));
    }
    if full_validation {
        if let Some(min_seconds) = platform_version.system_limits.min_document_ttl_seconds {
            if seconds < min_seconds {
                return Err(structure_error(format!(
                    "document type \"{}\" sets `ttl: {}`, below the shortest time to live a \
                     document type may declare, {} seconds",
                    name, seconds, min_seconds,
                )));
            }
        }
        if let Some(max_seconds) = platform_version.system_limits.max_document_ttl_seconds {
            if seconds > max_seconds {
                return Err(structure_error(format!(
                    "document type \"{}\" sets `ttl: {}`, above the longest time to live a \
                     document type may declare, {} seconds",
                    name, seconds, max_seconds,
                )));
            }
        }
    }
    if !document_type.required_fields.contains(CREATED_AT) {
        return Err(structure_error(format!(
            "document type \"{}\" sets `ttl`, which is counted from a document's creation: \
             list `$createdAt` in `required`",
            name,
        )));
    }
    if document_type.documents_keep_history {
        return Err(structure_error(format!(
            "document type \"{}\" sets both `documentsKeepHistory: true` and `ttl`, but the \
             storage layer refuses to delete a document whose type keeps history",
            name,
        )));
    }
    if document_type.index_only {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must not set `ttl`: there is no stored row the \
             platform could delete by id",
            name,
        )));
    }
    if document_type
        .indices
        .values()
        .any(|index| index.contested_index.is_some())
    {
        return Err(structure_error(format!(
            "document type \"{}\" has a contested index and must not set `ttl`: a contested \
             document waits in its vote poll until the poll awards it, and could expire \
             before it is stored",
            name,
        )));
    }

    document_type.documents_ttl_seconds = Some(seconds);
    Ok(())
}

/// Whether a document type schema sets `canBeDeleted: "onlyWhenConsumed"`, read before the
/// core parser consumes `schema`, through the map accessor every doctype keyword is read with
/// (the first entry under the key), same shape as [`parse_index_only_keyword`]. Only the
/// generation-3 driver calls this: it passes the answer to the core parse, which then reads
/// the owner's delete as `false`, and records it on the parsed type
/// ([`apply_deleted_only_when_consumed`]). Any other value is left to the core parse, which
/// reads `canBeDeleted` as a boolean.
pub(super) fn parse_can_be_deleted_only_when_consumed_keyword(schema: &Value) -> bool {
    schema.to_map().ok().is_some_and(|schema_map| {
        Value::get_optional_from_map(schema_map, CAN_BE_DELETED).and_then(Value::as_text)
            == Some(CAN_BE_DELETED_ONLY_WHEN_CONSUMED)
    })
}

/// Records `canBeDeleted: "onlyWhenConsumed"` on a generation-3 type: its owner can not delete
/// a document (the core parse read `documents_can_be_deleted` as `false`), and a create whose
/// `refersTo` declares `consume` can. Its documents can then leave state, so the type is a
/// `deletableDocument` target. Runs after `apply_index_only`, and refuses, on every parse as
/// `ttl` does, a type whose documents nothing could consume: one that keeps history, which
/// the storage layer never deletes from, and an indexOnly one, which has no stored row a
/// reference finds.
pub(super) fn apply_deleted_only_when_consumed(
    document_type: &mut DocumentTypeV2,
    deleted_only_when_consumed: bool,
    name: &str,
) -> Result<(), ProtocolError> {
    if !deleted_only_when_consumed {
        return Ok(());
    }
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    if document_type.documents_keep_history {
        return Err(structure_error(format!(
            "document type \"{name}\" sets both `documentsKeepHistory: true` and \
             `canBeDeleted: \"onlyWhenConsumed\"`, but the storage layer refuses to delete a \
             document whose type keeps history, so nothing could consume one: set \
             `canBeDeleted` to false",
        )));
    }
    if document_type.index_only {
        return Err(structure_error(format!(
            "indexOnly document type \"{name}\" must not set `canBeDeleted: \"onlyWhenConsumed\"`: \
             there is no stored row a reference could find and consume",
        )));
    }
    document_type.documents_deleted_only_when_consumed = true;
    Ok(())
}

/// Reads a doctype-level array of top-level property names (`entryPayload`,
/// or `changeFields` inside `moderatorAbilities`) before the core parse
/// consumes `schema`, same shape as [`parse_index_only_keyword`]. Only the
/// generation-3 driver calls this; earlier generations ignore such keywords
/// exactly as they ignore every doctype-level keyword they predate (their
/// meta-schemas still reject them under `full_validation`).
///
/// Every entry must be a string, on either path. A non-string entry is refused
/// rather than silently dropped: dropping it would record a smaller set than
/// the author declared. A contract admitted under meta-schema v0 was never
/// checked against this keyword; `try_from_schema_generation_3` states why the
/// stored path is strict all the same.
pub(super) fn parse_property_name_list_keyword(
    schema: &Value,
    name: &str,
    keyword: &str,
) -> Result<BTreeSet<String>, ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };

    let Ok(schema_map) = schema.to_map() else {
        return Ok(BTreeSet::new());
    };
    let Some(value) = Value::get_optional_from_map(schema_map, keyword) else {
        return Ok(BTreeSet::new());
    };
    let Value::Array(entries) = value else {
        return Err(structure_error(format!(
            "document type \"{name}\": `{keyword}` must be an array of top-level property names"
        )));
    };

    entries
        .iter()
        .map(|entry| {
            entry.as_text().map(str::to_owned).ok_or_else(|| {
                structure_error(format!(
                    "document type \"{name}\": every `{keyword}` entry must be a property name \
                     (a string)"
                ))
            })
        })
        .collect()
}

/// What the `immutable` keyword lists (protocol version 14), read before the
/// core parse consumes `schema`.
#[derive(Debug, Default)]
pub(super) struct ImmutableKeyword {
    /// The properties listed by name, frozen at document creation.
    pub(super) always: BTreeSet<String>,
    /// The properties listed with a condition, each with its `when` as
    /// declared: its paths are judged against the parsed document type, so it
    /// is parsed by [`apply_immutable_fields`].
    pub(super) conditional: BTreeMap<String, Value>,
}

/// Reads the doctype-level `immutable` keyword: an array whose entries are a
/// top-level property name, frozen at document creation, or an object with
/// exactly `property`, a property name, and `when`, the condition under which
/// it is frozen. A property may be listed once. Its shape is enforced on both
/// paths, as every doctype-level keyword of this generation's is: an entry of
/// another shape is refused rather than dropped, which would record less than
/// the author declared.
///
/// `immutableAllowSetting`, which listed the immutable properties a replace
/// could still set while absent, is refused on every parse, naming the
/// conditional entry that says it now, so that no contract written with it
/// loads with another meaning.
pub(super) fn parse_immutable_keyword(
    schema: &Value,
    name: &str,
) -> Result<ImmutableKeyword, ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    let keyword = property_names::IMMUTABLE;
    let property_key = property_names::immutable_entry::PROPERTY;
    let when_key = property_names::immutable_entry::WHEN;

    let Ok(schema_map) = schema.to_map() else {
        return Ok(ImmutableKeyword::default());
    };
    if Value::get_optional_from_map(schema_map, property_names::IMMUTABLE_ALLOW_SETTING).is_some() {
        return Err(structure_error(format!(
            "document type \"{name}\": `{}` is replaced by a conditional `{keyword}` entry: \
             {{ \"{property_key}\": \"p\", \"{when_key}\": {{ \"present\": \"$old.p\" }} }} \
             freezes \"p\" once the stored document holds it",
            property_names::IMMUTABLE_ALLOW_SETTING
        )));
    }
    let Some(value) = Value::get_optional_from_map(schema_map, keyword) else {
        return Ok(ImmutableKeyword::default());
    };
    let Value::Array(entries) = value else {
        return Err(structure_error(format!(
            "document type \"{name}\": `{keyword}` must be an array of property names and \
             {{ \"{property_key}\", \"{when_key}\" }} objects"
        )));
    };

    let mut listed = ImmutableKeyword::default();
    for entry in entries {
        let (property, when) = match entry {
            Value::Text(property) => (property.clone(), None),
            Value::Map(members) => {
                let mut property = None;
                let mut when = None;
                for (key, member) in members {
                    match key.as_text() {
                        Some(key) if key == property_key => property = member.as_text(),
                        Some(key) if key == when_key => when = Some(member),
                        _ => {
                            return Err(structure_error(format!(
                                "document type \"{name}\": an `{keyword}` entry object holds \
                                 exactly \"{property_key}\" and \"{when_key}\", not \"{}\"",
                                key.non_qualified_string_representation()
                            )))
                        }
                    }
                }
                let (Some(property), Some(when)) = (property, when) else {
                    return Err(structure_error(format!(
                        "document type \"{name}\": an `{keyword}` entry object needs \
                         \"{property_key}\", a property name, and \"{when_key}\", the \
                         condition under which it is frozen"
                    )));
                };
                (property.to_string(), Some(when.clone()))
            }
            _ => {
                return Err(structure_error(format!(
                    "document type \"{name}\": every `{keyword}` entry must be a property name \
                     (a string) or a {{ \"{property_key}\", \"{when_key}\" }} object"
                )))
            }
        };
        if listed.always.contains(&property) || listed.conditional.contains_key(&property) {
            return Err(structure_error(format!(
                "document type \"{name}\" lists \"{property}\" under `{keyword}` twice"
            )));
        }
        match when {
            None => {
                listed.always.insert(property);
            }
            Some(when) => {
                listed.conditional.insert(property, when);
            }
        }
    }
    Ok(listed)
}

/// Writes what the `immutable` keyword lists onto the parsed document type:
/// the properties frozen at creation, and the others with their parsed
/// conditions. Runs after the core parse, whose properties it reads.
///
/// A condition is parsed by [`parse_replace_condition`]: the grammar of a
/// `propertyConstraints` rule, its reads checked as a rule's are on every
/// parse (a stored condition reading a property the type does not declare, or
/// a transient one, could only ever read it as absent), the stored document
/// read through `$old.`, and no `countOf` or `sumOf`.
///
/// Under full validation, for contracts entering the chain, the lints: the
/// type's documents must be mutable; every listed property is a declared
/// top-level property, neither a system property, which the platform manages,
/// nor transient, never stored, so frozen at "absent" it could never be written
/// (a nested path is refused with a hint to list the containing object, which
/// freezes it whole: the replace compares top-level values); and a condition
/// stays within the node limit of a rule and lists no condition twice. They are
/// schema lints rather than storage-layout invariants, so stored contracts
/// bypass them, which keeps a later tightening from ever making a committed
/// contract unreadable.
pub(super) fn apply_immutable_fields(
    document_type: &mut DocumentTypeV2,
    listed: ImmutableKeyword,
    schema_defs: Option<&BTreeMap<String, Value>>,
    name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    let ImmutableKeyword {
        always,
        conditional,
    } = listed;

    let mut conditions = BTreeMap::new();
    for (property, when) in conditional {
        let condition = parse_replace_condition(
            document_type,
            &when,
            schema_defs,
            name,
            property_names::IMMUTABLE,
            &format!("condition of \"{property}\""),
            full_validation,
            platform_version,
        )?;
        conditions.insert(property, condition);
    }

    if full_validation {
        if !(always.is_empty() && conditions.is_empty()) && !document_type.documents_mutable {
            return Err(structure_error(format!(
                "document type \"{name}\" lists `immutable` properties but its documents are not \
                 mutable (documentsMutable: false), so every property is already immutable; \
                 remove the `immutable` list or set documentsMutable: true"
            )));
        }
        for property in always.iter().chain(conditions.keys()) {
            if property.starts_with('$') {
                return Err(structure_error(format!(
                    "document type \"{name}\" lists system property \"{property}\" as immutable: \
                     system properties are managed by the platform and cannot be listed"
                )));
            }
            if !document_type.properties.contains_key(property) {
                let hint = if property.contains('.') {
                    "; nested paths are not accepted, list the top-level property that contains \
                     it to freeze it whole"
                } else {
                    ""
                };
                return Err(structure_error(format!(
                    "document type \"{name}\" lists \"{property}\" as immutable, but it is not a \
                     top-level property of the document type{hint}"
                )));
            }
            // A transient property is never stored, so the stored document
            // always lacks it and any replace that supplies it counts as
            // setting it. Frozen at "absent", it could never be written; if
            // it is also required, no replace could ever pass at all.
            if document_type.transient_fields.contains(property) {
                return Err(structure_error(format!(
                    "document type \"{name}\" lists \"{property}\" as both transient and \
                     immutable: a transient property is never stored, so every replace that \
                     supplies it while it is frozen would be refused; remove it from one of the \
                     two lists"
                )));
            }
        }
    }

    document_type.immutable_fields = always;
    document_type.immutable_field_conditions = conditions;

    Ok(())
}

/// The `summableOffCountIndex` rules an indexOnly document type checks, for one summableOffCountIndex
/// index: the reason it is refused, or `None`.
///
/// - The source is another index of the type that holds every document
///   exactly once: it keeps entries (it is not a summableOffCountIndex index itself), skips no
///   document, outlives no delete and involves no `$createdAt` (no window
///   fans a document out).
/// - Every source property is a property of the summableOffCountIndex index, so a group
///   here never merges several source groups.
/// - Every other property is fixed by the source: a referring value of a
///   `where` on a same-contract `permanentDocument` or `moderatedDocument`
///   reference a source property holds ([`Index::count_index_derivations`]), so
///   a source group never spreads over several groups here.
/// - No other index continues below the summableOffCountIndex index's last property:
///   that value position holds the group's counter, not a tree.
fn summable_off_count_index_error(
    index_name: &str,
    index: &Index,
    source_name: &str,
    document_type: &DocumentTypeV2,
    document_type_name: &str,
) -> Option<String> {
    let prefix = format!(
        "summableOffCountIndex index \"{index_name}\" on indexOnly document type \"{document_type_name}\""
    );
    let Some(source) = document_type.indices.get(source_name) else {
        return Some(format!(
            "{prefix} sums the count of \"{source_name}\", which is not an index of the \
             document type"
        ));
    };
    if source.is_summable_off_count_index() {
        return Some(format!(
            "{prefix} sums the count of \"{source_name}\", which is a summableOffCountIndex index itself and \
             keeps no entries"
        ));
    }
    // One summed value per document type, as for property sums: sum queries
    // name it, and they name this one by the source index. (A property sum
    // named like the source is refused below, the source sharing a
    // property's name.)
    if let Some((other_name, other_value)) =
        document_type
            .indices
            .iter()
            .find_map(|(other_name, other)| {
                other
                    .summed_value_name()
                    .filter(|other_value| *other_value != source_name)
                    .map(|other_value| (other_name, other_value))
            })
    {
        return Some(format!(
            "{prefix} sums the count of \"{source_name}\", but index \"{other_name}\" sums \
             \"{other_value}\": a document type keeps one summed value, which sum queries name"
        ));
    }
    if document_type.flattened_properties.contains_key(source_name) {
        return Some(format!(
            "{prefix} sums the count of \"{source_name}\", which is also a property of the \
             document type: sum queries name the source index for the summed value, so it must \
             not share its name with a property"
        ));
    }
    if !source.keys_each_live_document_by_its_values() {
        return Some(format!(
            "{prefix} sums the count of \"{source_name}\", which must hold every document \
             exactly once: it may not skip documents (skipIfAbsent), keep the entries of deleted \
             ones (outlivesDelete) or involve $createdAt"
        ));
    }
    if let Some(missing) = source.properties.iter().find(|source_property| {
        !index
            .properties
            .iter()
            .any(|property| property.name == source_property.name)
    }) {
        return Some(format!(
            "{prefix} lacks \"{}\", a property of its source \"{source_name}\": every source \
             property must be a property of the summableOffCountIndex index, or one of its groups would \
             count several source groups",
            missing.name
        ));
    }
    if let Err(property) = index.count_index_derivations(
        source,
        &document_type.flattened_properties,
        document_type.data_contract_id,
    ) {
        return Some(format!(
            "{prefix} has \"{property}\", which is neither a property of its source \
             \"{source_name}\" nor fixed by it: it must be a referring value of a `where` on a \
             same-contract permanentDocument or moderatedDocument reference by id (no findBy or \
             inList) held by a property of \"{source_name}\", or one source group would spread \
             over several groups"
        ));
    }
    let depth = index.properties.len();
    if let Some((other_name, _)) = document_type.indices.iter().find(|(_, other)| {
        other.properties.len() > depth && index.shares_leading_levels(other, depth)
    }) {
        return Some(format!(
            "{prefix} is continued by index \"{other_name}\", which lists its properties first: \
             the value position of a summableOffCountIndex index's last property holds the group's \
             counter, so no other index can continue below it"
        ));
    }
    None
}

/// Parses a condition a replace is judged by, the `when` of an `immutable`
/// entry or a type's `retractedWhen`. It takes the grammar of a
/// `propertyConstraints` rule, and what it reads is checked as a rule's reads
/// are ([`super::validate_property_constraint_reads`]), on every parse. Beyond
/// a rule, it may read the stored document through `$old.`, as a replace
/// judges it on the document it writes, and may read no `countOf` or `sumOf`:
/// it reads the document alone, so a replace judges it without reading state.
/// Under full validation it stays within the node limit of a rule and lists no
/// condition twice. Every message reads `document type "{name}" {keyword}
/// {subject} ...`.
#[allow(clippy::too_many_arguments)]
fn parse_replace_condition(
    document_type: &DocumentTypeV2,
    when: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    name: &str,
    keyword: &str,
    subject: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<PropertyConstraint, ProtocolError> {
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    let condition_error = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "document type \"{name}\" {keyword} {message}"
        ))
    };
    let property_kind = |path: &str| {
        super::property_equality_kind(
            &document_type.flattened_properties,
            path.strip_prefix(STORED_DOCUMENT_PREFIX).unwrap_or(path),
        )
    };
    let condition =
        parse_property_constraint_condition(when, name, &property_kind).map_err(|message| {
            consensus_or_protocol_data_contract_error(condition_error(format!(
                "{subject} {message}"
            )))
        })?;
    super::validate_property_constraint_reads(
        document_type,
        schema_defs,
        subject,
        &condition,
        true,
        &condition_error,
    )
    .map_err(consensus_or_protocol_data_contract_error)?;
    if !condition.aggregate_reads().is_empty() {
        return Err(structure_error(format!(
            "document type \"{name}\" {keyword} {subject} reads a countOf or sumOf total: a \
             condition reads the document alone, so a replace judges it without reading state"
        )));
    }
    if full_validation {
        let max_nodes = platform_version.system_limits.max_property_constraint_nodes;
        let nodes = condition.node_count();
        if nodes > usize::from(max_nodes) {
            return Err(structure_error(format!(
                "document type \"{name}\" {keyword} {subject} has {nodes} nodes, above the \
                 maximum of {max_nodes}"
            )));
        }
        if let Some((repeat, earlier)) = condition.repeated_condition() {
            return Err(structure_error(format!(
                "document type \"{name}\" {keyword} {subject} at {repeat} repeats the \
                 condition at {earlier}"
            )));
        }
    }
    Ok(condition)
}

/// Reads the doctype-level `retractedWhen` keyword (protocol version 14) onto
/// the parsed document type: the condition under which a replace writes a
/// retracted document. On a moderated contract a banned or suspended owner can
/// write nothing new, but the batch transformer lets it make this one replace,
/// refusing each of its replaces whose written document does not meet the
/// condition. So an author whose documents can not be deleted (`canBeDeleted:
/// false`) can still take back what it wrote. The condition is parsed as an
/// `immutable` entry's `when` ([`parse_replace_condition`]) and judged as one,
/// on the document the replace writes with the stored one under `$old.`.
///
/// What a retracted document holds is the type's own rules to say: a
/// `propertyConstraints` rule that it carries no content, and an `immutable`
/// entry that keeps it retracted. The condition only picks the replaces a
/// barred owner may make; every other rule of the type still judges them.
///
/// The type's documents must be mutable, or there is no replace, and the
/// contract must keep a banlist or a suspension list, or no owner is ever
/// barred. The keyword and the moderation config both arrive with protocol
/// version 14, so no stored contract breaks either rule and both are checked
/// on every parse. Runs after the core parse, whose properties it reads.
pub(super) fn apply_retracted_when(
    document_type: &mut DocumentTypeV2,
    data_contract_config: &DataContractConfig,
    schema_defs: Option<&BTreeMap<String, Value>>,
    name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let keyword = property_names::RETRACTED_WHEN;
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
    };
    let Some(when) = document_type
        .schema
        .to_map()
        .ok()
        .and_then(|schema_map| Value::get_optional_from_map(schema_map, keyword))
        .cloned()
    else {
        return Ok(());
    };
    if !document_type.documents_mutable {
        return Err(structure_error(format!(
            "document type \"{name}\" sets `{keyword}`, but its documents are not mutable \
             (documentsMutable: false), so there is no replace to retract with"
        )));
    }
    let bars_anyone = data_contract_config
        .moderation()
        .is_some_and(|moderation| moderation.barring_lists().next().is_some());
    if !bars_anyone {
        return Err(structure_error(format!(
            "document type \"{name}\" sets `{keyword}`, which names the replace a banned or \
             suspended owner may still make, but the contract keeps neither a banlist nor a \
             suspension list, so no owner is ever barred (moderation can only be declared when \
             the contract is created)"
        )));
    }
    let condition = parse_replace_condition(
        document_type,
        &when,
        schema_defs,
        name,
        keyword,
        "condition",
        full_validation,
        platform_version,
    )?;
    document_type.retracted_when = Some(condition);
    Ok(())
}

/// The `skipIfAbsent` rules every document type shares, for one index that
/// declares the keyword: the reason it is refused, or `None`.
///
/// - The skip set is non-empty: an index none of whose properties is optional
///   could never skip.
/// - Every skip property is a top-level (non-dotted) schema property that is
///   not a system property and not listed in `required`, or a derived index
///   property, which reads through a reference of the type. Its presence is
///   then a single lookup, the same for every walker: Drive puts a derived
///   value into the document's properties under the derived name before it
///   keys the document. Whether a schema property can actually be absent is
///   judged here; a derived one is judged where the referenced type is in
///   hand (`resolve_derived_index_properties`).
/// - No ranking level sits above the index's deepest skip property. Such a
///   level would count only the documents carrying the deeper property, but a
///   query may only read a skip index when it binds every skip property, so
///   nothing could ever read it.
fn skip_if_absent_index_error(
    index_name: &str,
    index: &Index,
    document_type_name: &str,
    required_fields: &BTreeSet<String>,
    flattened_properties: &IndexMap<String, DocumentProperty>,
) -> Option<String> {
    if index.skip_if_absent_properties.is_empty() {
        return Some(format!(
            "index \"{}\" on document type \"{}\" declares `skipIfAbsent`, but none of its \
             properties is optional: a required or system property can never be absent, so \
             the index could never skip — remove the flag, or remove a property from \
             `required` (`true` leaves out a value read through a reference: name one in \
             the array form to skip on it)",
            index_name, document_type_name,
        ));
    }
    for skip_property in index.skip_if_absent_properties.iter() {
        if skip_property.starts_with('$') {
            return Some(format!(
                "index \"{}\" on document type \"{}\" skips on system property \"{}\": a skip \
                 property must be an optional schema property, and system properties are \
                 always present",
                index_name, document_type_name, skip_property,
            ));
        }
        if skip_property.contains('.')
            && !reads_through_reference(skip_property, flattened_properties)
        {
            return Some(format!(
                "index \"{}\" on document type \"{}\" skips on nested property \"{}\": a skip \
                 property must be a top-level property, so that presence is a single lookup \
                 with no partially-present ancestor states",
                index_name, document_type_name, skip_property,
            ));
        }
        if required_fields.contains(skip_property) {
            return Some(format!(
                "index \"{}\" on document type \"{}\" skips on \"{}\", which is listed in \
                 `required`: a required property can never be absent, so the index could \
                 never skip on it — remove \"{}\" from `required` or from the skipIfAbsent \
                 list",
                index_name, document_type_name, skip_property, skip_property,
            ));
        }
    }
    let deepest_skip = index
        .properties
        .iter()
        .enumerate()
        .rev()
        .find(|(_, property)| index.skip_if_absent_properties.contains(&property.name));
    if let Some((deepest_skip_position, deepest_skip_property)) = deepest_skip {
        for position in index.at_level_positions(index.ranked_at_levels()) {
            if position < deepest_skip_position {
                let ranked_at = &index.properties[position].name;
                return Some(format!(
                    "index \"{}\" on document type \"{}\" ranks at \"{}\", above its skip \
                     property \"{}\": that ranking would count only documents carrying \"{}\", \
                     and no query can read a skip index without binding every skip property \
                     — rank at or below \"{}\"",
                    index_name,
                    document_type_name,
                    ranked_at,
                    deepest_skip_property.name,
                    deepest_skip_property.name,
                    deepest_skip_property.name,
                ));
            }
        }
    }
    None
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
/// On a stored type it refuses the index keywords only an indexOnly type may
/// use and checks the `skipIfAbsent` rules a stored type follows.
///
/// Must run AFTER [`apply_doctype_aggregates`] — it rejects the doctype-level
/// aggregate flags, which describe the primary-key tree an indexOnly type
/// does not have.
pub(super) fn apply_index_only(
    document_type: &mut DocumentTypeV2,
    index_only: bool,
    name: &str,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    use crate::document::property_names::{CREATED_AT, OWNER_ID};

    // Only generation 3 calls this, so no protocol version before 14 sees
    // these rules or the class of error they are reported with.
    let structure_error = |message: String| {
        consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
            message,
        ))
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
        // An index without properties is only meaningful as a flat indexOnly
        // index (keyed by its terminal alone); on a stored type it would
        // reach no level at all and index nothing.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.properties.is_empty())
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" has no properties: an index keyed by \
                 a terminal alone (a flat index) is only allowed on an indexOnly document \
                 type",
                index_name, name,
            )));
        }
        if !document_type.entry_payload.is_empty() {
            return Err(structure_error(format!(
                "document type \"{}\" declares `entryPayload`, which is only allowed on \
                 indexOnly document types (a stored document type keeps every property in \
                 its primary row)",
                name,
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
        // And for `summableOffCountIndex`: a stored type's index entries are references to
        // stored rows, whose count its own count trees already keep.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.is_summable_off_count_index())
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" declares `summableOffCountIndex`, which is only \
                 allowed on indexOnly document types (set `indexOnly: true` on the document \
                 type, or remove the keyword)",
                index_name, name,
            )));
        }
        // And for `outlivesDelete`: a stored document is deleted by id, its
        // stored row saying where each of its entries is, so a delete never
        // lacks a value to find one by.
        if let Some((index_name, _)) = document_type
            .indices
            .iter()
            .find(|(_, index)| index.outlives_delete)
        {
            return Err(structure_error(format!(
                "index \"{}\" on document type \"{}\" declares `outlivesDelete`, which is \
                 only allowed on indexOnly document types (set `indexOnly: true` on the \
                 document type, or remove the flag)",
                index_name, name,
            )));
        }
        // `skipIfAbsent` on a stored type: a document that omits a property
        // of the index's skip set writes no reference into the index; the
        // index's other optional properties keep the null layout.
        for (index_name, index) in document_type.indices.iter() {
            if !index.skip_if_absent {
                continue;
            }
            if let Some(message) = skip_if_absent_index_error(
                index_name,
                index,
                name,
                &document_type.required_fields,
                &document_type.flattened_properties,
            ) {
                return Err(structure_error(message));
            }
            // A contest is decided over the document's full value tuple; a
            // document the index skipped would have no vote poll to enter.
            if index.contested_index.is_some() {
                return Err(structure_error(format!(
                    "index \"{}\" on document type \"{}\" is contested and declares \
                     `skipIfAbsent`: a contested index needs every value of every \
                     document, so it cannot skip",
                    index_name, name,
                )));
            }
            // An empty byte array is keyed like a missing value, the empty
            // key, so a stored skip property that could be empty would be
            // indexed under the key other indexes use for documents without
            // it.
            for skip_property in index.skip_if_absent_properties.iter() {
                if let Some(DocumentPropertyType::ByteArray(sizes)) = document_type
                    .flattened_properties
                    .get(skip_property)
                    .map(|property| &property.property_type)
                {
                    if sizes.min_size.unwrap_or_default() == 0 {
                        return Err(structure_error(format!(
                            "index \"{}\" on document type \"{}\" skips on byte array \
                             \"{}\", which may be empty: an empty byte array is indexed \
                             under the same key as a missing value, so set `minItems` to at \
                             least 1",
                            index_name, name, skip_property,
                        )));
                    }
                }
            }
            // A ranking at a skip property's level must not share that level
            // with an index that keeps the null layout for the property: that
            // index creates the level's null value tree for documents the skip
            // index leaves out, and the ranking would show it as a group with
            // zero aggregates and no documents behind it.
            for (position, skip_property) in
                index.properties.iter().enumerate().filter(|(_, property)| {
                    index.skip_if_absent_properties.contains(&property.name)
                })
            {
                let last = position + 1 == index.properties.len();
                let ranks_here = index.ranked_at_levels().any(|at| *at == skip_property.name)
                    || (last && index.ranks_its_last_property());
                if !ranks_here {
                    continue;
                }
                if let Some((other_name, _)) =
                    document_type.indices.iter().find(|(other_name, other)| {
                        *other_name != index_name
                            && !other
                                .skip_if_absent_properties
                                .contains(&skip_property.name)
                            && index.shares_leading_levels(other, position + 1)
                    })
                {
                    return Err(structure_error(format!(
                        "index \"{}\" on document type \"{}\" ranks at its skip property \
                         \"{}\", whose level index \"{}\" shares without skipping on it: \
                         \"{}\" would create the null group of that level, and the ranking \
                         would show it with zero aggregates; skip on \"{}\" in \"{}\" too, \
                         or give the ranking its own prefix",
                        index_name,
                        name,
                        skip_property.name,
                        other_name,
                        other_name,
                        skip_property.name,
                        other_name,
                    )));
                }
            }
            // A document whose indexed values are all missing lacks the skip
            // properties too, so the skip already leaves it out:
            // `nullSearchable: false` would add nothing.
            if !index.null_searchable {
                return Err(structure_error(format!(
                    "index \"{}\" on document type \"{}\" sets both `skipIfAbsent` and \
                     `nullSearchable: false`: a document with every indexed value missing \
                     also misses the skip properties, so the skip already leaves it out — \
                     remove `nullSearchable`",
                    index_name, name,
                )));
            }
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

    // ---- entry payload --------------------------------------------------
    // `entryPayload` names the type's value slot: top-level scalar
    // properties stored in every entry's value, after the row commitment,
    // instead of in a key. They are still committed (the commitment hashes
    // every present property) and still required, but they sit in no
    // index, so the every-property-indexed rule below exempts them. Each
    // must be bounded, since fee estimation sizes the entry value by the
    // sum of their bounds, and the sum is capped by the field value limit.
    let mut payload_max_total: u32 = 0;
    for payload_property in document_type.entry_payload.iter() {
        let Some(property) = document_type.properties.get(payload_property) else {
            return Err(structure_error(format!(
                "entryPayload of indexOnly document type \"{}\" names \"{}\", which is not \
                 a top-level property of the document type",
                name, payload_property,
            )));
        };
        if matches!(
            property.property_type,
            DocumentPropertyType::Object(_)
                | DocumentPropertyType::Array(_)
                | DocumentPropertyType::VariableTypeArray(_)
                | DocumentPropertyType::TypedArray(_)
        ) {
            return Err(structure_error(format!(
                "entryPayload property \"{}\" of indexOnly document type \"{}\" must be a \
                 scalar (a byte array, string, integer, boolean, date or identifier): the \
                 entry value is a flat concatenation of length-framed scalars",
                payload_property, name,
            )));
        }
        // A string whose `maxLength` puts its worst case past `u16::MAX` bytes
        // overflows the width computation; it could never fit an entry's value.
        let max_width = match property.property_type.max_byte_size(platform_version) {
            Ok(max_width) => max_width.unwrap_or(u16::MAX),
            Err(ProtocolError::Overflow(_)) => {
                return Err(structure_error(format!(
                    "entryPayload property \"{}\" of indexOnly document type \"{}\" may \
                     encode to more than {} bytes, over the {}-byte cap on an entry's value",
                    payload_property,
                    name,
                    u16::MAX,
                    platform_version.system_limits.max_field_value_size,
                )))
            }
            Err(error) => return Err(error),
        };
        if max_width == u16::MAX {
            return Err(structure_error(format!(
                "entryPayload property \"{}\" of indexOnly document type \"{}\" must be \
                 bounded (declare maxItems on a byte array or maxLength on a string): fee \
                 estimation sizes every entry's value by the payload bounds",
                payload_property, name,
            )));
        }
        // Two bytes of length frame per property.
        payload_max_total += u32::from(max_width) + 2;
        if !document_type.required_fields.contains(payload_property) {
            return Err(structure_error(format!(
                "entryPayload property \"{}\" of indexOnly document type \"{}\" must be \
                 listed in `required`: the entry value has no representation for an absent \
                 property",
                payload_property, name,
            )));
        }
        if let Some((index_name, _)) = document_type.indices.iter().find(|(_, index)| {
            index.terminal_contains(payload_property)
                || index
                    .properties
                    .iter()
                    .any(|index_property| index_property.name == *payload_property)
        }) {
            return Err(structure_error(format!(
                "entryPayload property \"{}\" of indexOnly document type \"{}\" also \
                 appears in index \"{}\": a property is either a key (a prefix property or \
                 a terminal component) or entry payload, never both",
                payload_property, name, index_name,
            )));
        }
    }
    if payload_max_total > platform_version.system_limits.max_field_value_size {
        return Err(structure_error(format!(
            "entryPayload of indexOnly document type \"{}\" may encode to {} bytes, over \
             the {}-byte cap on an entry's value",
            name, payload_max_total, platform_version.system_limits.max_field_value_size,
        )));
    }

    // ---- per-index rules ------------------------------------------------
    for (index_name, index) in document_type.indices.iter() {
        if index.properties.is_empty() {
            // FLAT index: no prefix levels, the entries live directly under
            // a level keyed by the terminal's component names. There is no
            // prefix level for an aggregate, a ranking, a time grid, a skip
            // property or a preallocation to apply to, so none of those
            // keywords is admitted on it.
            if index.countable.is_countable()
                || index.range_countable
                || index.summable.is_some()
                || index.range_summable
                || index.declares_any_ranking()
                || index.is_bucketed()
                || index.skip_if_absent
                || index.preallocated
            {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" has no properties (a \
                     flat index keyed by its terminal alone), so it admits no countable, \
                     summable, ranked, timeRange, integerRange, skipIfAbsent or preallocated \
                     keyword: \
                     there is no prefix level for them to apply to",
                    index_name, name,
                )));
            }
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
                 false: an indexOnly property is either required or a skip property of \
                 every index holding it, so no null entries exist to suppress (a \
                 skipIfAbsent index writes nothing for a document it skips; \
                 nullSearchable suppresses stored-type null-layout entries, which \
                 indexOnly types never write)",
                index_name, name,
            )));
        }
        // `skipIfAbsent`: the skip set's members, and the levels an
        // aggregate may sit at, follow the rules every document type shares.
        // What only an indexOnly type needs (every optional property of an
        // index is in its skip set, and each has a skip index of its own) is
        // checked with coverage below.
        if index.skip_if_absent {
            if let Some(message) = skip_if_absent_index_error(
                index_name,
                index,
                name,
                &document_type.required_fields,
                &document_type.flattened_properties,
            ) {
                return Err(structure_error(message));
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
        //
        // `integerRange` is refused. An indexOnly index keeps one entry per
        // (prefix values, terminal), and a bucketed level holds window
        // starts, not values: two rows that differ only in the bucketed
        // integer would claim the same entry in every window they share, so
        // the index would silently act as a uniqueness constraint over
        // overlapping value bands. (A `$createdAt` window cannot do this: the
        // proof index, which involves no `$createdAt`, already keeps rows
        // that differ only in `$createdAt` apart.)
        if index.integer_range.is_some() {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" declares integerRange: an \
                 indexOnly index keeps one entry per window and other values, so rows that \
                 differ only in the bucketed integer would collide in the windows they share; \
                 bucket integers on a document type that stores its documents",
                index_name, name,
            )));
        }
        // The sum axes (summable / rangeSummable / rankedSummable /
        // rankedAverageable / the averageable sugar) are admitted: a
        // summable index's terminal entry is an
        // `ItemWithSumItem(commitment, amount)` carrying the summed
        // property's value, and the doctype-level summable cross-checks
        // (canonical property, i64-safe integer type, `required`
        // membership) run for every doctype, indexOnly included.

        // `parse_indices` gives every index of an indexOnly type but a
        // summableOffCountIndex one a terminal, and the index parser refuses an empty
        // one, so a contract cannot get here: this is the parser failing, not
        // the contract. A summableOffCountIndex index keeps no entries, so it has no
        // member key to check.
        let components = index.terminal_components();
        if components.is_empty() && !index.is_summable_off_count_index() {
            return Err(ProtocolError::CorruptedCodeExecution(format!(
                "index \"{}\" on indexOnly document type \"{}\" has no terminal after \
                 normalization: internal parser error",
                index_name, name,
            )));
        }

        for component in components {
            if index
                .properties
                .iter()
                .any(|property| property.name == *component)
            {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" repeats its terminal \
                     component (\"{}\") in its properties: the terminal is the member key \
                     below the listed properties, so listing it again would index the same \
                     dimension twice",
                    index_name, name, component,
                )));
            }
        }

        // The terminal is the member key: the encoded values of its
        // components, concatenated in order. Any property a prefix position
        // admits may serve as a component: every path derives the member
        // key through the same tree-key encoding the prefix levels use (the
        // walkers and probes via `get_raw_for_document_type`, queries and
        // executed proofs via `serialize_value_for_key`, synthesis via
        // `decode_value_for_tree_keys`), so a component needs no particular
        // width or meaning — only the shape limits every indexed value
        // carries. Structural uniqueness spans the whole key: one entry per
        // (prefix values, terminal values).
        //
        // Every component but the last must be fixed width: a leading
        // component is followed by more key bytes, and only a fixed-width
        // encoding keeps equality on the leading components a clean key
        // range (and lets synthesis split the key back). Strings are never
        // fixed width (their bound counts characters, not bytes), so a
        // string can only be the last component.
        //
        // System properties other than `$ownerId` are refused: `$createdAt`
        // is the one other system value an indexOnly entry can carry, and
        // the rules that reason about it (the proof-index selection,
        // `required` membership, bucketing) all walk the prefix properties,
        // so admitting it as a component would need each of them extended
        // first.
        let mut terminal_max_width: u32 = 0;
        for (position, component) in components.iter().enumerate() {
            let is_last = position + 1 == components.len();
            let max_width: u32 = if component == OWNER_ID {
                32
            } else {
                if component.starts_with('$') {
                    return Err(structure_error(format!(
                        "terminal component \"{}\" of index \"{}\" on indexOnly document \
                         type \"{}\" is a system property: only $ownerId may be a terminal \
                         component (name a schema property, or list $createdAt among the \
                         index's properties instead)",
                        component, index_name, name,
                    )));
                }
                // A flat level is keyed by its component names, each behind
                // a zero byte (`flat_level_key_for`); a name carrying one
                // would alias another flat level or a property-name tree.
                // The meta-schema's name pattern already excludes it for
                // contracts entering the chain; this keeps the invariant
                // explicit for every parse.
                if component.contains('\0') {
                    return Err(structure_error(format!(
                        "terminal component \"{}\" of index \"{}\" on indexOnly document \
                         type \"{}\" contains a zero byte, which the flat level key \
                         reserves as its separator",
                        component.escape_default(),
                        index_name,
                        name,
                    )));
                }
                let Some(property) = document_type.flattened_properties.get(component) else {
                    return Err(structure_error(format!(
                        "terminal component \"{}\" of index \"{}\" on indexOnly document \
                         type \"{}\" does not name a property of the document type",
                        component, index_name, name,
                    )));
                };
                check_indexable_property_shape(
                    name,
                    index_name,
                    component,
                    &property.property_type,
                )?;
                let max_width = property
                    .property_type
                    .max_byte_size(platform_version)?
                    .unwrap_or(u16::MAX);
                // Fixed width by the tree-key encoding itself: the same
                // helper synthesis splits member keys with.
                let fixed_width = property.property_type.fixed_tree_key_width().is_some();
                if !is_last && !fixed_width {
                    return Err(structure_error(format!(
                        "terminal component \"{}\" of index \"{}\" on indexOnly document \
                         type \"{}\" is followed by another component but is not fixed \
                         width: every component but the last must encode to a fixed number \
                         of bytes (a byte array with minItems equal to maxItems, an \
                         identifier, an integer, a boolean or a date); a string or a \
                         variable-size byte array can only be the last component",
                        component, index_name, name,
                    )));
                }
                u32::from(max_width)
            };
            terminal_max_width += max_width;
        }
        if terminal_max_width > u32::from(MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH) {
            return Err(structure_error(format!(
                "the terminal of index \"{}\" on indexOnly document type \"{}\" encodes to \
                 up to {} bytes, over the {}-byte member key cap: shorten or drop a \
                 component",
                index_name, name, terminal_max_width, MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH,
            )));
        }

        // The flat level is itself a GroveDB key. Bounding the encoded
        // values above does not bound the concatenated component names.
        if let Some(flat_key) = index.flat_level_key() {
            if flat_key.len() > usize::from(MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH) {
                return Err(structure_error(format!(
                    "the flat level of index \"{}\" on indexOnly document type \"{}\" \
                     encodes to {} bytes, over the {}-byte flat level key cap: shorten \
                     or drop a terminal component name",
                    index_name,
                    name,
                    flat_key.len(),
                    MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH,
                )));
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
        // uniqueness-without-owner shape. A summableOffCountIndex index keeps no
        // entries: a delete takes its count back only once the entries of
        // the indexes that keep them, its source among them, matched.
        if !index.is_summable_off_count_index() && !index.involves(OWNER_ID) {
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
        if (index.terminal_contains(CREATED_AT)
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
        if index.preallocated && index.is_bucketed() {
            return Err(structure_error(format!(
                "index \"{}\" on indexOnly document type \"{}\" declares `preallocated` \
                 together with `timeRange` or `integerRange`: a bucketed level is keyed by \
                 window starts computed at write time, so its path cannot be preallocated \
                 from a referenced document",
                index_name, name,
            )));
        }

        // `outlivesDelete` leaves the index's entries behind when their
        // document is deleted, so they must expire on their own: only a
        // `timeRange` index with a `ttl` qualifies, whose windows are dropped
        // whole once past it. A deleted document's entries keep what it
        // wrote until a later document with the same key writes over them,
        // in the windows the two share only: an amount (a sum) or payload
        // values would then mix two documents' across the windows.
        if index.outlives_delete {
            if index
                .time_range
                .as_ref()
                .is_none_or(|time_range| time_range.ttl_seconds.is_none())
            {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares \
                     `outlivesDelete` without a `timeRange` carrying a `ttl`: the entries a \
                     delete leaves must expire with their window, or they would count for \
                     good",
                    index_name, name,
                )));
            }
            if index.summable.is_some() {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares \
                     `outlivesDelete` together with a sum: a deleted document's entries keep \
                     its amount until a later document writes over them in the windows the \
                     two share, mixing two documents' amounts across the windows",
                    index_name, name,
                )));
            }
            if !document_type.entry_payload.is_empty() {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares \
                     `outlivesDelete` on a type with `entryPayload`: a deleted document's \
                     entries keep its payload until a later document writes over them in the \
                     windows the two share, mixing two documents' payloads across the windows",
                    index_name, name,
                )));
            }
            // A create writes over an entry already standing here, so two
            // documents in state must never share one: the index's key
            // (every property but `$createdAt`, and its terminal) must hold
            // the whole key of an index a delete clears and that skips no
            // document, whose entry the create probes as a duplicate. The
            // entry then stands for no other live document than the one
            // writing it.
            let key: BTreeSet<&str> = index
                .properties
                .iter()
                .map(|property| property.name.as_str())
                .chain(index.terminal_components().iter().map(String::as_str))
                .filter(|name| *name != CREATED_AT)
                .collect();
            // (An index involving `$createdAt` never fits that key, so the
            // shared predicate's `$createdAt` term changes nothing here.)
            let keyed_by_a_cleared_index = document_type.indices.values().any(|other| {
                other.keys_each_live_document_by_its_values()
                    && other
                        .properties
                        .iter()
                        .map(|property| property.name.as_str())
                        .chain(other.terminal_components().iter().map(String::as_str))
                        .all(|name| key.contains(name))
            });
            if !keyed_by_a_cleared_index {
                return Err(structure_error(format!(
                    "index \"{}\" on indexOnly document type \"{}\" declares \
                     `outlivesDelete`, but its key (its properties but $createdAt, and its \
                     terminal) holds the whole key of no index that a delete clears and that \
                     skips no document: two documents in state could then share one of its \
                     entries, which a create writes over",
                    index_name, name,
                )));
            }
        }

        // The binding derivation is shared with the rs-drive insert path
        // (see `index::preallocation`); rejecting a flag with no binding
        // here is what lets that path trust every `preallocated: true` it
        // sees. A binding through a moderatedDocument reference also needs
        // the target's removal record to keep every key it binds, which
        // only the parse of the whole contract can see
        // (`validate_preallocated_indexes_kept_on_removal`).
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
                 be either a property with a same-contract permanentDocument or \
                 moderatedDocument `refersTo` declaration (the referring property — its \
                 value is the referenced document's $id; a deletableDocument declaration \
                 does not qualify, since the trees would outlive a deleted target with \
                 nothing left to say what they were keyed by) or a referring value of \
                 that declaration's `where` \
                 (consensus-equal to a referenced-document property, which may be the \
                 referenced document's $ownerId or $creatorId). The referring document's \
                 OWN system properties like $ownerId cannot be determined by the \
                 referenced document, so a preallocated index may carry $ownerId only as \
                 its terminal",
                index_name, name,
            )));
        }
        // `summableOffCountIndex`: a summableOffCountIndex index keeps, per group, the number of its
        // source's entries in that group. The count is lossless when the
        // source holds every document exactly once and a group here holds
        // exactly the documents of one source group. That the values fixing
        // a group never change is judged with the referenced types, by the
        // parse of the whole contract (`validate_summable_off_count_indexes_lossless`).
        if let Some(source_name) = &index.summable_off_count_index {
            if let Some(message) =
                summable_off_count_index_error(index_name, index, source_name, document_type, name)
            {
                return Err(structure_error(message));
            }
        }
    }

    // At least one index must involve no `$createdAt` at all AND not be
    // `skipIfAbsent` — the PROOF index. Executed-transition proofs
    // (waitForStateTransitionResult) locate the entry a create or delete
    // produced from the transition's values alone; a client verifier
    // cannot know the block timestamp an entry was keyed with, and a
    // skipIfAbsent index has no entry at all for the documents it skips.
    // If every index were time-keyed or skippable, creates and deletes of
    // the type would work while transition-proof requests failed. (Every
    // index already embeds `$ownerId`, so any `$createdAt`-free non-skip
    // index qualifies as the proof index.) Nor may it outlive deletes: a
    // delete's proof shows the entry gone, which such an index keeps.
    let has_proof_index = document_type
        .indices
        .values()
        .any(Index::keys_each_live_document_by_its_values);
    if !has_proof_index {
        return Err(structure_error(format!(
            "indexOnly document type \"{}\" must declare at least one index that neither \
             involves $createdAt nor sets skipIfAbsent or outlivesDelete: \
             executed-transition proofs locate entries from the transition's values alone — \
             they cannot reproduce the block timestamp a time-keyed entry was written with, \
             a skipIfAbsent index has no entry for the documents it skips, and an \
             outlivesDelete index keeps the entries a delete leaves",
            name,
        )));
    }

    // ---- coverage and requiredness --------------------------------------
    // The index content IS the document: a property in no index would not
    // exist, and an absent value has no representation in an index path.
    // The one sanctioned hole is a skip property: it may be optional because
    // its absence removes the whole entry of every index that holds it —
    // there is genuinely nothing to store. Everything else must be required.
    // A required property must be covered by at least one NON-skip index: a
    // skip index carries no value at all for documents it skips, so a
    // property covered only by skip indexes would be validated, committed
    // into the row commitment, and then written nowhere — unrecoverable by
    // any query, and the document undeletable once the client forgets the
    // value. An optional property must be covered by a skip index whose skip
    // set is that property alone, for the same reason: a document carrying
    // it but missing another skip property of a wider index would otherwise
    // store it nowhere.
    let skip_properties: BTreeSet<&str> = document_type
        .indices
        .values()
        .flat_map(|index| index.skip_if_absent_properties.iter())
        .map(String::as_str)
        .collect();
    // A summableOffCountIndex index keeps no value per document, so it covers nothing.
    // The properties of one that its source lacks are fixed by the source's
    // references (checked above): a client that lost them reads them back
    // from the referenced document, so they need no index of their own.
    let fixed_by_a_source: BTreeSet<&str> = document_type
        .indices
        .values()
        .filter_map(|index| {
            let source = document_type
                .indices
                .get(index.summable_off_count_index.as_deref()?)?;
            index
                .count_index_derivations(
                    source,
                    &document_type.flattened_properties,
                    document_type.data_contract_id,
                )
                .ok()
        })
        .flatten()
        .map(|derivation| derivation.property)
        .collect();
    for (property_name, property) in document_type.flattened_properties.iter() {
        if matches!(property.property_type, DocumentPropertyType::Object(_)) {
            // Containers are covered through their flattened leaves.
            continue;
        }
        if document_type.entry_payload.contains(property_name.as_str()) {
            // Stored in every entry's value: validated above (required,
            // bounded, in no index).
            continue;
        }
        let optional = !document_type.required_fields.contains(property_name);
        if optional && !skip_properties.contains(property_name.as_str()) {
            return Err(structure_error(format!(
                "property \"{}\" on indexOnly document type \"{}\" must be listed in \
                 `required`: the index path is the storage, and an absent value would \
                 need the null index layout this mode deliberately has no equivalent \
                 of (only a property a skipIfAbsent index skips on may be optional)",
                property_name, name,
            )));
        }
        let holds_property =
            |index: &Index| !index.is_summable_off_count_index() && index.involves(property_name);
        if fixed_by_a_source.contains(property_name.as_str()) {
            // Covered by its source's reference; the rules every index
            // holding an optional property follows still apply below.
        } else if optional {
            // A time-windowed index keeps a value only in its windows, which
            // document queries do not read and a `ttl` drains, so it cannot be
            // where an optional value is kept.
            let covered = document_type.indices.values().any(|index| {
                matches!(
                    index.skip_if_absent_properties.as_slice(),
                    [only] if only == property_name
                ) && index.time_range.is_none()
                    && holds_property(index)
            });
            if !covered {
                return Err(structure_error(format!(
                    "optional property \"{}\" on indexOnly document type \"{}\" needs an \
                     index without a timeRange whose only skip property it is: otherwise a \
                     document carrying \"{}\" could keep it only in time windows, or in no \
                     index at all when it misses another skip property of a wider index",
                    property_name, name, property_name,
                )));
            }
        } else if !document_type
            .indices
            .values()
            .any(|index| !index.skip_if_absent && !index.outlives_delete && holds_property(index))
        {
            // Nor may an outlivesDelete index be the only one holding it: a
            // delete carries every schema property, and could not be checked
            // against entries it leaves
            return Err(structure_error(format!(
                "property \"{}\" on indexOnly document type \"{}\" does not appear in any \
                 index that neither sets skipIfAbsent nor outlivesDelete (as a property or \
                 terminal): on an indexOnly type only indexed values exist and are \
                 recoverable, a skipIfAbsent index holds no value at all for documents it \
                 skips, and an outlivesDelete index is not checked by a delete, so the \
                 property would be silently dropped",
                property_name, name,
            )));
        }
        if optional {
            // Every index involving an optional property must skip on it,
            // and it can never be a terminal. This is the invariant the write
            // walkers rely on: a level keyed by an absent optional property is
            // only ever reached by indexes that skip the document, so the
            // walkers build nothing there.
            for (index_name, index) in document_type.indices.iter() {
                if index.terminal_contains(property_name) {
                    return Err(structure_error(format!(
                        "optional property \"{}\" on indexOnly document type \"{}\" is the \
                         terminal of index \"{}\": a terminal is every entry's member key \
                         and can never be absent — list the property in `required` or \
                         change the terminal",
                        property_name, name, index_name,
                    )));
                }
                let in_index = index
                    .properties
                    .iter()
                    .any(|index_property| index_property.name == *property_name);
                if in_index
                    && !index
                        .skip_if_absent_properties
                        .iter()
                        .any(|skip_property| skip_property == property_name)
                {
                    return Err(structure_error(format!(
                        "optional property \"{}\" on indexOnly document type \"{}\" appears \
                         in index \"{}\", which does not skip documents that omit it: an \
                         index participates for every document unless it skips, and an \
                         absent value has no index representation — set `skipIfAbsent: \
                         true` on the index, name the property in its skipIfAbsent list, or \
                         list the property in `required`",
                        property_name, name, index_name,
                    )));
                }
            }
        }

        // A required nested leaf inside an OPTIONAL ancestor object is only
        // conditionally present — `required: ["targetId"]` inside an
        // unrequired `profile` lets a valid document omit the whole object.
        // Every ancestor path of an indexed dotted property must therefore
        // be required too, or the no-null invariant silently breaks.
        if let Some(ancestor) = unrequired_ancestor(property_name, &document_type.required_fields) {
            return Err(structure_error(format!(
                "property \"{}\" on indexOnly document type \"{}\" sits inside \
                 \"{}\", which is not listed in `required`: a valid document could \
                 omit the whole object, leaving the indexed leaf absent",
                property_name, name, ancestor,
            )));
        }
    }

    Ok(())
}

/// The first object path above the dotted `property_name` that is not in
/// `required_fields`, if any. `required_fields` lists a nested object's own
/// `required` members under their full paths whether or not the object is
/// itself required, so a required leaf is only always present when every
/// ancestor path is required too.
fn unrequired_ancestor(property_name: &str, required_fields: &BTreeSet<String>) -> Option<String> {
    let mut ancestor = String::new();
    for segment in property_name.split('.') {
        if !ancestor.is_empty() {
            if !required_fields.contains(&ancestor) {
                return Some(ancestor);
            }
            ancestor.push('.');
        }
        ancestor.push_str(segment);
    }
    None
}
