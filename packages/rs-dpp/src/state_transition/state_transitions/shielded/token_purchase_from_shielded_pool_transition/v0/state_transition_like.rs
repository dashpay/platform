use crate::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<TokenPurchaseFromShieldedPoolTransitionV0> for StateTransition {
    fn from(value: TokenPurchaseFromShieldedPoolTransitionV0) -> Self {
        let transition: TokenPurchaseFromShieldedPoolTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for TokenPurchaseFromShieldedPoolTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::TokenPurchaseFromShieldedPool
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// Unique by every spent nullifier of both bundles: two transitions spending the same note
    /// in either pool can never both be valid.
    fn unique_identifiers(&self) -> Vec<String> {
        self.token_actions
            .iter()
            .chain(self.fee_actions.iter())
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }
}
