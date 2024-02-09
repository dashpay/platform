use std::fmt::Display;

use dashcore_rpc::dashcore::PrivateKey;
use dpp::{
    identity::state_transition::asset_lock_proof::AssetLockProof, platform_value::BinaryData,
    state_transition::StateTransition,
};

use super::WalletError;

// Minimal wallet functionalities needed for the SDK to work
pub struct MinimalWallet {}
impl MinimalWallet {
    pub fn sign_state_transition(&self, st: &StateTransition) -> Result<BinaryData, WalletError> {
        todo!("sign_state_transition")
    }
    pub fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), WalletError> {
        todo!("lock_assets")
    }
}
