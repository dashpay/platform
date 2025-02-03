use crate::drive::tokens::paths::token_pre_programmed_distributions_path_vec;
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
