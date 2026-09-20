use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeIndexVersions, DocumentTypeMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions, RecursiveSchemaValidatorVersions, TokenVersions,
};
use versioned_feature_core::FeatureVersionBounds;

// Introduced in protocol version 15. Identical to v6 except `apply_update`
// becomes `Some(0)`: the merge of a delta-based (V1) data contract update
// onto the stored contract exists from the version that admits the delta
// transition, and is absent (`None`) on every earlier snapshot. v6 remains
// unchanged for protocol version 14 chain replay.
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
        apply_update: Some(0), // changed: the delta-based (V1) contract update merge exists from protocol version 15
    },
    document_type_versions: DocumentTypeVersions {
        index_versions: DocumentTypeIndexVersions {
            index_levels_from_indices: 0,
        },
        class_method_versions: DocumentTypeClassMethodVersions {
            try_from_schema: 3,
            create_document_types_from_document_schemas: 1,
        },
        structure_version: 0,
        schema: DocumentTypeSchemaVersions {
            document_type_schema: 3,
            should_add_creator_id: 1,
            enrich_with_base_schema: 1,
            find_identifier_and_binary_paths: 0,
            apply_property_reference: Some(0),
            apply_required_since: Some(0),
            validate_max_depth: 0,
            max_depth: 256,
            recursive_schema_validator_versions: RecursiveSchemaValidatorVersions {
                traversal_validator: 0,
            },
            validate_schema_compatibility: 1,
        },
        methods: DocumentTypeMethodVersions {
            create_document_from_data: 0,
            create_document_with_prevalidated_properties: 0,
            prefunded_voting_balance_for_document: 0,
            contested_vote_poll_for_document: 0,
            estimated_size: 1,
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
    },
};
