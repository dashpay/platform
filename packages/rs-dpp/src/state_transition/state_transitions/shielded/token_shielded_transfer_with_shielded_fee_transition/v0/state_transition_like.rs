use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<TokenShieldedTransferWithShieldedFeeTransitionV0> for StateTransition {
    fn from(value: TokenShieldedTransferWithShieldedFeeTransitionV0) -> Self {
        let transition: TokenShieldedTransferWithShieldedFeeTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for TokenShieldedTransferWithShieldedFeeTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::TokenShieldedTransferWithShieldedFee
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
