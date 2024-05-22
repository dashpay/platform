mod transformer;

use crate::drive::votes::resolved::votes::ResolvedVote;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;

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
