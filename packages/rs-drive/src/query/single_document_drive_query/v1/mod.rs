use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{Query, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::common::encode::encode_u64;
use grovedb::PathQuery;
use platform_version::version::PlatformVersion;

/// The last time a document's history can order. A time is stored with its top bit flipped, so
/// a time past this one would sort below every time before it.
const LAST_ORDERED_TIME_MS: u64 = i64::MAX as u64;

impl SingleDocumentDriveQuery {
    /// Generation 0, except for two lookups in a document type that keeps history.
    ///
    /// A lookup at a block time stops above the pointer to the latest version, so it answers
    /// with the newest version stored at or before that time, and with nothing for a time
    /// before the document was first stored.
    ///
    /// The lookup of a document that may be contested states its cap itself. Its history lookup
    /// has a subquery, above which GroveDB's merge refuses an overall limit from its generation
    /// 2 (`GROVE_V5`, which protocol version 15 is the first to select), where the generation
    /// before turned that limit into the cap of the merged branch. Written as that cap here, it
    /// merges to the query generation 0 built.
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
                Ok(self.construct_non_contested_path_query_v1())
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => {
                if self.document_type_keeps_history && self.block_time_ms.is_some() {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(
                        "a document with history that may be contested can not be looked up at \
                         a block time"
                            .to_string(),
                    )));
                }
                let mut non_contested = self.construct_non_contested_path_query_v1();
                let contested = self.construct_contested_path_query(true);
                if self.document_type_keeps_history {
                    non_contested.query.query.limit = non_contested.query.limit.take();
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
    fn construct_non_contested_path_query_v1(&self) -> PathQuery {
        let mut path_query = self.construct_non_contested_path_query(true);
        if self.document_type_keeps_history {
            if let Some(block_time_ms) = self.block_time_ms {
                // A document's history holds one key per time a version was stored, and the
                // pointer to the latest version at key `[0]`, below all of them. Walking down
                // from the block time and stopping above the pointer, the first key met is the
                // newest version stored at or before that time, and there is none when the
                // document was first stored later.
                let mut versions_up_to_block_time = Query::new_with_direction(false);
                versions_up_to_block_time.insert_range_after_to_inclusive(
                    vec![0]..=encode_u64(block_time_ms.min(LAST_ORDERED_TIME_MS)),
                );
                path_query
                    .query
                    .query
                    .set_subquery(versions_up_to_block_time);
            }
        }
        path_query
    }
}
