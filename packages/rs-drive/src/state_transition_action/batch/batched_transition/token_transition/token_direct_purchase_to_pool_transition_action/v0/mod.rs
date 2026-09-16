mod transformer;

use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::shielded::SerializedAction;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};
use crate::state_transition_action::shielded::ShieldedActionNote;

/// The resolved `TokenDirectPurchaseToPool` batched transition.
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseToPoolTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount of tokens purchased
    pub token_count: TokenAmount,
    /// The most credits the buyer agrees to pay
    pub total_agreed_price: Credits,
    /// The Orchard actions
    pub actions: Vec<SerializedAction>,
    /// The Orchard anchor
    pub anchor: [u8; 32],
    /// The Halo 2 proof
    pub proof: Vec<u8>,
    /// The RedPallas binding signature
    pub binding_signature: [u8; 64],
}

/// Accessors for the version 0 action.
pub trait TokenDirectPurchaseToPoolTransitionActionAccessorsV0 {
    /// The base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes the action and returns its base
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The amount of tokens purchased
    fn token_count(&self) -> TokenAmount;
    /// The most credits the buyer agrees to pay
    fn total_agreed_price(&self) -> Credits;

    /// The Orchard actions
    fn actions(&self) -> &[SerializedAction];

    /// The Orchard anchor
    fn anchor(&self) -> &[u8; 32];

    /// The Halo 2 proof
    fn proof(&self) -> &[u8];

    /// The RedPallas binding signature
    fn binding_signature(&self) -> &[u8; 64];

    /// The notes every action creates
    fn notes(&self) -> Vec<ShieldedActionNote> {
        self.actions()
            .iter()
            .map(ShieldedActionNote::from)
            .collect()
    }

    /// The nullifiers every action spends
    fn nullifiers(&self) -> Vec<[u8; 32]> {
        self.actions()
            .iter()
            .map(|action| action.nullifier)
            .collect()
    }

    /// The token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// The token id
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// The data contract id
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// A reference to the fetched data contract
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// The fetched data contract
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// The identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }
}

impl TokenDirectPurchaseToPoolTransitionActionAccessorsV0
    for TokenDirectPurchaseToPoolTransitionActionV0
{
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn token_count(&self) -> TokenAmount {
        self.token_count
    }
    fn total_agreed_price(&self) -> Credits {
        self.total_agreed_price
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
