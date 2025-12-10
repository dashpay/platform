use dpp::dashcore::ProTxHash;

/// Masternode list Changes
#[derive(Debug, Clone)]
pub struct MasternodeListChanges {
    /// The new masternodes
    pub new_masternodes: Vec<ProTxHash>,
    /// The removed masternodes
    pub removed_masternodes: Vec<ProTxHash>,
    /// The banned masternodes
    pub banned_masternodes: Vec<ProTxHash>,
    /// The unbanned masternodes
    pub unbanned_masternodes: Vec<ProTxHash>,
    /// the new masternodes that come in as banned
    pub new_banned_masternodes: Vec<ProTxHash>,
}
