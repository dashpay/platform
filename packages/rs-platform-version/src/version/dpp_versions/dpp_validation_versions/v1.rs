use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DataContractValidationVersions, DocumentTypeValidationVersions,
    JsonSchemaValidatorVersions, VotingValidationVersions,
};

pub const DPP_VALIDATION_VERSIONS_V1: DPPValidationVersions = DPPValidationVersions {
    json_schema_validator: JsonSchemaValidatorVersions {
        new: 0,
        validate: 0,
        compile: 0,
        compile_and_validate: 0,
    },
    data_contract: DataContractValidationVersions {
        validate: 0,
        validate_config_update: 0,
        validate_token_config_update: 0,
        validate_index_definitions: 0,
        validate_index_naming_duplicates: 0,
        validate_not_defined_properties: 0,
        validate_property_definition: 0,
        validate_token_config_groups_exist: 0,
        validate_localizations: 0,
    },
    document_type: DocumentTypeValidationVersions {
        validate_update: 0,
        contested_index_limit: 1,
        unique_index_limit: 10,
    },
    voting: VotingValidationVersions {
        allow_other_contenders_time_mainnet_ms: 604_800_000, // 1 week in ms
        allow_other_contenders_time_testing_ms: 604_800_000, // 1 week in ms for v1 (changes in v2)
        votes_allowed_per_masternode: 5,
    },
};
