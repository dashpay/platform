use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::error::Error;
use crate::query::Query;
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::VotePoll;
use grovedb::{PathQuery, SizedQuery};

/// Vote Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct IdentityBasedVoteDriveQuery {
    /// The identity who would have made the vote
    pub identity_id: Identifier,
    /// What vote poll are we asking for?
    pub vote_poll: VotePoll,
}

impl IdentityBasedVoteDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> Result<PathQuery, Error> {
        // First we should get the overall document_type_path
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            self.identity_id.as_bytes(),
        );

        let vote_id = self.vote_poll.unique_id()?;

        let mut query = Query::new();
        query.insert_key(vote_id.to_vec());

        Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
    }
}
