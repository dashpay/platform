use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciDocumentQueryHelperVersions, DriveAbciQueryVersions,
};

/// Version 3 of the Drive ABCI query versions.
///
/// Differs from v2 in exactly one slot:
/// `document_query_helpers.compute_aggregate_mode_and_check_limit` is 2
/// rather than 1. That is the boolean-`HAVING` routing gate. The v1
/// helper rejects every non-empty `having` ("HAVING clause is not yet
/// implemented"); the v2 helper routes a grouped aggregate carrying
/// exactly one `having` clause (`GROUP BY p HAVING <agg> <op> <value>
/// LIMIT n`) to the having-range executor, which serves it as a
/// value-bounded range read of the covering ranked index's axis
/// secondary. Everything else — including multi-clause `having` and
/// `having` on a select with no ranked axis — keeps the v1 behavior.
///
/// Mixed-network safety comes from the shipped tables: protocol
/// versions 1–11 select `DRIVE_ABCI_QUERY_VERSIONS_V0`, and versions
/// 12–13 select `DRIVE_ABCI_QUERY_VERSIONS_V1`. Both use helper
/// version 0 and reject ranked and `HAVING` shapes, so nodes agree
/// until the PV14 upgrade carries. The wire surface is unchanged —
/// `GetDocumentsRequestV1.having` has been wire-stable since the v1
/// document query, and the response reuses the additive
/// `ResultData.ranked` entries shape (with `skipped` unset, since a
/// range page has no rank base).
pub const DRIVE_ABCI_QUERY_VERSIONS_V3: DriveAbciQueryVersions = DriveAbciQueryVersions {
    document_query_helpers: DriveAbciDocumentQueryHelperVersions {
        compute_aggregate_mode_and_check_limit: 2,
    },
    ..DRIVE_ABCI_QUERY_VERSIONS_V2
};
