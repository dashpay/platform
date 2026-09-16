//! Versioned lowering for document queries with at most one
//! non-primary-key `In` clause, dispatched by
//! `DriveDocumentQueryMethodVersions.non_primary_key_single_in_path_query`.
//! Only reachable through the v1+ non-primary-key lowering: the v0
//! lowering predates this method and carries its own frozen single-`In`
//! construction.

mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Lowers a query with at most one `In` clause into a path query.
    pub(in crate::query) fn get_non_primary_key_single_in_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .non_primary_key_single_in_path_query
        {
            0 => self.get_non_primary_key_single_in_path_query_v0(
                document_type_path,
                starts_at_document,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentQuery::get_non_primary_key_single_in_path_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
