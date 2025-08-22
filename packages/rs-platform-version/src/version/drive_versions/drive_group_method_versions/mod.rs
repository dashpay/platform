use grovedb_version::version::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveGroupMethodVersions {
    pub fetch: DriveGroupFetchMethodVersions,
    pub prove: DriveGroupProveMethodVersions,
    pub insert: DriveGroupInsertMethodVersions,
    pub cost_estimation: DriveGroupCostEstimationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupFetchMethodVersions {
    pub fetch_action_id_signers_power: FeatureVersion,
    pub fetch_active_action_info: FeatureVersion,
    pub fetch_action_id_info_keep_serialized: FeatureVersion,
    pub fetch_action_id_has_signer: FeatureVersion,
    pub fetch_group_info: FeatureVersion,
    pub fetch_group_infos: FeatureVersion,
    pub fetch_action_infos: FeatureVersion,
    pub fetch_action_signers: FeatureVersion,
    pub fetch_action_is_closed: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupProveMethodVersions {
    pub prove_group_info: FeatureVersion,
    pub prove_group_infos: FeatureVersion,
    pub prove_action_infos: FeatureVersion,
    pub prove_action_signers: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupInsertMethodVersions {
    pub add_new_groups: FeatureVersion,
    pub add_group_action: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupCostEstimationMethodVersions {
    pub for_add_group_action: FeatureVersion,
    pub for_add_group: FeatureVersion,
}
