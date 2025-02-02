use crate::drive::tokens::paths::token_pre_programmed_distributions_path_vec;
use grovedb::{PathQuery, Query, SizedQuery};

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
pub fn pre_programmed_distributions_query(token_id: [u8; 32], limit: Option<u16>) -> PathQuery {
    let path = token_pre_programmed_distributions_path_vec(token_id);
    let mut query = Query::new_range_full();
    query.set_subquery(Query::new_range_full());
    PathQuery::new(path, SizedQuery::new(query, limit, None))
}
