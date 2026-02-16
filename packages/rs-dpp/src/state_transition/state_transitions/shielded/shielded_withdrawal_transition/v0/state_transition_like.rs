use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::{
    prelude::{Identifier, UserFeeIncrease},
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::ShieldedWithdrawal;
use crate::version::FeatureVersion;

impl From<ShieldedWithdrawalTransitionV0> for StateTransition {
    fn from(value: ShieldedWithdrawalTransitionV0) -> Self {
        let transition: ShieldedWithdrawalTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldedWithdrawalTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        ShieldedWithdrawal
    }

    /// Returns IDs of modified data (none for shielded transitions)
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// For ZK-only transitions, uniqueness comes from nullifiers in the actions.
    /// Each nullifier can only be used once, making them natural unique identifiers.
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        0
    }

    fn set_user_fee_increase(&mut self, _user_fee_increase: UserFeeIncrease) {
        // No-op: shielded withdrawal fees are cryptographically locked by the Orchard binding signature
    }
}
