use crate::version::drive_versions::drive_vote_method_versions::v3::DRIVE_VOTE_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_vote_method_versions::{
    DriveVoteMethodVersions, DriveVoteReadinessMethodVersions, DriveVoteSetupMethodVersions,
};

/// Introduced in protocol version 17 (5.0) for compilation readiness.
///
/// Changes over [`DRIVE_VOTE_METHOD_VERSIONS_V3`]:
///
/// * `setup.add_initial_vote_tree_main_structure_operations` 0 -> 1: genesis creates the
///   readiness subtree `[Votes] / r` with its four children and the readiness fund sum tree
///   `[PreFundedSpecializedBalances] / 129` beside the voting fund tree. The fund tree is
///   created here rather than by the unversioned prefunded helper because genesis builds every
///   lower layer in one batch and this dispatcher has its own slot.
/// * the `readiness` method group turns on at generation 0.
pub const DRIVE_VOTE_METHOD_VERSIONS_V4: DriveVoteMethodVersions = DriveVoteMethodVersions {
    setup: DriveVoteSetupMethodVersions {
        add_initial_vote_tree_main_structure_operations: 1, // changed in v17: readiness subtree and fund tree at genesis
    },
    readiness: DriveVoteReadinessMethodVersions {
        open_round: Some(0),
        cancel_round: Some(0),
        retire_round: Some(0),
        insert_report: Some(0),
        fetch_round: Some(0),
        fetch_raw_count: Some(0),
        fetch_reports_page: Some(0),
        fetch_rounds_page: Some(0),
        store_scan_cursor: Some(0),
        fetch_scan_cursor: Some(0),
        clear_scan_cursor: Some(0),
        update_round_evaluation: Some(0),
        record_crossing: Some(0),
        fetch_rounds_due: Some(0),
        activate_round: Some(0),
        prune_reports: Some(0),
        fetch_retired_round: Some(0),
        cleanup_retired_round: Some(0),
        store_evaluation_cursor: Some(0),
        fetch_evaluation_cursor: Some(0),
        add_fund: Some(0),
        deduct_from_fund: Some(0),
        empty_fund: Some(0),
        fetch_fund: Some(0),
        prove_fund: Some(0),
        estimation_costs: Some(0),
    },
    ..DRIVE_VOTE_METHOD_VERSIONS_V3
};
