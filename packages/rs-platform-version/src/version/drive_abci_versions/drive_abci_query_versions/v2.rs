use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciDocumentQueryHelperVersions, DriveAbciQueryVersions,
};

/// Version 2 of the Drive ABCI query versions.
///
/// Differs from v1 in exactly one slot:
/// `document_query_helpers.compute_aggregate_mode_and_check_limit` is 1
/// rather than 0. That is the ranked-routing gate. The v0 helper has no
/// ranked path at all; the v1 helper routes a grouped aggregate whose
/// single `order_by` names the selected aggregate (`ORDER BY <agg>
/// [ASC|DESC] LIMIT n [OFFSET m]`) to the ranked executor, and otherwise
/// behaves exactly as v0. Both reject every non-empty `having` with
/// "HAVING clause is not yet implemented".
///
/// Keeping the flip in v14's own table is what lets a mixed-version network
/// agree: protocol version 13 and earlier keep the v1 table, so those nodes
/// still reject ranked queries, while v14 nodes answer them. The wire
/// surface is unchanged — `document_query` stays at v1 because
/// `GetDocumentsRequestV1` already carries `selects` / `group_by` /
/// `having`, and the ranked *response* shape is an additive
/// `ResultData.ranked` variant older clients simply never receive.
pub const DRIVE_ABCI_QUERY_VERSIONS_V2: DriveAbciQueryVersions = DriveAbciQueryVersions {
    document_query_helpers: DriveAbciDocumentQueryHelperVersions {
        compute_aggregate_mode_and_check_limit: 1,
    },
    ..DRIVE_ABCI_QUERY_VERSIONS_V1
};
