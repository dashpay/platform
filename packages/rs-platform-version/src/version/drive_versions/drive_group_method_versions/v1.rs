use crate::version::drive_versions::drive_group_method_versions::{
    DriveGroupCostEstimationMethodVersions, DriveGroupFetchMethodVersions,
    DriveGroupInsertMethodVersions, DriveGroupMethodVersions, DriveGroupProveMethodVersions,
};

pub const DRIVE_GROUP_METHOD_VERSIONS_V1: DriveGroupMethodVersions = DriveGroupMethodVersions {
    fetch: DriveGroupFetchMethodVersions {
        fetch_action_id_signers_power: 0,
    },
    prove: DriveGroupProveMethodVersions {},
    insert: DriveGroupInsertMethodVersions {
        add_new_groups: 0,
        add_group_action: 0,
    },
    cost_estimation: DriveGroupCostEstimationMethodVersions {},
};
