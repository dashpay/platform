mod transformer;

use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::voting::votes::Vote;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct MasternodeVoteTransitionActionV0 {
    /// the pro tx hash identifier of the masternode
    pub pro_tx_hash: Identifier,
    /// the resource votes
    pub vote: Vote,
    /// nonce
    pub nonce: IdentityNonce,
}
