use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciMethodVersions, DriveAbciVotingMethodVersions,
};

/// Drive ABCI method versions 11. Introduced in protocol version 17 (5.0).
///
/// One slot changes over `DRIVE_ABCI_METHOD_VERSIONS_V10`:
/// `voting.check_for_ended_vote_polls` becomes 2. Generation 2 of the
/// per-block finalization event no longer selects the winner itself: for
/// every ended poll it calls `Drive::award_contested_document_vote_poll`,
/// which re-derives the winner from state with generation 1's rule and
/// inserts the winning document in one operation, then records the finalized
/// poll and cleans up on the same block transaction exactly as generation 1
/// did.
///
/// Everything else matches v10.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    voting: DriveAbciVotingMethodVersions {
        check_for_ended_vote_polls: 2, // changed in v17: the winner is selected and awarded inside Drive
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.voting
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
