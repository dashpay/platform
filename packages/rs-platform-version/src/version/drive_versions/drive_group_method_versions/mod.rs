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
    pub fetch_action_id_info: FeatureVersion,
    pub fetch_action_id_info_keep_serialized: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupProveMethodVersions {}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupInsertMethodVersions {
    pub add_new_groups: FeatureVersion,
    pub add_group_action: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroupCostEstimationMethodVersions {}
