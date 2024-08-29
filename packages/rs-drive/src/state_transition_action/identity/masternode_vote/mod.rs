/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::state_transition_action::identity::masternode_vote::v0::{
    MasternodeVoteTransitionActionV0, PreviousVoteCount,
};
use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

/// action
#[derive(Debug, Clone, From)]
pub enum MasternodeVoteTransitionAction {
    /// v0
    V0(MasternodeVoteTransitionActionV0),
}

impl MasternodeVoteTransitionAction {
    /// the pro tx hash identifier of the masternode
    pub fn pro_tx_hash(&self) -> Identifier {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.pro_tx_hash,
        }
    }

    /// the voter identity id
    pub fn voter_identity_id(&self) -> Identifier {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.voter_identity_id,
        }
    }

    /// the masternode list state based voting address
    pub fn voting_address(&self) -> [u8; 20] {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.voting_address,
        }
    }

    /// Resource votes
    pub fn vote_ref(&self) -> &ResolvedVote {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => &transition.vote,
        }
    }

    /// Resource votes as owned
    pub fn vote_owned(self) -> ResolvedVote {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.vote,
        }
    }

    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// Vote strength
    pub fn vote_strength(&self) -> u8 {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.vote_strength,
        }
    }

    /// The previous resource vote choice that needs to be removed
    pub fn take_previous_resource_vote_choice_to_remove(
        &mut self,
    ) -> Option<(ResourceVoteChoice, PreviousVoteCount)> {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => {
                transition.previous_resource_vote_choice_to_remove.take()
            }
        }
    }

    /// The previous resource vote choice that needs to be removed
    pub fn previous_resource_vote_choice_to_remove(
        &self,
    ) -> &Option<(ResourceVoteChoice, PreviousVoteCount)> {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => {
                &transition.previous_resource_vote_choice_to_remove
            }
        }
    }
}
