use crate::version::drive_versions::drive_group_method_versions::{
    DriveGroupCostEstimationMethodVersions, DriveGroupFetchMethodVersions,
    DriveGroupInsertMethodVersions, DriveGroupMethodVersions, DriveGroupProveMethodVersions,
};

/// Drive group methods for protocol version 15.
///
/// Relative to [`super::v1::DRIVE_GROUP_METHOD_VERSIONS_V1`], `insert.add_group_action` is
/// bumped to `1`: when the caller passes no transaction, the fee-returning wrapper writes and
/// prices inside one owned transaction and commits it only once `Drive::calculate_fee`
/// succeeded. Pricing an owner-attributed storage removal without the fee history is an error
/// from this version, and generation 0 committed a closing action before that error surfaced.
/// The operation builders are unchanged.
pub const DRIVE_GROUP_METHOD_VERSIONS_V2: DriveGroupMethodVersions = DriveGroupMethodVersions {
    fetch: DriveGroupFetchMethodVersions {
        fetch_action_id_signers_power: 0,
        fetch_active_action_info: 0,
        fetch_action_id_info_keep_serialized: 0,
        fetch_action_id_has_signer: 0,
        fetch_group_info: 0,
        fetch_group_infos: 0,
        fetch_action_infos: 0,
        fetch_action_signers: 0,
        fetch_action_is_closed: 0,
    },
    prove: DriveGroupProveMethodVersions {
        prove_group_info: 0,
        prove_group_infos: 0,
        prove_action_infos: 0,
        prove_action_signers: 0,
    },
    insert: DriveGroupInsertMethodVersions {
        add_new_groups: 0,
        add_group_action: 1, // changed in v15: the fee-returning wrapper prices before it commits its owned transaction
    },
    cost_estimation: DriveGroupCostEstimationMethodVersions {
        for_add_group_action: 0,
        for_add_group: 0,
    },
};
