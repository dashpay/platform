mod transformer;

use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use crate::drive::votes::resolved::votes::ResolvedVote;

/// action v0
#[derive(Debug, Clone)]
pub struct MasternodeVoteTransitionActionV0 {
    /// the pro tx hash identifier of the masternode
    pub pro_tx_hash: Identifier,
    /// the resource votes
    pub vote: ResolvedVote,
    /// nonce
    pub nonce: IdentityNonce,
}
