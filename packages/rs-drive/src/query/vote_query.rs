use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_for_identity_vec,
    vote_decisions_identity_votes_tree_path_for_identity_vec,
};
use crate::error::Error;
use crate::query::Query;
use bincode::{Decode, Encode};
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::VotePoll;
use grovedb::{PathQuery, SizedQuery};

/// Vote Drive Query struct
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct IdentityBasedVoteDriveQuery {
    /// The identity who would have made the vote
    pub identity_id: Identifier,
    /// What vote poll are we asking for?
    pub vote_poll: VotePoll,
}

impl IdentityBasedVoteDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> Result<PathQuery, Error> {
        // Each poll kind indexes the votes of a masternode under its own branch
        let path = match &self.vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(_) => {
                vote_contested_resource_identity_votes_tree_path_for_identity_vec(
                    self.identity_id.as_bytes(),
                )
            }
            VotePoll::YesNoVotePoll(_) => vote_decisions_identity_votes_tree_path_for_identity_vec(
                self.identity_id.as_bytes(),
            ),
        };

        let vote_id = self.vote_poll.unique_id()?;

        let mut query = Query::new();
        query.insert_key(vote_id.to_vec());

        Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
    }
}
