use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::votes;
use crate::error::Error;
use crate::query::{Query, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::common::encode::encode_u64;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// The query protocol versions 1 to 14 build.
    ///
    /// In a document type that keeps history, a lookup at a block time ranges over every key
    /// up to that time. That reaches the pointer to the latest version at key `[0]`, which
    /// sorts below every time, so a time before the document was first stored answers with the
    /// latest version. No proof of such a lookup verifies either: generation 0 of the verifier
    /// asks GroveDB to name the absent keys, which a range that is open at one end does not
    /// allow.
    #[inline(always)]
    pub(super) fn construct_path_query_v0(
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
                .map_err(Error::from)
            }
            SingleDocumentDriveQueryContestedStatus::Contested => {
                Ok(self.construct_contested_path_query(true))
            }
        }
    }

    /// Operations to construct the normal path query.
    pub(super) fn construct_non_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
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
    pub(super) fn construct_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
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
