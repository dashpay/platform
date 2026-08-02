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
// `document_type_schema` doubles as the *grammar* gate, not just the
// meta-schema selector: the index parser only recognizes the ranked keywords
// when this value is >= 3, so a non-validating parse (check_tx, cache warm-up,
// restore) cannot smuggle them onto a pre-v14 node either.
//
// `try_from_schema` deliberately stays at 2: the dispatcher reads the
// meta-schema version out of `document_type_schema` rather than branching on
// its own version, so pointing at meta-schema v3 needs no new dispatch arm.
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
            try_from_schema: 2,
            create_document_types_from_document_schemas: 1,
        },
        structure_version: 0,
        schema: DocumentTypeSchemaVersions {
            document_type_schema: 3, // changed: v3 document meta-schema — v2 plus the ranked index keywords, and the gate the index parser reads
            should_add_creator_id: 1,
            enrich_with_base_schema: 1,
            find_identifier_and_binary_paths: 0,
            validate_max_depth: 0,
            max_depth: 256,
            recursive_schema_validator_versions: RecursiveSchemaValidatorVersions {
                traversal_validator: 0,
            },
            validate_schema_compatibility: 0,
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
