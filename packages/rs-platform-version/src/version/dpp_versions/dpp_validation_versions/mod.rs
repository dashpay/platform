use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPValidationVersions {
    pub json_schema_validator: JsonSchemaValidatorVersions,
    pub data_contract: DataContractValidationVersions,
    pub document_type: DocumentTypeValidationVersions,
    pub voting: VotingValidationVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DataContractValidationVersions {
    pub validate: FeatureVersion,
    pub validate_config_update: FeatureVersion,
    pub validate_token_config_update: FeatureVersion,
    pub validate_index_definitions: FeatureVersion,
    pub validate_index_naming_duplicates: FeatureVersion,
    pub validate_not_defined_properties: FeatureVersion,
    pub validate_property_definition: FeatureVersion,
    pub validate_token_config_groups_exist: FeatureVersion,
    pub validate_localizations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct VotingValidationVersions {
    /// How long do we allow other contenders to join a contest after the first contender
    pub allow_other_contenders_time_mainnet_ms: u64,
    /// How long do we allow other contenders to join a contest after the first contender in a testing environment
    pub allow_other_contenders_time_testing_ms: u64,
    /// How many votes do we allow from the same masternode?
    pub votes_allowed_per_masternode: u16,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeValidationVersions {
    pub validate_update: FeatureVersion,
    pub unique_index_limit: u16,
    pub contested_index_limit: u16,
}

#[derive(Clone, Debug, Default)]
pub struct JsonSchemaValidatorVersions {
    pub new: FeatureVersion,
    pub validate: FeatureVersion,
    pub compile: FeatureVersion,
    pub compile_and_validate: FeatureVersion,
}
