use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_query_versions::{
    DriveAbciDataContractQueryHelperVersions, DriveAbciDocumentQueryHelperVersions,
    DriveAbciQueryIdentityVersions, DriveAbciQueryVersions,
};
use versioned_feature_core::FeatureVersionBounds;

/// Version 2 of the Drive ABCI query versions, selected by protocol version 14.
///
/// Differs from v1 in three slots.
///
/// `document_query_helpers.compute_aggregate_mode_and_check_limit` is 2
/// rather than 0. That opens two routes on the v1 document-query handler:
/// the ranked path, a grouped aggregate whose single `order_by` names the
/// selected aggregate (`ORDER BY <agg> [ASC|DESC] LIMIT n [OFFSET m]`),
/// served by the ranked executor; and the boolean-`HAVING` range path, a
/// grouped aggregate carrying exactly one `having` clause
/// (`GROUP BY p HAVING <agg> <op> <value> LIMIT n`), served as a
/// value-bounded range read of the covering ranked index's axis secondary.
/// Everything else, including multi-clause `having` and `having` on a
/// select with no ranked axis, is rejected as before ("HAVING clause is not
/// yet implemented").
///
/// `data_contract_query_helpers.latest_versions_read` is 1 rather than 0:
/// from protocol version 14 every contract carries a four-byte version item
/// beside it (`[64, id, 2] / 64`, backfilled on the first block of the version),
/// so `getDataContractsLatestVersions` without `include_contracts` reads that
/// item and proves it instead of the contracts. The tables protocol versions
/// 1 to 13 select keep helper version 0, which reads and proves the contracts
/// a state without the items still has.
///
/// Mixed-network safety comes from the shipped tables: protocol versions
/// 1-11 select `DRIVE_ABCI_QUERY_VERSIONS_V0` and 12-13 select
/// `DRIVE_ABCI_QUERY_VERSIONS_V1`. Both use helper version 0 and reject
/// ranked and `HAVING` shapes, so nodes agree until the protocol version 14
/// upgrade carries. For documents the wire surface is unchanged: `document_query` stays at
/// v1 because `GetDocumentsRequestV1` already carries `selects` / `group_by`
/// / `having`, and the ranked and range responses reuse the additive
/// `ResultData.ranked` entries shape (with `skipped` unset for a range page,
/// which has no rank base), which older clients never receive.
///
/// `identity_based_queries.identity_by_non_unique_public_key_hash` accepts
/// request V1 (`max_version` 1, default 1). A V1 request is answered with one
/// GroveDB proof of the index selection and the identity; V0 keeps its separate
/// index and identity proofs. Earlier tables accept only V0, as 4.1 nodes do, so
/// every node of a network below protocol 14 refuses V1 the same way.
/// V0 stays accepted (`min_version` 0) because a client sends V0 until it has
/// verified an answer signed by a version that accepts V1, and older clients
/// only know V0. Drop V0 only once no supported client sends it; keep V1 while
/// any supported client may still send it, since a client never learns a
/// version newer than its own build.
pub const DRIVE_ABCI_QUERY_VERSIONS_V2: DriveAbciQueryVersions = DriveAbciQueryVersions {
    identity_based_queries: DriveAbciQueryIdentityVersions {
        identity_by_non_unique_public_key_hash: FeatureVersionBounds {
            min_version: 0,
            max_version: 1,
            default_current_version: 1,
        },
        ..DRIVE_ABCI_QUERY_VERSIONS_V1.identity_based_queries
    },
    document_query_helpers: DriveAbciDocumentQueryHelperVersions {
        compute_aggregate_mode_and_check_limit: 2,
    },
    data_contract_query_helpers: DriveAbciDataContractQueryHelperVersions {
        latest_versions_read: 1,
    },
    ..DRIVE_ABCI_QUERY_VERSIONS_V1
};
