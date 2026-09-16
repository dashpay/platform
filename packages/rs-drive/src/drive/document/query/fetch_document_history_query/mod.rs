mod v0;
mod v1;

use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::document_history_drive_query::invalid;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl Drive {
    /// Creates a path query for historical entries of a specified document
    /// in the layout that predates protocol version 14.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_document_history_query(
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
        {
            0 => Self::fetch_document_history_query_v0(
                contract_id,
                document_type_name,
                document_id,
                start_at_ms,
                limit,
                offset,
            ),
            // The layout changed at protocol version 14: the page query is
            // built by `DocumentHistoryDriveQuery::construct_path_query`,
            // which carries the revision and cursor filters this signature
            // cannot express.
            1 => Err(invalid(
                "the document history layout changed at protocol version 14; use the history query",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    pub(crate) fn validate_document_history_limit(limit: Option<u16>) -> Result<u16, Error> {
        let limit = limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT);
        if !(1..=MAX_DOCUMENT_HISTORY_FETCH_LIMIT).contains(&limit) {
            return Err(Error::Drive(DriveError::InvalidDocumentHistoryFetchLimit(
                limit,
            )));
        }

        Ok(limit)
    }
}
