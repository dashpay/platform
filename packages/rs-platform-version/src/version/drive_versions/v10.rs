use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{DriveMethodVersions, DriveVersion};

/// Drive version 10.
/// Introduced in protocol v17, the 5.0 protocol version.
///
/// * **Native contested award operation**: `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`
///   turns on `insert_contested.award_contested_document_vote_poll`, the one
///   Drive operation that finalizes an ended contested resource vote poll
///   from this version on. It takes no contender: the poll's stored status,
///   its end-date queue entry, the vote tallies and the stored contender
///   bytes are all read from state inside the operation, so no caller can
///   name a winner, award before the end date or award a poll twice. The
///   slot is `None` on every earlier table, so no historical protocol
///   version can dispatch it.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5, // changed in v10: the native contested award operation
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
