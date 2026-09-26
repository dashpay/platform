use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentInsertContestedMethodVersions, DriveDocumentMethodVersions,
};

/// V5 is protocol version 17's document-method table.
///
/// One slot changes over `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`:
/// `insert_contested.award_contested_document_vote_poll` becomes `Some(0)`,
/// activating `Drive::award_contested_document_vote_poll`. The operation is
/// how the block executor finalizes an ended contested resource vote poll
/// from this version on: it takes no contender, checks that the poll's
/// stored status is `Started` and that its end-date queue entry exists at an
/// end date the block time has reached, tallies the votes, applies the
/// native tie-break and inserts the stored bytes of the winner, all in one
/// Drive call. Any caller that names a wrong end date, a poll that has not
/// ended, or a poll already finalized gets a typed rejection with no
/// operation applied.
///
/// The slot is `None` on V1 to V4, so a table of an earlier protocol version
/// cannot dispatch the operation at all. Everything else matches V4.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        insert_contested: DriveDocumentInsertContestedMethodVersions {
            award_contested_document_vote_poll: Some(0), // new in v17: the native award operation
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert_contested
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
