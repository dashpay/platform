use crate::platform_types::masternode::v0::accessors::MasternodeAccessorsV0;
use crate::platform_types::masternode::Masternode;
use dpp::dashcore::{ProTxHash, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;

impl MasternodeAccessorsV0 for Masternode {
    fn node_type(&self) -> MasternodeType {
        match self {
            Masternode::V0(v0) => v0.node_type.clone(), //todo(copy)
        }
    }

    fn pro_tx_hash(&self) -> ProTxHash {
        match self {
            Masternode::V0(v0) => v0.pro_tx_hash,
        }
    }

    fn collateral_hash(&self) -> Txid {
        match self {
            Masternode::V0(v0) => v0.collateral_hash,
        }
    }

    fn collateral_index(&self) -> u32 {
        match self {
            Masternode::V0(v0) => v0.collateral_index,
        }
    }

    fn collateral_address(&self) -> [u8; 20] {
        match self {
            Masternode::V0(v0) => v0.collateral_address,
        }
    }

    fn operator_reward(&self) -> f32 {
        match self {
            Masternode::V0(v0) => v0.operator_reward,
        }
    }
}
