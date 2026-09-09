use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

#[derive(Clone, Debug, Default)]
pub struct DPPValidationVersions {
    pub json_schema_validator: JsonSchemaValidatorVersions,
    pub data_contract: DataContractValidationVersions,
    pub document_type: DocumentTypeValidationVersions,
    pub voting: VotingValidationVersions,
    pub validation_result: ValidationResultMethodVersions,
}

/// Versions of the aggregator methods on
/// [`crate::validation::ValidationResult`] (`flatten`, `merge_many`).
///
/// Issue #2867: in v0 the aggregators returned `Some(empty_vec)` when no
/// per-item input contributed any data, which caused
/// `validating-state-transition-for-free` — empty-action batches were treated
/// as paid (and stayed in the block) instead of unpaid (removed in
/// prepare_proposal). v1 returns `None` in that case so the result correctly
/// flows down the unpaid path.
#[derive(Clone, Debug, Default)]
pub struct ValidationResultMethodVersions {
    pub flatten: FeatureVersion,
    pub merge_many: FeatureVersion,
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
