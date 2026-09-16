mod v0;
mod v1;

use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl Drive {
    /// Creates a path query for historical entries of a specified document
    /// in the layout selected by the platform version.
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
            1 => Self::fetch_document_history_query_v1(
                contract_id,
                document_type_name,
                document_id,
                start_at_ms,
                limit,
                offset,
            ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::paths::document_history_path;

    #[test]
    fn should_keep_legacy_query_builder_available_after_layout_change() {
        let contract_id = [1; 32];
        let document_id = [2; 32];
        let document_type_name = "note";
        let path_query = Drive::fetch_document_history_query(
            contract_id,
            document_type_name,
            document_id,
            1_000,
            Some(3),
            Some(2),
            PlatformVersion::get(14).expect("protocol 14"),
        )
        .expect("the legacy-shaped query is supported by the new layout");

        assert_eq!(
            path_query.path,
            document_history_path(&contract_id, document_type_name, &document_id)
        );
        assert_eq!(path_query.query.limit, Some(3));
        assert_eq!(path_query.query.offset, Some(2));
    }
}
