mod transformer;

use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::shielded::SerializedAction;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};
use crate::state_transition_action::shielded::ShieldedActionNote;

/// A transfer inside the token's shielded pool. Carries the Orchard spend bundle (value balance zero) so state validation can verify the proof and lower it to nullifier inserts and note appends.
#[derive(Debug, Clone)]
pub struct TokenShieldedTransferTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the token pool's note commitment tree the bundle was built against
    pub anchor: [u8; 32],
    /// Halo 2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
}

/// Accessors for `TokenShieldedTransferTransitionActionV0`
pub trait TokenShieldedTransferTransitionActionAccessorsV0 {
    /// Returns the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns the base owned token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The Orchard actions
    fn actions(&self) -> &[SerializedAction];

    /// The Orchard anchor the bundle was built against
    fn anchor(&self) -> &[u8; 32];

    /// The Halo 2 proof bytes
    fn proof(&self) -> &[u8];

    /// The RedPallas binding signature
    fn binding_signature(&self) -> &[u8; 64];

    /// The output notes to append to the pool's commitment tree, one per action
    fn notes(&self) -> Vec<ShieldedActionNote> {
        self.actions()
            .iter()
            .map(ShieldedActionNote::from)
            .collect()
    }

    /// The nullifiers revealed by the actions
    fn nullifiers(&self) -> Vec<[u8; 32]> {
        self.actions()
            .iter()
            .map(|action| action.nullifier)
            .collect()
    }

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID from the base action
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info from the base action
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the identity contract nonce from the base action
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }
}

impl TokenShieldedTransferTransitionActionAccessorsV0 for TokenShieldedTransferTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn actions(&self) -> &[SerializedAction] {
        &self.actions
    }

    fn anchor(&self) -> &[u8; 32] {
        &self.anchor
    }

    fn proof(&self) -> &[u8] {
        &self.proof
    }

    fn binding_signature(&self) -> &[u8; 64] {
        &self.binding_signature
    }
}
