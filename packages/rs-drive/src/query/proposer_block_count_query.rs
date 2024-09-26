use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::query::Query;
use dpp::block::epoch::Epoch;
use grovedb::{PathQuery, SizedQuery};

/// Represents an optional limit for the number of proposers to retrieve in a query.
///
/// - `Some(u16)`: Limits the number of proposers returned.
/// - `None`: No limit on the number of proposers.
pub type ProposerQueryLimit = Option<u16>;
/// Indicates whether the query should include the starting proposer in the results.
///
/// - `true`: The starting proposer is included in the results.
/// - `false`: The starting proposer is excluded.
pub type ProposerQueryStartAtIncluded = bool;
/// Represents an optional starting point for a proposer query, consisting of:
///
/// - A tuple of a 32-byte array representing the start proposer's identifier and
///   a boolean indicating whether to include the starting proposer.
///
/// - `Some(([u8; 32], bool))`: A specific proposer to start from, with an inclusion flag.
/// - `None`: The query will start from the beginning or a default point.
pub type ProposerQueryStartAt = Option<([u8; 32], ProposerQueryStartAtIncluded)>;

/// Specifies the type of query to retrieve proposers, with two options:
///
/// - `ByRange(ProposerQueryLimit, ProposerQueryStartAt)`: Query proposers within a range,
///   with an optional limit and an optional starting point.
///
/// - `ByIds(Vec<Vec<u8>>)`: Query specific proposers by their identifiers.
pub enum ProposerQueryType {
    /// Queries proposers within a specified range.
    ///
    /// - `ProposerQueryLimit`: Limits the number of proposers returned. If `None`, there is no limit.
    /// - `ProposerQueryStartAt`: Specifies the proposer to start from. If `None`, the query starts at the beginning.
    ByRange(ProposerQueryLimit, ProposerQueryStartAt),

    /// Queries specific proposers by their identifiers.
    ///
    /// - `Vec<Vec<u8>>`: A vector of proposer IDs (byte arrays) to retrieve.
    ByIds(Vec<Vec<u8>>),
}

impl ProposerQueryType {
    /// Should we get optional elements?
    pub fn allows_optional(&self) -> bool {
        match self {
            ProposerQueryType::ByRange(_, _) => false,
            ProposerQueryType::ByIds(_) => true,
        }
    }

    /// Gets the path query for the proposer query type
    pub fn into_path_query(self, epoch: &Epoch) -> PathQuery {
        let path_as_vec = epoch.get_proposers_path_vec();

        let mut query = Query::new();

        match self {
            ProposerQueryType::ByRange(limit, start_at) => {
                match start_at {
                    None => {
                        query.insert_all();
                    }
                    Some((identity_id, included)) => {
                        if included {
                            query.insert_range_from(identity_id.to_vec()..);
                        } else {
                            query.insert_range_after(identity_id.to_vec()..);
                        }
                    }
                }
                PathQuery::new(path_as_vec, SizedQuery::new(query, limit, None))
            }
            ProposerQueryType::ByIds(ids) => {
                let len = ids.len();
                query.insert_keys(ids);
                PathQuery::new(path_as_vec, SizedQuery::new(query, Some(len as u16), None))
            }
        }
    }
}
