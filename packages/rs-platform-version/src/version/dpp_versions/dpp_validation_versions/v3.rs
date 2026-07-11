use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DataContractValidationVersions, DocumentTypeValidationVersions,
    JsonSchemaValidatorVersions, ValidationResultMethodVersions, VotingValidationVersions,
};

pub const DPP_VALIDATION_VERSIONS_V3: DPPValidationVersions = DPPValidationVersions {
    json_schema_validator: JsonSchemaValidatorVersions {
        new: 0,
        validate: 0,
        compile: 0,
        compile_and_validate: 0,
    },
    data_contract: DataContractValidationVersions {
        validate: 0,
        // prevent sized_integer_types config downgrade on contract update
        validate_config_update: 1,
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
        allow_other_contenders_time_testing_ms: 2_700_000,   //45 minutes
        votes_allowed_per_masternode: 5,
    },
    // Issue #2867: bump aggregator methods to v1 — `flatten` / `merge_many`
    // now return `data: None` when no input contributed any data, instead of
    // the legacy `Some(empty_vec)`. Closes the
    // "validating-state-transition-for-free" gap where an all-failed
    // documents batch was being recorded as PaidConsensusError with an empty
    // action and the same exact bytes could be replayed across blocks.
    validation_result: ValidationResultMethodVersions {
        flatten: 1,
        merge_many: 1,
    },
};
