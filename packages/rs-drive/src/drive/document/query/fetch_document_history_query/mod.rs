use crate::drive::document::history::{invalid, DocumentHistoryQueryV1};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl Drive {
    /// Builds the entries path query of a historical document read, through
    /// the method version the protocol selects.
    pub fn fetch_document_history_query(
        query: &DocumentHistoryQueryV1,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
        {
            1 => query.entries_query_v1(),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_query".to_owned(),
                known_versions: vec![1],
                received: version,
            })),
        }
    }
}
