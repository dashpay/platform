use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeIndexVersions, DocumentTypeMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions, RecursiveSchemaValidatorVersions, TokenVersions,
};
use versioned_feature_core::FeatureVersionBounds;

// Introduced in protocol version 10, system_properties are changed to allow to make indexes on $creatorId
pub const CONTRACT_VERSIONS_V3: DPPContractVersions = DPPContractVersions {
    max_serialized_size: 65000,
    contract_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 1,
        default_current_version: 1,
    },
    contract_structure_version: 1,
    created_data_contract_structure: 0,
    config: FeatureVersionBounds {
        min_version: 0,
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
            try_from_schema: 1,
            create_document_types_from_document_schemas: 1,
        },
        structure_version: 0,
        schema: DocumentTypeSchemaVersions {
            should_add_creator_id: 1, //changed
            enrich_with_base_schema: 0,
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
