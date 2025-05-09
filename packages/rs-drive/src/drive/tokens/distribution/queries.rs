use crate::drive::tokens::paths::{
    token_perpetual_distributions_identity_last_claimed_time_path_vec,
    token_pre_programmed_distributions_path_vec,
};
use crate::drive::Drive;
use crate::query::QueryItem;
use dpp::identifier::Identifier;
use dpp::prelude::{StartAtIncluded, TimestampMillis};
use grovedb::{PathQuery, Query, SizedQuery};

/// Defines the starting point for a query on pre-programmed token distributions.
///
/// # Usage
///
/// This struct is used to define a query range when retrieving pre-programmed distributions.
/// It allows queries to start at a specific point instead of always fetching the full dataset.
pub struct QueryPreProgrammedDistributionStartAt {
    /// The timestamp (in milliseconds) from which distributions should be queried.
    /// Only distributions at or after this time will be included.
    pub start_at_time: TimestampMillis,

    /// An optional recipient filter that specifies a recipient to start querying from within the given timestamp.
    ///
    /// - If `Some((recipient, StartAtIncluded::Yes))`, the query will start **from** this recipient.
    /// - If `Some((recipient, StartAtIncluded::No))`, the query will start **after** this recipient.
    /// - If `None`, the query will return all recipients from `start_at_time`.
    pub start_at_recipient: Option<(Identifier, StartAtIncluded)>,
}
impl Drive {
    /// Constructs a query that fetches the entire pre‑programmed distributions subtree for a token.
    ///
    /// The query uses the path returned by `token_pre_programmed_distributions_path_vec(token_id)` and
    /// inserts a full‑range query (i.e. `RangeFull`) so that all time keys are returned.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `limit`: An optional limit
    ///
    /// # Returns
    ///
    /// A `PathQuery` that, when executed, will return all entries under the token’s
    /// pre‑programmed distributions tree.
    pub fn pre_programmed_distributions_query(
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
    ) -> PathQuery {
        let path = token_pre_programmed_distributions_path_vec(token_id);
        let query = match start_at {
            None => {
                let mut query = Query::new_range_full();
                query.set_subquery(Query::new_range_full());
                query
            }
            Some(QueryPreProgrammedDistributionStartAt {
                start_at_time,
                start_at_recipient,
            }) => {
                let mut query = Query::new_single_query_item(QueryItem::RangeFrom(
                    start_at_time.to_be_bytes().to_vec()..,
                ));
                query.set_subquery(Query::new_range_full());
                if let Some((start_at_recipient, included)) = start_at_recipient {
                    let subquery = if included {
                        Query::new_single_query_item(QueryItem::RangeFrom(
                            start_at_recipient.to_vec()..,
                        ))
                    } else {
                        Query::new_single_query_item(QueryItem::RangeAfter(
                            start_at_recipient.to_vec()..,
                        ))
                    };
                    query.add_conditional_subquery(
                        QueryItem::Key(start_at_time.to_be_bytes().to_vec()),
                        None,
                        Some(subquery),
                    )
                }
                query
            }
        };

        PathQuery::new(path, SizedQuery::new(query, limit, None))
    }

    /// Constructs a `PathQuery` to retrieve the last perpetual distribution moment
    /// for a given identity under a specific token.
    ///
    /// This query targets the `token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id)`
    /// path in GroveDB and looks up the value associated with the `identity_id` key. The result is expected
    /// to be a single `Item` element containing raw bytes that encode the last claim information
    /// (e.g., a timestamp, block height, or other representation defined by the distribution type).
    ///
    /// # Parameters
    ///
    /// - `token_id`: A 32-byte identifier of the token whose distribution history is being queried.
    /// - `identity_id`: A 32-byte identifier of the identity whose last claim is being looked up.
    ///
    /// # Returns
    ///
    /// A `PathQuery` suitable for use in `grove_get`, `grove_get_raw`, or `grove_get_proved_path_query`,
    /// targeting a single element in the tree.
    ///
    /// This query is used when either retrieving the value directly or generating a proof
    /// for it using GroveDB’s query engine.
    ///
    pub fn perpetual_distribution_last_paid_moment_query(
        token_id: [u8; 32],
        identity_id: [u8; 32],
    ) -> PathQuery {
        let path = token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id);
        let query = Query::new_single_key(identity_id.to_vec());
        PathQuery::new(path, SizedQuery::new(query, Some(1), None))
    }
}
