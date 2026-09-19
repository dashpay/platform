//! Versioned primary-key layout selection for document queries.

mod v0;
mod v1;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;

/// The value stored below a keep-history document's primary key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::query) enum PrimaryKeyPathQueryTarget {
    /// The primary key contains a subtree of timestamp-keyed revisions.
    LegacyHistoryTree,
    /// The primary key contains the current serialized document directly.
    CurrentDocument,
}

pub(in crate::query) fn primary_key_path_query_target(
    platform_version: &PlatformVersion,
) -> Result<PrimaryKeyPathQueryTarget, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .primary_key_path_query
    {
        0 => Ok(v0::primary_key_path_query_target_v0()),
        1 => Ok(v1::primary_key_path_query_target_v1()),
        received => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "primary_key_path_query".to_string(),
            known_versions: vec![0, 1],
            received,
        })),
    }
}
