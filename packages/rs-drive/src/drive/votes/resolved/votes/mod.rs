pub(crate) mod resolve;
/// Resolved resource vote module
pub mod resolved_resource_vote;

use crate::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
use dpp::identifier::Identifier;
use dpp::ProtocolError;

/// Represents the different types of resolved votes within the system.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedVote {
    /// A resolved vote for a specific resource.
    ResolvedResourceVote(ResolvedResourceVote),
}

impl ResolvedVote {
    /// Retrieves the specialized balance identifier associated with the resolved vote.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(identifier))` if a specialized balance ID is available.
    /// * `Ok(None)` if no specialized balance ID is associated.
    /// * `Err(ProtocolError)` if there is an error retrieving the balance ID.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an issue retrieving the specialized balance ID.
    pub fn specialized_balance_id(&self) -> Result<Option<Identifier>, ProtocolError> {
        match self {
            ResolvedVote::ResolvedResourceVote(resource_vote) => {
                resource_vote.vote_poll().specialized_balance_id()
            }
        }
    }
}
