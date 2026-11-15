use crate::drive::document::paths::{
    contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY, DOCUMENT_LIFECYCLE_TREE_KEY,
};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::primary_key_path_query::{
    primary_key_path_query_target, PrimaryKeyPathQueryTarget,
};
use crate::query::{Query, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::common::encode::encode_u64;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

/// The last time a document's history can order. A time is stored with its top bit flipped, so
/// a time past this one would sort below every time before it.
const LAST_ORDERED_TIME_MS: u64 = i64::MAX as u64;

impl SingleDocumentDriveQuery {
    /// Generation 0, except for two lookups in a document type that keeps history.
    ///
    /// A lookup at a block time walks the retained revisions in descending order,
    /// answering with the newest revision stored at or before that time, or absence.
    /// The legacy layout instead stops above its latest-version pointer. A released
    /// producer never combines this generation with the legacy layout; that combination
    /// preserves the legacy semantics for clients comparing layouts.
    ///
    /// The lookup of a document that may be contested states both branch caps itself
    /// for merge generation 2, giving the same merged bytes as GroveDB merge generation 1's
    /// derived cap.
    ///
    /// Such a lookup at a block time is refused. A contest's area keeps no history: it holds
    /// each contender as it was submitted, with no time to compare, so a document submitted
    /// after the block time would be found there.
    #[inline(always)]
    pub(super) fn construct_path_query_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match self.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => {
                self.construct_non_contested_path_query_v1(platform_version)
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => {
                if self.document_type_keeps_history && self.block_time_ms.is_some() {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(
                        "a document with history that may be contested can not be looked up at \
                         a block time"
                            .to_string(),
                    )));
                }
                let mut non_contested =
                    self.construct_non_contested_path_query_v1(platform_version)?;
                let mut contested = self.construct_contested_path_query(true);
                // Only this merge generation requires this normalization; future
                // generations must define their own cap semantics.
                if platform_version
                    .drive
                    .grove_version
                    .grovedb_versions
                    .path_query_methods
                    .merge
                    == 2
                {
                    for selection in [&mut non_contested, &mut contested] {
                        if let Some(global_limit) = selection.query.limit.take() {
                            selection.query.query.limit = Some(
                                selection
                                    .query
                                    .query
                                    .limit
                                    .map_or(global_limit, |limit| limit.min(global_limit)),
                            );
                        }
                    }
                }
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

    /// Generation 0's query of the document among the documents of its type, with the range of
    /// a lookup at a block time replaced.
    fn construct_non_contested_path_query_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.is_lifecycle_temporal_query(platform_version)? {
            let mut lifecycle_path =
                contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
            lifecycle_path.push(vec![DOCUMENT_LIFECYCLE_TREE_KEY]);
            let mut lifecycle = Query::new();
            lifecycle.insert_key(self.document_id.to_vec());
            let lifecycle = PathQuery::new(lifecycle_path, SizedQuery::new(lifecycle, None, None));

            let mut history_path =
                contract_document_type_path_vec(&self.contract_id, &self.document_type_name);
            history_path.push(vec![DOCUMENT_HISTORY_TREE_KEY]);
            let mut document = Query::new();
            document.insert_key(self.document_id.to_vec());
            let mut revisions = Query::new_with_direction(false);
            let mut upper = encode_u64(
                self.block_time_ms
                    .unwrap_or_default()
                    .min(LAST_ORDERED_TIME_MS),
            );
            upper.extend([0xff; 8]);
            revisions.insert_range_inclusive(encode_u64(0)..=upper);
            // The cap belongs to the history leaf. Ancestor caps would also limit
            // the lifecycle evidence when this query joins a larger proof.
            revisions.limit = Some(1);
            document.set_subquery(revisions);
            let history = PathQuery::new(history_path, SizedQuery::new(document, None, None));
            return PathQuery::merge(
                vec![&lifecycle, &history],
                &platform_version.drive.grove_version,
            )
            .map_err(Error::from);
        }
        let mut path_query = self.construct_non_contested_path_query(true, platform_version)?;
        if self.document_type_keeps_history {
            if let Some(block_time_ms) = self.block_time_ms {
                let mut versions_up_to_block_time = Query::new_with_direction(false);
                let upper = encode_u64(block_time_ms.min(LAST_ORDERED_TIME_MS));
                // Stop above the legacy latest-version pointer, which must not
                // answer a time before the first retained version.
                versions_up_to_block_time.insert_range_after_to_inclusive(vec![0]..=upper);
                path_query
                    .query
                    .query
                    .set_subquery(versions_up_to_block_time);
            }
        }
        Ok(path_query)
    }
    /// Whether this lookup needs authenticated lifecycle evidence as well as a revision.
    pub(crate) fn is_lifecycle_temporal_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        Ok(
            self.contested_status == SingleDocumentDriveQueryContestedStatus::NotContested
                && self.document_type_keeps_history
                && self.block_time_ms.is_some()
                && primary_key_path_query_target(platform_version)?
                    == PrimaryKeyPathQueryTarget::CurrentDocument,
        )
    }
}
