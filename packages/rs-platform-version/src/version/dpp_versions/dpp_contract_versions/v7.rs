use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeIndexVersions, DocumentTypeMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions, RecursiveSchemaValidatorVersions, TokenVersions,
};
use versioned_feature_core::FeatureVersionBounds;

// Introduced in protocol version 15 with token shielded pools. Identical to v6 except
// `token_configuration_format` admits format 1 (`TokenConfiguration::V1`, the per-token
// shielded pool flag). v6 keeps max 0 for protocol version 14 chain replay, so a
// pre-activation contract cannot carry the flag.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
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
        // Generation 1 (requiredSince): feeds the new contract version into
        // per-document-type update validation and validates requiredSince
        // annotations on document types introduced by the update, which the
        // per-type pass never sees. Generation 0 stays byte-identical for
        // replay of pre-v14 blocks.
        validate_update: 1,
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
            estimated_size: 1, // changed: adds the document serialization format 3 contract-version stamp varint (worst case 5 bytes) to the estimate
            // Changed: v1 requires a query's bound fields to cover a
            // contiguous prefix of the candidate index (equalities exactly
            // covering the leading properties, range/in immediately after,
            // no unused property before an order-by field). v0 matched by
            // positionless set membership, so a query binding only later
            // index properties selected an index the positional path
            // lowering misaligns on — returning cryptographically proven
            // wrong or empty results. Gapped candidates are now skipped
            // per-candidate, letting a well-shaped index win or the query
            // fail with WhereClauseOnNonIndexedProperty. v0 stays frozen
            // for replay at protocol versions <= 13.
            index_for_types: 1,
            max_size: 0,
            serialize_value_for_key: 0,
            deserialize_value_for_key: 0,
        },
    },
    token_versions: TokenVersions {
        // 1: an epoch-based perpetual distribution needs an interval of at least one epoch.
        validate_structure_interval: 1,
        // changed: `TokenConfigurationV1` (the per-token shielded pool flag) is admitted from
        // protocol version 15; v6 keeps max 0 so a pre-activation contract cannot carry it.
        token_configuration_format: FeatureVersionBounds {
            min_version: 0,
            max_version: 1,
            default_current_version: 0,
        },
    },
};
