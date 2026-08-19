//! v1 of the non-primary-key path-query lowering (protocol version 14):
//! routes by query shape to the versioned shape lowerings — multiple
//! `In` clauses on consecutive index properties go to
//! [`DriveDocumentQuery::get_non_primary_key_multiple_in_path_query`],
//! everything else to
//! [`DriveDocumentQuery::get_non_primary_key_single_in_path_query`].
//! Relative to v0 this accepts multiple `In` clauses, makes cursor
//! pagination over a multi-branch level sibling-branch correct, and
//! derives cursorless left-over directions from `order_by`; the actual
//! constructions live in the shape lowerings' version modules.

use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// v1 of the non-primary-key path query lowering (protocol version 14):
    /// routes by shape to the versioned single-`In` / multiple-`In`
    /// lowerings.
    pub(in crate::query) fn get_non_primary_key_path_query_v1(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.internal_clauses.in_clauses.len() > 1 {
            self.get_non_primary_key_multiple_in_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        } else {
            self.get_non_primary_key_single_in_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        }
    }
}
