use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::error::Error;
use crate::query::masternode_yes_no_vote_query::MasternodeYesNoVoteDriveQuery;
use crate::query::Query;
use bincode::{Decode, Encode};
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::VotePoll;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

/// Vote Drive Query struct
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct IdentityBasedVoteDriveQuery {
    /// The identity who would have made the vote
    pub identity_id: Identifier,
    /// What vote poll are we asking for?
    pub vote_poll: VotePoll,
}

impl IdentityBasedVoteDriveQuery {
    /// Operations to construct a path query. The platform version selects how a yes/no poll's
    /// id is derived.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // Each poll kind indexes the votes of a masternode under its own branch
        match &self.vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(_) => {
                let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
                    self.identity_id.as_bytes(),
                );
                let vote_id = self.vote_poll.unique_id(platform_version)?;

                let mut query = Query::new();
                query.insert_key(vote_id.to_vec());

                Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
            }
            VotePoll::YesNoVotePoll(vote_poll) => Ok(MasternodeYesNoVoteDriveQuery {
                pro_tx_hash: self.identity_id,
                vote_poll_id: vote_poll.unique_id(platform_version)?,
            }
            .construct_path_query()),
        }
    }
}
