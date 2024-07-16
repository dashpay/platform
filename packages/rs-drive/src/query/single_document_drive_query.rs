use crate::drive::document::paths::contract_document_type_path_vec;
use crate::util::common::encode::encode_u64;

use crate::drive::votes;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;
use platform_version::TryFromPlatformVersioned;

/// The expected contested status of a document
/// Drives stores the document in either the not contested location (most of the time)
/// Or a temporary contested area while the contest is ongoing
#[derive(Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum SingleDocumentDriveQueryContestedStatus {
    /// The document was not contested by the system.
    NotContested = 0,
    /// We don't know if the document was contested by the system, or we are not sure if the contest
    /// is already over or not.
    MaybeContested = 1,
    /// We know that the document was contested by the system and the contest is not over.
    Contested = 2,
}

impl TryFrom<i32> for SingleDocumentDriveQueryContestedStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SingleDocumentDriveQueryContestedStatus::NotContested),
            1 => Ok(SingleDocumentDriveQueryContestedStatus::MaybeContested),
            2 => Ok(SingleDocumentDriveQueryContestedStatus::Contested),
            n => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "unsupported contested status {}, only 0, 1 and 2 are supported",
                n
            )))),
        }
    }
}

/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct SingleDocumentDriveQuery {
    ///DataContract
    pub contract_id: [u8; 32],
    /// Document type
    pub document_type_name: String,
    /// Document type keeps history
    pub document_type_keeps_history: bool,
    /// Document
    pub document_id: [u8; 32],
    /// Block time
    pub block_time_ms: Option<u64>,
    /// True if the document might have gone to a contested resolution
    pub contested_status: SingleDocumentDriveQueryContestedStatus,
}

impl SingleDocumentDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match self.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => {
                Ok(self.construct_non_contested_path_query(true))
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => {
                let non_contested = self.construct_non_contested_path_query(true);
                let contested = self.construct_contested_path_query(true);
                PathQuery::merge(
                    vec![&non_contested, &contested],
                    &platform_version.drive.grove_version,
                )
                .map_err(Error::GroveDB)
            }
            SingleDocumentDriveQueryContestedStatus::Contested => {
                Ok(self.construct_contested_path_query(true))
            }
        }
    }

    /// Operations to construct the normal path query.
    fn construct_non_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
        // First we should get the overall document_type_path
        let mut path =
            contract_document_type_path_vec(&self.contract_id, self.document_type_name.as_str());

        path.push(vec![0]);

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        if self.document_type_keeps_history {
            // if the documents keep history then we should insert a subquery
            if let Some(block_time) = self.block_time_ms {
                let encoded_block_time = encode_u64(block_time);
                let mut sub_query = Query::new_with_direction(false);
                sub_query.insert_range_to_inclusive(..=encoded_block_time);
                query.set_subquery(sub_query);
            } else {
                query.set_subquery_key(vec![0]);
            }
        }

        let limit = if with_limit_1 { Some(1) } else { None };

        PathQuery::new(path, SizedQuery::new(query, limit, None))
    }

    /// Operations to construct the contested path query.
    fn construct_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
        // First we should get the overall document_type_path
        let path = votes::paths::vote_contested_resource_contract_documents_storage_path_vec(
            &self.contract_id,
            self.document_type_name.as_str(),
        );

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        let limit = if with_limit_1 { Some(1) } else { None };

        PathQuery::new(path, SizedQuery::new(query, limit, None))
    }
}

impl TryFromPlatformVersioned<SingleDocumentDriveQuery> for PathQuery {
    type Error = Error;
    fn try_from_platform_versioned(
        value: SingleDocumentDriveQuery,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        value.construct_path_query(platform_version)
    }
}
