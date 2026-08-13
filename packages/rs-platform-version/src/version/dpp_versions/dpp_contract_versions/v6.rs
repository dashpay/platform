use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeIndexVersions, DocumentTypeMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions, RecursiveSchemaValidatorVersions, TokenVersions,
};
use versioned_feature_core::FeatureVersionBounds;

// Introduced in protocol version 14. Uses the v3 document meta-schema, which
// is v2 plus the ranked index keywords (rankedCountable / rankedSummable /
// rankedAverageable). v5 (meta-schema v2) remains for protocol version 13 so
// pre-activation validation is unchanged — under v2 those keys still fail an
// index entry's `additionalProperties: false`.
//
// `try_from_schema` moves to 3, selecting a new document-type parser
// generation (`try_from_schema/v3`). New-protocol-version grammar gets its own
// generation module rather than a version gate inside a shipped one, so the
// generation-0/1/2 parsers stay byte-identical to the code consensus already
// ran and replaying a historical block cannot pick up grammar that post-dates
// it. Generation 3 admits the ranked keywords unconditionally — it exists if
// and only if the meta-schema is v3, so it needs no version read of its own.
//
// `document_type_schema` moves to 3 in the same step: generation 3 and
// meta-schema v3 are introduced together and pair by construction. Under v2 the
// ranked keys still fail an index entry's `additionalProperties: false`, so v5
// (protocol version 13) keeps pre-activation validation unchanged.
//
// `validate_schema_compatibility` moves to 1: the compatibility validator has
// no keyword rule for `indices`, so under generation 0 any contract-update
// schema diff under `/indices` that survived the index checks hard-errored
// (an internal error, not a consensus-invalid result). Index definitions are
// validated by `validate_update` v1's name-keyed comparison at protocol
// version 14, so generation 1 strips the top-level `indices` key before
// diffing — an index-order-only update (a semantic no-op) now validates
// cleanly, while real index changes keep their dedicated consensus error.
pub const CONTRACT_VERSIONS_V6: DPPContractVersions = DPPContractVersions {
    max_serialized_size: 65000,
    contract_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 1,
        default_current_version: 1,
    },
    contract_structure_version: 1,
    created_data_contract_structure: 0,
    config: FeatureVersionBounds {
        min_version: 1,
        max_version: 1,
        default_current_version: 1,
    },
    methods: DataContractMethodVersions {
        validate_document: 0,
        validate_update: 0,
        schema: 0,
        validate_groups: 0,
        equal_ignoring_time_fields: 0,
        registration_cost: 1,
    },
    document_type_versions: DocumentTypeVersions {
        index_versions: DocumentTypeIndexVersions {
            index_levels_from_indices: 0,
        },
        class_method_versions: DocumentTypeClassMethodVersions {
            try_from_schema: 3, // changed: parser generation 3 — generation 2 plus the ranked index keywords
            create_document_types_from_document_schemas: 1,
        },
        structure_version: 0,
        schema: DocumentTypeSchemaVersions {
            document_type_schema: 3, // changed: v3 document meta-schema — v2 plus the ranked index keywords, and the gate the index parser reads
            should_add_creator_id: 1,
            enrich_with_base_schema: 1,
            find_identifier_and_binary_paths: 0,
            apply_property_reference: Some(0), // changed: the meta-schema v3 `refersTo` keyword is folded into the parsed property type; None before this version means the keyword is ignored, as it was before it existed
            apply_required_since: Some(0), // changed: the meta-schema v3 `requiredSince` keyword (contract version a property is required from) is parsed onto the property; None before this version means the keyword is ignored, as it was before it existed
            validate_max_depth: 0,
            max_depth: 256,
            recursive_schema_validator_versions: RecursiveSchemaValidatorVersions {
                traversal_validator: 0,
            },
            validate_schema_compatibility: 1, // changed: strips `indices` before diffing — index changes are validated by `validate_update` v1, so an index-order-only diff no longer hard-errors
        },
        methods: DocumentTypeMethodVersions {
            create_document_from_data: 0,
            create_document_with_prevalidated_properties: 0,
            prefunded_voting_balance_for_document: 0,
            contested_vote_poll_for_document: 0,
            estimated_size: 0,
            index_for_types: 0,
            max_size: 0,
            serialize_value_for_key: 0,
            deserialize_value_for_key: 0,
        },
    },
    token_versions: TokenVersions {
        validate_structure_interval: 0,
    },
};
