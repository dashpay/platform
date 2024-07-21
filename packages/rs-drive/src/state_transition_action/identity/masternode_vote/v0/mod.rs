mod transformer;

use crate::drive::votes::resolved::votes::ResolvedVote;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

/// The previous vote count
pub type PreviousVoteCount = u16;

/// action v0
#[derive(Debug, Clone)]
pub struct MasternodeVoteTransitionActionV0 {
    /// the pro tx hash identifier of the masternode
    pub pro_tx_hash: Identifier,
    /// The voter identity id is made by hashing the pro_tx_hash and the voting address
    pub voter_identity_id: Identifier,
    /// The voting address used
    pub voting_address: [u8; 20],
    /// masternode type vote strength, masternodes have 1, evonodes have 4
    pub vote_strength: u8,
    /// the resource votes
    pub vote: ResolvedVote,
    /// vote choice to remove
    pub previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
    /// nonce
    pub nonce: IdentityNonce,
}
