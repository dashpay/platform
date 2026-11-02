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
    ///
    /// # Parameters
    ///
    /// * `document_type_path`: The path of the document type's tree the query starts from.
    /// * `starts_at_document`: The cursor document and whether it is included (`startAt`) or
    ///   excluded (`startAfter`), if any.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(PathQuery)` walking the index the query picks, paginated from the cursor if any.
    /// * `Err(Error)` when the method version is unknown, the query has more than one `In`
    ///   clause, no index fits the clauses or they do not cover its prefix contiguously, or a
    ///   value does not serialize.
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
