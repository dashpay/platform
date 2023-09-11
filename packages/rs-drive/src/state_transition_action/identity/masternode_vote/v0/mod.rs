mod transformer;

use dpp::platform_value::Identifier;
use dpp::voting::Vote;
use serde::{Deserialize, Serialize};

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasternodeVoteTransitionActionV0 {
    /// the pro tx hash identifier of the masternode
    pub pro_tx_hash: Identifier,
    /// the resource vote
    pub vote: Vote,
}
