use dpp::dashcore::{ProTxHash, Txid};
use dpp::dashcore_rpc::json::MasternodeType;

/// The masternode accessors for version 0
pub trait MasternodeAccessorsV0 {
    /// The node type, for now either a masternode or an evonode
    fn node_type(&self) -> MasternodeType;
    /// A unique hash representing the masternode's registration transaction.
    fn pro_tx_hash(&self) -> ProTxHash;
    /// A unique hash representing the collateral transaction.
    fn collateral_hash(&self) -> Txid;
    /// The index of the collateral transaction output.
    fn collateral_index(&self) -> u32;
    /// The address where the collateral is stored.
    fn collateral_address(&self) -> [u8; 20];
    /// The amount of the operator's reward for running the masternode.
    fn operator_reward(&self) -> f32;
}
